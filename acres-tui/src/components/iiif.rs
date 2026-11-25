use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use acres::Catalog;
use color_eyre::eyre::{self};
use iiif::{BaseUri, Format, ImageRequest, Quality, Region, Rotation, Size};
use ratatui::{Frame, prelude::Rect};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{
    action::{Action, Timing},
    components::{Component, home::Decoded},
};

/// How long the selection must rest on an artwork before its placeholder replaces the art, so
/// holding `j` through a list does not redraw every row (ADR 0013).
pub const PREVIEW_DELAY: Duration = Duration::from_millis(150);

/// Width of the full image: the AIC's recommended size, which it serves for every artwork.
pub const FULL_WIDTH: u32 = 843;

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

    fn put(&mut self, id: u32, image: Decoded) {
        self.0.retain(|(seen, _)| *seen != id);
        if self.0.len() == RECENT {
            self.0.pop_front();
        }
        self.0.push_back((id, image));
    }
}

/// Shows the selected artwork's image. Browsing shows the `lqip` placeholder that came with the
/// list, which costs no request, and the full image is fetched only when asked for. Images are
/// decoded on a blocking thread, off the app loop.
pub struct Iiif {
    base_uri: Option<BaseUri>,
    artwork_id: u32,
    /// Whether the full image of the shown artwork was asked for, so asking again does nothing.
    full: bool,
    action_tx: UnboundedSender<Action>,
    catalog: Catalog,
    recent: Arc<Mutex<Recent>>,
    /// The placeholder waiting out the preview delay, or the full image being fetched. A newer
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
            base_uri: None,
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

    /// Shows `placeholder` once the preview delay passes, or clears the art if there is none.
    fn preview(&mut self, placeholder: Option<Vec<u8>>) {
        let tx = self.action_tx.clone();
        let id = self.artwork_id;
        self.pending = Some(tokio::spawn(async move {
            tokio::time::sleep(PREVIEW_DELAY).await;
            let Some(bytes) = placeholder else {
                let _ = tx.send(Action::ClearImage);
                return;
            };
            match decode(bytes, Timing::new(format!("placeholder {id}"))).await {
                Ok((image, timing)) => {
                    let _ = tx.send(Action::UpdateImage {
                        id,
                        image,
                        full: false,
                        timing,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("placeholder {id}: {e}")));
                    let _ = tx.send(Action::ClearImage);
                }
            }
        }));
    }

    /// Fetches and decodes the full image now, replacing any placeholder still waiting.
    fn fetch_full(&mut self) {
        let Some(uri) = &self.base_uri else { return };
        let image_request = ImageRequest::builder()
            .base_uri(uri.clone())
            .region(Region::Full)
            .size(Size::Width(FULL_WIDTH))
            .rotation(Rotation::Degrees(
                0.0.try_into().expect("0 degrees is a valid setting"),
            ))
            .quality(Quality::Default)
            .format(Format::Jpg)
            .build();
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
            match decode(fetched.value.to_vec(), timing).await {
                Ok((image, timing)) => {
                    recent
                        .lock()
                        .expect("no panics while holding the lock")
                        .put(id, image.clone());
                    let _ = tx.send(Action::UpdateImage {
                        id,
                        image,
                        full: true,
                        timing,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Action::Log(format!("image {id}: {e}")));
                }
            }
        }));
    }
}

impl Component for Iiif {
    fn update(&mut self, action: Action) -> eyre::Result<Option<Action>> {
        match action {
            Action::ShowArtwork(entry) => {
                self.abort_pending();
                self.artwork_id = entry.id();
                self.base_uri = entry.base_uri();
                let seen = self
                    .recent
                    .lock()
                    .expect("no panics while holding the lock")
                    .get(entry.id());
                self.full = seen.is_some();
                match seen {
                    // Seen in full recently: show it again at once, with no placeholder.
                    Some(image) => {
                        let _ = self.action_tx.send(Action::UpdateImage {
                            id: entry.id(),
                            image,
                            full: true,
                            timing: Timing::new(format!("recent {}", entry.id())),
                        });
                    }
                    None => self.preview(entry.placeholder()),
                }
            }
            Action::ShowFullImage if !self.full && self.base_uri.is_some() => {
                self.full = true;
                self.fetch_full();
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
    use std::time::Duration;

    use tokio::sync::mpsc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;
    use crate::entry::Entry;

    /// La Grande Jatte, from the demo fixtures, served as every full image.
    const IMAGE: &[u8] = include_bytes!(
        "../../tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg"
    );

    /// A real 8x5 `lqip` GIF.
    const LQIP: &str = "R0lGODlhCAAFAPUAAE1WUFVTU1NWUFZYVllbVlpaW1tcWF1cXVxiWl1jWl9mX2NmVWVnXF9eZlxdaGZkYmpmZWRtYGZuY2htZGpsanBsZXVwZXB5Znp+ZXJwaYSAaYiBaYqIbIeAdI2DcYqFcI+FcoiCd5iRcouJgIyKhoyNi4mMjo2RkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAAAAAAALAAAAAAIAAUAAAYlwBLmMmEkCAHTaRTqfB4QEmgj8mgaGY6lsohIHAXKwQBQIAaCIAA7";

    /// An artwork with an image and a placeholder.
    fn entry(server: &MockServer, id: u32) -> Entry {
        Entry {
            summary: acres::artworks::ArtworkSummary {
                id,
                image_id: Some(format!("image-{id}")),
                thumbnail: Some(acres::artworks::Thumbnail {
                    lqip: Some(format!("data:image/gif;base64,{LQIP}")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            iiif_url: format!("{}/iiif", server.uri()),
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

    /// The images that reach the art `within` a time, as `(id, full)`, with `(0, false)` for a
    /// cleared art pane.
    async fn arrivals(
        rx: &mut mpsc::UnboundedReceiver<Action>,
        within: Duration,
    ) -> Vec<(u32, bool)> {
        let mut ids = Vec::new();
        let _ = tokio::time::timeout(within, async {
            while let Some(action) = rx.recv().await {
                match action {
                    Action::UpdateImage { id, full, .. } => ids.push((id, full)),
                    Action::ClearImage => ids.push((0, false)),
                    _ => {}
                }
            }
        })
        .await;
        ids
    }

    fn iiif(server: &MockServer) -> (Iiif, mpsc::UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let catalog = Catalog::builder()
            .base_uri(server.uri())
            .no_cache()
            .no_shared_rate_limit()
            .offline(false)
            .build()
            .unwrap();
        (Iiif::new(tx, catalog), rx)
    }

    /// Fails the test if anything is fetched.
    async fn no_requests(server: &MockServer) {
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn browsing_shows_only_the_last_placeholder_and_fetches_nothing() {
        let server = MockServer::start().await;
        no_requests(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        show(&mut iiif, entry(&server, 2));
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(400)).await,
            [(2, false)]
        );
    }

    #[tokio::test]
    async fn an_artwork_without_a_placeholder_clears_the_art() {
        let server = MockServer::start().await;
        no_requests(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        let mut bare = entry(&server, 1);
        bare.summary.thumbnail = None;
        show(&mut iiif, bare);
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(400)).await,
            [(0, false)]
        );
    }

    #[tokio::test]
    async fn the_full_image_comes_only_when_asked_for_and_only_once() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).expect(1).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(400)).await,
            [(1, false)]
        );
        iiif.update(Action::ShowFullImage).unwrap();
        iiif.update(Action::ShowFullImage).unwrap();
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(600)).await,
            [(1, true)]
        );
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
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(1200)).await,
            [(2, false)]
        );
    }

    #[tokio::test]
    async fn a_full_image_seen_recently_shows_again_without_a_fetch() {
        let server = MockServer::start().await;
        full(1, Duration::ZERO).expect(1).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        iiif.update(Action::ShowFullImage).unwrap();
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(600)).await,
            [(1, true)]
        );
        show(&mut iiif, entry(&server, 2));
        show(&mut iiif, entry(&server, 1));
        // At once, and no placeholder follows it.
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(400)).await,
            [(1, true)]
        );
        iiif.update(Action::ShowFullImage).unwrap();
        assert!(
            arrivals(&mut rx, Duration::from_millis(300))
                .await
                .is_empty()
        );
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
    }
}
