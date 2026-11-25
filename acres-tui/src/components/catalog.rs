//! Fetches lists for the explorer: the Highlights start screen and pages of a search.
//!
//! Every list request carries the generation of the search it belongs to. A request for a newer
//! generation aborts the one in flight, which also gives up its place in the catalog's queue,
//! and the explorer drops any result older than its latest search (ADR 0013).

use std::time::{Duration, Instant};

use acres::{
    Catalog, Page, Resource,
    artworks::{
        ArtworkSummary,
        request::{
            artworks,
            search::{self, Filters},
        },
    },
};
use color_eyre::eyre;
use ratatui::{Frame, layout::Rect};
use serde::Deserialize;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    action::{Action, Timing},
    components::Component,
    entry::Entry,
};

/// Records per page of results.
pub const PAGE_SIZE: u32 = 50;

/// What to list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Load {
    /// The start screen: these artworks, in this order.
    Highlights(Vec<u32>),
    /// Page `page` (from 1) of a full-text search.
    Search { q: String, page: u32 },
}

/// A list that came back, or why it didn't.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub generation: u64,
    pub load: Load,
    pub result: Result<Listing, String>,
}

/// Records from one request, and how many there are in all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    pub entries: Vec<Entry>,
    pub total: u64,
    /// Whether the Highlights were unavailable and this is the fallback listing instead.
    pub fallback: bool,
}

/// A Highlights record, with what is needed to drop one that went stale.
#[derive(Deserialize)]
struct Highlight {
    #[serde(flatten)]
    summary: ArtworkSummary,
    #[serde(default)]
    is_public_domain: bool,
}

pub struct Fetcher {
    catalog: Catalog,
    action_tx: UnboundedSender<Action>,
    /// The generation of the request in flight, if any.
    generation: u64,
    pending: Option<JoinHandle<()>>,
    /// What the status bar was last told about the rate limit.
    rate_limited: bool,
}

impl Fetcher {
    pub fn new(action_tx: UnboundedSender<Action>, catalog: Catalog) -> Self {
        Self {
            catalog,
            action_tx,
            generation: 0,
            pending: None,
            rate_limited: false,
        }
    }

    fn start(&mut self, generation: u64, load: Load) {
        // A newer search makes the one in flight pointless. A next page of the same search
        // waits its turn instead.
        if generation != self.generation
            && let Some(previous) = self.pending.take()
        {
            previous.abort();
        }
        self.generation = generation;
        let catalog = self.catalog.clone();
        let tx = self.action_tx.clone();
        self.pending = Some(tokio::spawn(async move {
            let result = fetch(&catalog, &load, &tx).await.map_err(|e| e.to_string());
            let _ = tx.send(Action::Loaded(Loaded {
                generation,
                load,
                result,
            }));
        }));
    }
}

/// Runs `load`, reporting each request's stages to the timing HUD.
async fn fetch(
    catalog: &Catalog,
    load: &Load,
    tx: &UnboundedSender<Action>,
) -> eyre::Result<Listing> {
    match load {
        Load::Highlights(ids) => {
            let listing = highlights(catalog, ids, tx).await?;
            if !listing.entries.is_empty() {
                return Ok(listing);
            }
            tracing::warn!("no highlights left; listing recent public-domain works instead");
            let request = search::Request::builder()
                .base_uri(catalog.base_uri())
                .filters(Filters {
                    has_image: true,
                    is_public_domain: Some(true),
                    ..Filters::default()
                })
                .sort(Some("updated_at".into()))
                .descending(true)
                .fields(Some(fields(ArtworkSummary::FIELDS)))
                .limit(Some(PAGE_SIZE))
                .build()?;
            let page: Page<ArtworkSummary> =
                timed(catalog, &request, "highlights fallback", tx).await?;
            Ok(Listing {
                total: page.pagination.total,
                entries: Entry::from_page(page),
                fallback: true,
            })
        }
        Load::Search { q, page } => {
            let request = search::Request::builder()
                .base_uri(catalog.base_uri())
                .q(Some(q.clone()))
                // A record without an image has no art (ADR 0013).
                .filters(Filters {
                    has_image: true,
                    ..Filters::default()
                })
                .fields(Some(fields(ArtworkSummary::FIELDS)))
                .limit(Some(PAGE_SIZE))
                .page(Some(*page))
                .build()?;
            let label = if *page == 1 {
                format!("search {q}")
            } else {
                format!("page {page}")
            };
            let page: Page<ArtworkSummary> = timed(catalog, &request, &label, tx).await?;
            Ok(Listing {
                total: page.pagination.total,
                entries: Entry::from_page(page),
                fallback: false,
            })
        }
    }
}

/// The Highlights in `ids` order, without any that are gone, no longer public domain, or have
/// no image.
async fn highlights(
    catalog: &Catalog,
    ids: &[u32],
    tx: &UnboundedSender<Action>,
) -> eyre::Result<Listing> {
    if ids.is_empty() {
        return Ok(Listing::default());
    }
    let mut wanted: Vec<&str> = ArtworkSummary::FIELDS.to_vec();
    wanted.push("is_public_domain");
    let request = artworks::Request::builder()
        .base_uri(catalog.base_uri())
        .ids(Some(ids.to_vec()))
        .fields(Some(fields(&wanted)))
        .limit(Some(u32::try_from(ids.len()).unwrap_or(u32::MAX)))
        .build()?;
    let page: Page<Highlight> = timed(catalog, &request, "highlights", tx).await?;
    let iiif_url = page.config.iiif_url;
    let mut found: Vec<Highlight> = page
        .data
        .into_iter()
        .filter(|h| h.is_public_domain && h.summary.image_id.is_some())
        .collect();
    // The API answers in its own order; list them in ours.
    found.sort_by_key(|h| ids.iter().position(|&id| id == h.summary.id));
    let entries: Vec<Entry> = found
        .into_iter()
        .map(|h| Entry {
            summary: h.summary,
            iiif_url: iiif_url.clone(),
        })
        .collect();
    Ok(Listing {
        total: entries.len() as u64,
        entries,
        fallback: false,
    })
}

/// Fetches and parses `request`, sending its wait, fetch and parse times (or a cache hit) to
/// the HUD as `label`.
async fn timed<T: serde::de::DeserializeOwned>(
    catalog: &Catalog,
    request: impl std::fmt::Display,
    label: &str,
    tx: &UnboundedSender<Action>,
) -> eyre::Result<T> {
    let endpoint = request.to_string();
    let fetched = catalog.raw(&endpoint, Resource::Page).await?;
    let start = Instant::now();
    let value = serde_json::from_slice(&fetched.value)
        .map_err(|e| eyre::eyre!("could not read {endpoint}: {e}"))?;
    let parse = start.elapsed();
    let mut timing = Timing::new(label);
    if fetched.from_cache {
        timing = timing.stage("hit", Duration::ZERO);
    } else {
        timing = timing
            .stage("wait", fetched.wait)
            .stage("fetch", fetched.fetch);
    }
    let _ = tx.send(Action::Timing(timing.stage("parse", parse)));
    Ok(value)
}

fn fields(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

impl Component for Fetcher {
    fn update(&mut self, action: Action) -> eyre::Result<Option<Action>> {
        match action {
            Action::Load { generation, load } => self.start(generation, load),
            Action::Tick => {
                let limited = self.catalog.is_rate_limited();
                if limited != self.rate_limited {
                    self.rate_limited = limited;
                    return Ok(Some(Action::RateLimited(limited)));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, _frame: &mut Frame, _area: Rect) -> eyre::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param, query_param_contains},
    };

    use super::*;

    fn catalog(server: &MockServer) -> Catalog {
        Catalog::builder()
            .base_uri(server.uri())
            .no_cache()
            .no_shared_rate_limit()
            .offline(false)
            .build()
            .unwrap()
    }

    fn page(rows: serde_json::Value, total: u64) -> serde_json::Value {
        serde_json::json!({
            "pagination": {"total": total, "limit": 50, "offset": 0, "total_pages": 1, "current_page": 1},
            "data": rows,
            "config": {"iiif_url": "https://iiif.example/iiif/2", "website_url": "https://example"},
        })
    }

    /// Sends `load` and returns what came back, skipping timings.
    async fn run(catalog: Catalog, load: Load) -> Loaded {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut fetcher = Fetcher::new(tx, catalog);
        fetcher
            .update(Action::Load {
                generation: 7,
                load,
            })
            .unwrap();
        loop {
            match rx.recv().await.expect("a result") {
                Action::Loaded(loaded) => return loaded,
                Action::Timing(_) => {}
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    fn titles(loaded: &Loaded) -> Vec<&str> {
        let listing = loaded.result.as_ref().expect("loaded");
        listing.entries.iter().map(Entry::title).collect()
    }

    #[tokio::test]
    async fn highlights_keep_our_order_and_drop_stale_ones() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks"))
            .and(query_param("ids", "3,1,2,4"))
            // A listing by ids has no pagination, as the live API answers it.
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {"id": 1, "title": "One", "image_id": "a", "is_public_domain": true},
                    {"id": 2, "title": "Two", "image_id": "b", "is_public_domain": false},
                    {"id": 3, "title": "Three", "image_id": "c", "is_public_domain": true},
                    {"id": 4, "title": "Four", "image_id": null, "is_public_domain": true},
                ],
                "config": {"iiif_url": "https://iiif.example/iiif/2", "website_url": "https://example"},
            })))
            .mount(&server)
            .await;
        let loaded = run(catalog(&server), Load::Highlights(vec![3, 1, 2, 4])).await;
        assert_eq!(loaded.generation, 7);
        assert_eq!(titles(&loaded), ["Three", "One"]);
        let listing = loaded.result.unwrap();
        assert!(!listing.fallback);
        assert_eq!(listing.entries[0].iiif_url, "https://iiif.example/iiif/2");
    }

    #[tokio::test]
    async fn no_highlights_left_falls_back_to_recent_public_domain_works() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(serde_json::json!([]), 0)))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/artworks/search"))
            .and(query_param("sort[updated_at][order]", "desc"))
            .and(query_param(
                "query[bool][filter][1][term][is_public_domain]",
                "true",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([{"id": 9, "title": "Recent", "image_id": "z"}]),
                62_000,
            )))
            .mount(&server)
            .await;
        let loaded = run(catalog(&server), Load::Highlights(vec![1])).await;
        assert_eq!(titles(&loaded), ["Recent"]);
        let listing = loaded.result.unwrap();
        assert!(listing.fallback);
        assert_eq!(listing.total, 62_000);
    }

    #[tokio::test]
    async fn search_asks_for_a_page_of_artworks_with_images() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks/search"))
            .and(query_param("q", "water lilies"))
            .and(query_param("page", "2"))
            .and(query_param("limit", "50"))
            .and(query_param(
                "query[bool][filter][0][exists][field]",
                "image_id",
            ))
            .and(query_param_contains("fields", "artist_display"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(
                serde_json::json!([{"id": 16568, "title": "Water Lilies", "image_id": "x"}]),
                120,
            )))
            .mount(&server)
            .await;
        let loaded = run(
            catalog(&server),
            Load::Search {
                q: "water lilies".into(),
                page: 2,
            },
        )
        .await;
        assert_eq!(titles(&loaded), ["Water Lilies"]);
        assert_eq!(loaded.result.unwrap().total, 120);
    }

    #[tokio::test]
    async fn a_failed_search_comes_back_as_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks/search"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let catalog = catalog(&server);
        let loaded = run(
            catalog,
            Load::Search {
                q: "x".into(),
                page: 1,
            },
        )
        .await;
        assert!(loaded.result.is_err());
    }

    #[tokio::test]
    async fn a_newer_generation_aborts_the_search_in_flight() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks/search"))
            .and(query_param("q", "slow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(page(serde_json::json!([]), 0))
                    .set_delay(Duration::from_secs(5)),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/artworks/search"))
            .and(query_param("q", "fast"))
            .respond_with(ResponseTemplate::new(200).set_body_json(page(serde_json::json!([]), 0)))
            .mount(&server)
            .await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut fetcher = Fetcher::new(tx, catalog(&server));
        for (generation, q) in [(1, "slow"), (2, "fast")] {
            fetcher
                .update(Action::Load {
                    generation,
                    load: Load::Search {
                        q: q.into(),
                        page: 1,
                    },
                })
                .unwrap();
        }
        let first = loop {
            match rx.recv().await.unwrap() {
                Action::Loaded(loaded) => break loaded,
                _ => continue,
            }
        };
        assert_eq!(first.generation, 2);
        // The slow one was aborted, so nothing else ever arrives.
        let more = tokio::time::timeout(Duration::from_millis(300), async {
            loop {
                if let Action::Loaded(loaded) = rx.recv().await? {
                    return Some(loaded);
                }
            }
        })
        .await;
        assert!(matches!(more, Err(_) | Ok(None)));
    }
}
