use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use acres::Catalog;
use color_eyre::eyre::{self};
use iiif::ImageRequest;
use ratatui::{Frame, prelude::Rect};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    action::{Action, Timing},
    components::{Component, home::Decoded},
    entry::Entry,
};

/// How long the selection must rest on an artwork before its cached image is read and decoded,
/// so holding `j` through a list does not decode every row (ADR 0013, ADR 0018).
pub const PREVIEW_DELAY: Duration = Duration::from_millis(150);

/// Full images kept decoded, so going back to one shows it at once (ADR 0013).
pub const RECENT: usize = 16;

/// The last [`RECENT`] full images decoded, most recent last.
#[derive(Debug, Default)]
struct Recent(VecDeque<(u32, Decoded)>);

impl Recent {
    fn get(&mut self, id: u32) -> Option<Decoded> {
        let at = self.0.iter().position(|(seen, _)| *seen == id)?;
        let entry = self.0.remove(at)?;
        let image = entry.1.clone();
        self.0.push_back(entry);
        Some(image)
    }

    fn contains(&self, id: u32) -> bool {
        self.0.iter().any(|(seen, _)| *seen == id)
    }

    fn put(&mut self, id: u32, image: Decoded) {
        self.0.retain(|(seen, _)| *seen != id);
        if self.0.len() == RECENT {
            self.0.pop_front();
        }
        self.0.push_back((id, image));
    }
}

/// Shows the selected artwork's image (ADR 0018). Browsing shows an image only if it was
/// decoded recently or is in the catalog's disk cache, so it never costs a request; otherwise the
/// art stays empty until the full image is asked for. Images are decoded on a blocking thread,
/// off the app loop. It also answers which listed artworks have their image cached.
pub struct Iiif {
    request: Option<ImageRequest>,
    artwork_id: u32,
    /// Whether the full image of the shown artwork was asked for, so asking again does nothing.
    full: bool,
    action_tx: UnboundedSender<Action>,
    catalog: Catalog,
    recent: Arc<Mutex<Recent>>,
    /// The cached image waiting out the preview delay, or the full image being fetched. A newer
    /// request aborts it, which also gives up its place in the catalog's queue.
    pending: Option<JoinHandle<()>>,
}

/// Decodes `bytes` on a blocking thread, adding the decode time to `timing`.
async fn decode(bytes: Vec<u8>, timing: Timing) -> eyre::Result<(Decoded, Timing)> {
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let image = Decoded::decode(&bytes)?;
        Ok((image, timing.stage("decode", start.elapsed())))
    })
    .await?
}

impl Iiif {
    pub fn new(action_tx: UnboundedSender<Action>, catalog: Catalog) -> Self {
        Self {
            request: None,
            artwork_id: 0,
            full: false,
            action_tx,
            catalog,
            recent: Arc::default(),
            pending: None,
        }
    }

    fn abort_pending(&mut self) {
        if let Some(previous) = self.pending.take() {
            previous.abort();
        }
    }

    fn recent(&self) -> std::sync::MutexGuard<'_, Recent> {
        self.recent
            .lock()
            .expect("no panics while holding the lock")
    }

    /// Shows the cached image, if there is one, once the preview delay passes.
    fn show_cached(&mut self) {
        let Some(request) = self.request.clone() else {
            return;
        };
        let tx = self.action_tx.clone();
        let catalog = self.catalog.clone();
        let recent = self.recent.clone();
        let id = self.artwork_id;
        self.pending = Some(tokio::spawn(async move {
            tokio::time::sleep(PREVIEW_DELAY).await;
            let read = Instant::now();
            let bytes = match catalog.cached_image(&request) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return,
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("cached image {id}: {e}")));
                    return;
                }
            };
            let timing = Timing::new(format!("cached {id}")).stage("read", read.elapsed());
            match decode(bytes.to_vec(), timing).await {
                Ok((image, timing)) => {
                    recent
                        .lock()
                        .expect("no panics while holding the lock")
                        .put(id, image.clone());
                    let _ = tx.send(Action::UpdateImage { id, image, timing });
                }
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("cached image {id}: {e}")));
                }
            }
        }));
    }

    /// Fetches and decodes the full image now, replacing any cached read still waiting.
    fn fetch_full(&mut self) {
        let Some(image_request) = self.request.clone() else {
            return;
        };
        tracing::debug!(image_request = %image_request);
        let tx = self.action_tx.clone();
        let catalog = self.catalog.clone();
        let recent = self.recent.clone();
        let id = self.artwork_id;
        self.abort_pending();
        self.pending = Some(tokio::spawn(async move {
            let fetched = match catalog.image(&image_request).await {
                Ok(fetched) => fetched,
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("image {id}: {e}")));
                    return;
                }
            };
            let mut timing = Timing::new(format!("image {id}"));
            timing = if fetched.from_cache {
                timing.stage("hit", Duration::ZERO)
            } else {
                timing
                    .stage("wait", fetched.wait)
                    .stage("fetch", fetched.fetch)
            };
            // Fetched means cached now, so its row can say so.
            let _ = tx.send(Action::Cached(vec![id]));
            match decode(fetched.value.to_vec(), timing).await {
                Ok((image, timing)) => {
                    recent
                        .lock()
                        .expect("no panics while holding the lock")
                        .put(id, image.clone());
                    let _ = tx.send(Action::UpdateImage { id, image, timing });
                }
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("image {id}: {e}")));
                }
            }
        }));
    }

    /// Checks `entries` against the disk cache on a blocking thread, and sends back the ids of
    /// those whose image is cached.
    fn check_cached(&self, entries: Vec<Entry>) {
        let tx = self.action_tx.clone();
        let catalog = self.catalog.clone();
        tokio::task::spawn_blocking(move || {
            let cached = cached_ids(&catalog, &entries);
            if !cached.is_empty() {
                let _ = tx.send(Action::Cached(cached));
            }
        });
    }
}

/// The ids of `entries` whose full image is in `catalog`'s cache: one `stat` each.
pub fn cached_ids(catalog: &Catalog, entries: &[Entry]) -> Vec<u32> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .image_request()
                .is_some_and(|request| catalog.has_cached_image(&request))
        })
        .map(Entry::id)
        .collect()
}

impl Component for Iiif {
    fn update(&mut self, action: Action) -> eyre::Result<Option<Action>> {
        match action {
            Action::ShowArtwork(entry) => {
                self.abort_pending();
                self.artwork_id = entry.id();
                self.request = entry.image_request();
                let seen = self.recent().get(entry.id());
                self.full = seen.is_some();
                match seen {
                    // Decoded recently: show it again at once.
                    Some(image) => {
                        let _ = self.action_tx.send(Action::UpdateImage {
                            id: entry.id(),
                            image,
                            timing: Timing::new(format!("recent {}", entry.id())),
                        });
                    }
                    // Clear at once, so the last artwork's art never sits beside this one's
                    // label, then show the cached image if there is one.
                    None => {
                        let _ = self.action_tx.send(Action::ClearImage);
                        self.show_cached();
                    }
                }
            }
            Action::ShowFullImage if !self.full && self.request.is_some() => {
                self.full = true;
                // A cached image already read and shown needs no second read.
                if !self.recent().contains(self.artwork_id) {
                    self.fetch_full();
                }
            }
            Action::CheckCached(entries) => self.check_cached(entries),
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
    use std::{path::PathBuf, time::Duration};

    use tokio::sync::mpsc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    /// La Grande Jatte, from the demo fixtures, served as every full image.
    const IMAGE: &[u8] = include_bytes!(
        "../../tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg"
    );

    /// An artwork with an image.
    fn entry(server: &MockServer, id: u32) -> Entry {
        Entry {
            summary: acres::artworks::ArtworkSummary {
                id,
                image_id: Some(format!("image-{id}")),
                ..Default::default()
            },
            config: acres::ApiConfig {
                iiif_url: format!("{}/iiif", server.uri()),
                ..Default::default()
            },
        }
    }

    fn full(id: u32, delay: Duration) -> Mock {
        Mock::given(method("GET"))
            .and(path(format!("/iiif/image-{id}/full/843,/0/default.jpg")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(IMAGE)
                    .set_delay(delay),
            )
    }

    fn show(iiif: &mut Iiif, entry: Entry) {
        assert_eq!(iiif.update(Action::ShowArtwork(entry)).unwrap(), None);
    }

    /// What reaches the art `within` a time: the id of each image, and 0 for a cleared pane.
    async fn arrivals(rx: &mut mpsc::UnboundedReceiver<Action>, within: Duration) -> Vec<u32> {
        let mut ids = Vec::new();
        let _ = tokio::time::timeout(within, async {
            while let Some(action) = rx.recv().await {
                match action {
                    Action::UpdateImage { id, .. } => ids.push(id),
                    Action::ClearImage => ids.push(0),
                    _ => {}
                }
            }
        })
        .await;
        ids
    }

    /// A fresh, empty cache directory for one test.
    fn cache_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("acres-tui-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn catalog(server: &MockServer, cache: Option<PathBuf>) -> Catalog {
        let builder = Catalog::builder()
            .base_uri(server.uri())
            .no_shared_rate_limit()
            .offline(false);
        match cache {
            Some(dir) => builder.cache_dir(dir),
            None => builder.no_cache(),
        }
        .build()
        .unwrap()
    }

    fn iiif_with(catalog: Catalog) -> (Iiif, mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Iiif::new(tx, catalog), rx)
    }

    fn iiif(server: &MockServer) -> (Iiif, mpsc::UnboundedReceiver<Action>) {
        iiif_with(catalog(server, None))
    }

    /// Fails the test if anything is fetched.
    async fn no_requests(server: &MockServer) {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(server)
            .await;
    }

    /// Fetches artwork `id`'s full image into the cache at `dir`.
    async fn cache_image(server: &MockServer, dir: &std::path::Path, id: u32) {
        let request = entry(server, id).image_request().unwrap();
        catalog(server, Some(dir.into()))
            .image(&request)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn browsing_uncached_artworks_clears_the_art_at_once_and_fetches_nothing() {
        let server = MockServer::start().await;
        no_requests(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        show(&mut iiif, entry(&server, 2));
        // Both cleared at once, and nothing follows.
        assert_eq!(
            rx.try_recv().ok().map(|a| a.to_string()),
            Some("ClearImage".into())
        );
        assert_eq!(
            rx.try_recv().ok().map(|a| a.to_string()),
            Some("ClearImage".into())
        );
        assert!(
            arrivals(&mut rx, Duration::from_millis(400))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn browsing_shows_only_the_last_cached_image_and_fetches_nothing() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).expect(1).mount(&server).await;
        full(2, Duration::ZERO).expect(0).mount(&server).await;
        let dir = cache_dir("browse");
        cache_image(&server, &dir, 1).await;
        let (mut iiif, mut rx) = iiif_with(catalog(&server, Some(dir.clone())));
        show(&mut iiif, entry(&server, 1));
        show(&mut iiif, entry(&server, 2));
        show(&mut iiif, entry(&server, 1));
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(500)).await,
            [0, 0, 0, 1],
            "cleared three times, then the cached image after the delay"
        );
        // Shown already, so asking for it reads and fetches nothing more.
        iiif.update(Action::ShowFullImage).unwrap();
        assert!(
            arrivals(&mut rx, Duration::from_millis(300))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn the_full_image_comes_only_when_asked_for_and_only_once() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).expect(1).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        assert_eq!(arrivals(&mut rx, Duration::from_millis(400)).await, [0]);
        iiif.update(Action::ShowFullImage).unwrap();
        iiif.update(Action::ShowFullImage).unwrap();
        assert_eq!(arrivals(&mut rx, Duration::from_millis(600)).await, [1]);
    }

    #[tokio::test]
    async fn a_fetched_image_marks_its_row_cached() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        iiif.update(Action::ShowFullImage).unwrap();
        let cached = tokio::time::timeout(Duration::from_millis(600), async {
            loop {
                if let Some(Action::Cached(ids)) = rx.recv().await {
                    return ids;
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(cached, [1]);
    }

    #[tokio::test]
    async fn moving_on_aborts_a_full_image_still_on_the_wire() {
        let server = MockServer::start().await;
        full(1, Duration::from_millis(800)).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        iiif.update(Action::ShowFullImage).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        show(&mut iiif, entry(&server, 2));
        assert_eq!(arrivals(&mut rx, Duration::from_millis(1200)).await, [0, 0]);
    }

    #[tokio::test]
    async fn a_full_image_seen_recently_shows_again_without_a_fetch() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).expect(1).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        iiif.update(Action::ShowFullImage).unwrap();
        assert_eq!(arrivals(&mut rx, Duration::from_millis(600)).await, [0, 1]);
        show(&mut iiif, entry(&server, 2));
        show(&mut iiif, entry(&server, 1));
        // At once, with no clear before it.
        assert_eq!(arrivals(&mut rx, Duration::from_millis(400)).await, [0, 1]);
        iiif.update(Action::ShowFullImage).unwrap();
        assert!(
            arrivals(&mut rx, Duration::from_millis(300))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn check_cached_answers_with_the_cached_ids() {
        let server = MockServer::start().await;
        full(2, Duration::ZERO).mount(&server).await;
        let dir = cache_dir("check");
        cache_image(&server, &dir, 2).await;
        let catalog = catalog(&server, Some(dir));
        let mut bare = entry(&server, 3);
        bare.summary.image_id = None;
        let entries = vec![entry(&server, 1), entry(&server, 2), bare];
        assert_eq!(cached_ids(&catalog, &entries), [2]);
        let (mut iiif, mut rx) = iiif_with(catalog);
        iiif.update(Action::CheckCached(entries)).unwrap();
        let answer = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .unwrap();
        assert!(matches!(answer, Some(Action::Cached(ids)) if ids == [2]));
    }

    #[test]
    fn recent_keeps_the_last_images_seen() {
        let image = Decoded::decode(IMAGE).unwrap();
        let mut recent = Recent::default();
        for id in 0..RECENT as u32 + 1 {
            recent.put(id, image.clone());
        }
        assert!(recent.get(0).is_none(), "the oldest went first");
        assert!(recent.get(1).is_some());
        recent.put(99, image.clone());
        assert!(recent.get(1).is_some(), "a get counts as seeing it again");
        assert!(recent.get(2).is_none());
        assert!(recent.contains(99));
    }
}
