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
    components::filters::Facets,
    entry::Entry,
};

/// Records per page of results.
pub const PAGE_SIZE: u32 = 50;

/// The order search results come in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    /// The API's own ranking.
    #[default]
    Relevance,
    Oldest,
    Newest,
    /// By title, A to Z. Titles that start with punctuation, such as quotes, come first.
    Title,
}

impl Sort {
    /// The next sort in the order `s` cycles through.
    pub fn next(self) -> Self {
        match self {
            Sort::Relevance => Sort::Oldest,
            Sort::Oldest => Sort::Newest,
            Sort::Newest => Sort::Title,
            Sort::Title => Sort::Relevance,
        }
    }

    /// How the status bar names it.
    pub fn label(self) -> &'static str {
        match self {
            Sort::Relevance => "relevance",
            Sort::Oldest => "oldest first",
            Sort::Newest => "newest first",
            Sort::Title => "title",
        }
    }
}

/// Some records carry placeholder dates far outside any real one, such as year -1824528578,
/// that would top a date sort. Sorting by date keeps to these years unless a range is set.
pub const DATED_YEARS: (i32, i32) = (-8000, 2100);

/// A search: the text typed, the filters set and the sort chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub filters: Filters,
    pub sort: Sort,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            text: String::new(),
            filters: default_filters(),
            sort: Sort::default(),
        }
    }
}

/// The filters a new session starts with: only artworks with an image, since one without has
/// no art (ADR 0013).
pub fn default_filters() -> Filters {
    Filters {
        has_image: true,
        ..Filters::default()
    }
}

impl Query {
    /// A search for `text`, with the default filters and sort.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Whether this asks for nothing in particular, so the start screen's Highlights fit.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// How many filters differ from the defaults.
    pub fn filter_count(&self) -> usize {
        let f = &self.filters;
        [
            !f.has_image,
            f.is_public_domain.is_some(),
            f.is_on_view.is_some(),
            f.department_id.is_some(),
            f.artwork_type_id.is_some(),
            f.artist_id.is_some(),
            f.years.is_some(),
        ]
        .into_iter()
        .filter(|&set| set)
        .count()
    }
}

/// What to list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Load {
    /// The start screen: these artworks, in this order.
    Highlights(Vec<u32>),
    /// Page `page` (from 1) of a search.
    Search { query: Query, page: u32 },
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
    /// The full record being fetched. Asking for another aborts it.
    record: Option<JoinHandle<()>>,
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
            record: None,
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

/// The departments and artwork types, to choose from in the filters.
async fn facets(catalog: &Catalog) -> eyre::Result<Facets> {
    let departments = catalog.departments().await?.value.data;
    let types = catalog.artwork_types().await?.value.data;
    Ok(Facets {
        departments: departments.into_iter().map(|d| (d.id, d.title)).collect(),
        types: types.into_iter().map(|t| (t.id, t.title)).collect(),
    })
}

impl Fetcher {
    /// Lists the departments and artwork types.
    fn fetch_facets(&self) {
        let catalog = self.catalog.clone();
        let tx = self.action_tx.clone();
        tokio::spawn(async move {
            let result = facets(&catalog).await.map_err(|e| e.to_string());
            let _ = tx.send(Action::FacetsLoaded(result));
        });
    }

    /// Fetches the full record of artwork `id`, replacing any record still on its way.
    fn fetch_record(&mut self, id: u32) {
        if let Some(previous) = self.record.take() {
            previous.abort();
        }
        let catalog = self.catalog.clone();
        let tx = self.action_tx.clone();
        self.record = Some(tokio::spawn(async move {
            let result = match catalog.artwork(id).await {
                Ok(fetched) => {
                    let mut timing = Timing::new(format!("record {id}"));
                    timing = if fetched.from_cache {
                        timing.stage("hit", Duration::ZERO)
                    } else {
                        timing
                            .stage("wait", fetched.wait)
                            .stage("fetch", fetched.fetch)
                    };
                    let _ = tx.send(Action::Timing(timing));
                    Ok(fetched.value.data)
                }
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(Action::RecordLoaded { id, result });
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
        Load::Search { query, page } => {
            let request = search_request(catalog.base_uri(), query, *page)?;
            let label = if *page == 1 {
                format!("search {}", query.text)
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

/// The request for page `page` of `query`.
fn search_request(base_uri: &str, query: &Query, page: u32) -> eyre::Result<search::Request> {
    let mut filters = query.filters.clone();
    let (sort, descending) = match query.sort {
        Sort::Relevance => (None, false),
        Sort::Oldest | Sort::Newest => {
            filters.years = filters.years.or(Some(DATED_YEARS));
            (Some("date_start"), query.sort == Sort::Newest)
        }
        // `title` is analyzed text, which the API cannot sort; its keyword form can.
        Sort::Title => (Some("title.keyword"), false),
    };
    let text = query.text.trim();
    let mut q = (!text.is_empty()).then(|| text.to_string());
    if sort.is_some() {
        // `q` matches loosely and ranks the loose matches low, which a sort would bring to the
        // top. Sorted, only titles and artists with every word match.
        filters.words = q.take();
    }
    Ok(search::Request::builder()
        .base_uri(base_uri)
        .q(q)
        .filters(filters)
        .sort(sort.map(String::from))
        .descending(descending)
        .fields(Some(fields(ArtworkSummary::FIELDS)))
        .limit(Some(PAGE_SIZE))
        .page(Some(page))
        .build()?)
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
    let config = page.config;
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
            config: config.clone(),
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
            Action::FetchRecord(id) => self.fetch_record(id),
            Action::FetchFacets => self.fetch_facets(),
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
        assert_eq!(
            listing.entries[0].config.iiif_url,
            "https://iiif.example/iiif/2"
        );
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
                query: Query::text("water lilies"),
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
                query: Query::text("x"),
                page: 1,
            },
        )
        .await;
        assert!(loaded.result.is_err());
    }

    fn url(query: &Query) -> String {
        search_request("https://api.example", query, 1)
            .unwrap()
            .to_string()
    }

    #[test]
    fn date_sorts_keep_to_real_years_and_title_sorts_by_keyword() {
        let mut query = Query::text("monet");
        query.sort = Sort::Newest;
        let newest = url(&query);
        assert!(newest.contains("sort[date_start][order]=desc"), "{newest}");
        assert!(newest.contains("[range][date_end][gte]=-8000"), "{newest}");
        assert!(newest.contains("[range][date_start][lte]=2100"), "{newest}");
        query.sort = Sort::Oldest;
        query.filters.years = Some((1850, 1900));
        let oldest = url(&query);
        assert!(oldest.contains("sort=date_start"), "{oldest}");
        assert!(
            oldest.contains("[range][date_end][gte]=1850"),
            "a set range wins"
        );
        query.sort = Sort::Title;
        let title = url(&query);
        assert!(title.contains("sort=title.keyword"));
        assert!(!title.contains("q=monet"), "sorted, the text must match");
        assert!(title.contains("[multi_match][query]=monet"), "{title}");
        query.sort = Sort::Relevance;
        assert!(url(&query).contains("q=monet"));
    }

    #[test]
    fn a_search_without_text_sends_no_q_and_counts_its_filters() {
        let mut query = Query::default();
        assert!(query.is_empty());
        query.filters.is_on_view = Some(true);
        query.filters.has_image = false;
        assert!(!url(&query).contains("q="));
        assert_eq!(query.filter_count(), 2);
        assert!(!query.is_empty());
    }

    #[tokio::test]
    async fn a_full_record_comes_back_for_its_artwork() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/artworks/27992"))
            .and(query_param_contains("fields", "description"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"id": 27992, "title": "A Sunday", "dimensions": "207.5 cm"},
            })))
            .expect(1)
            .mount(&server)
            .await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut fetcher = Fetcher::new(tx, catalog(&server));
        fetcher.update(Action::FetchRecord(27992)).unwrap();
        loop {
            match rx.recv().await.expect("a result") {
                Action::RecordLoaded { id, result } => {
                    assert_eq!(id, 27992);
                    assert_eq!(result.unwrap().dimensions.as_deref(), Some("207.5 cm"));
                    break;
                }
                Action::Timing(timing) => assert_eq!(timing.label, "record 27992"),
                other => panic!("unexpected {other:?}"),
            }
        }
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
                        query: Query::text(q),
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
