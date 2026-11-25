use acres::Catalog;
use color_eyre::eyre::{self};
use iiif::{BaseUri, Format, ImageRequest, Quality, Region, Rotation, Size};
use ratatui::{Frame, prelude::Rect};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{action::Action, components::Component};

/// How long the selection must rest on an artwork before its thumbnail is fetched, so holding
/// `j` through a list does not fetch every row (ADR 0013).
pub const PREVIEW_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

/// Width of the thumbnail fetched while browsing, in pixels. It is about a fifteenth of the full
/// image's bytes and still fills a 100-column pane in half blocks.
pub const THUMBNAIL_WIDTH: u32 = 200;

/// Width of the full image, fetched on request: the largest the AIC serves for every artwork.
pub const FULL_WIDTH: u32 = 843;

/// Fetches the shown artwork's image: a thumbnail once the selection rests on it, and the full
/// image when asked.
pub struct Iiif {
    base_uri: Option<BaseUri>,
    artwork_id: u32,
    /// Whether the full image of the shown artwork was asked for, so asking again does nothing.
    full: bool,
    action_tx: UnboundedSender<Action>,
    catalog: Catalog,
    /// The image waiting out the preview delay or being fetched. A newer request aborts it,
    /// which also gives up its place in the catalog's queue.
    pending: Option<JoinHandle<()>>,
}

impl Iiif {
    pub fn new(action_tx: UnboundedSender<Action>, catalog: Catalog) -> Self {
        Self {
            base_uri: None,
            artwork_id: 0,
            full: false,
            action_tx,
            catalog,
            pending: None,
        }
    }

    fn abort_pending(&mut self) {
        if let Some(previous) = self.pending.take() {
            previous.abort();
        }
    }

    /// Fetches the shown artwork's image at `width`, after `delay`, replacing any fetch pending.
    fn fetch(&mut self, width: u32, delay: std::time::Duration) {
        let Some(uri) = &self.base_uri else { return };
        let image_request = ImageRequest::builder()
            .base_uri(uri.clone())
            .region(Region::Full)
            .size(Size::Width(width))
            .rotation(Rotation::Degrees(
                0.0.try_into().expect("0 degrees is a valid setting"),
            ))
            .quality(Quality::Default)
            .format(Format::Jpg)
            .build();
        tracing::debug!(image_request = %image_request);
        let action_tx = self.action_tx.clone();
        let catalog = self.catalog.clone();
        let id = self.artwork_id;
        let full = width == FULL_WIDTH;
        self.abort_pending();
        self.pending = Some(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let start = std::time::Instant::now();
            match catalog.image(&image_request).await {
                Ok(fetched) => {
                    let _ = action_tx.send(Action::UpdateImage {
                        id,
                        image: fetched.value,
                        fetch: start.elapsed(),
                        full,
                    });
                }
                Err(e) => tracing::error!("failed to get image: {e}"),
            }
        }));
    }
}

impl Component for Iiif {
    fn update(&mut self, action: Action) -> eyre::Result<Option<Action>> {
        match action {
            Action::ShowArtwork(entry) => {
                self.artwork_id = entry.id();
                self.base_uri = entry.base_uri();
                self.full = false;
                if self.base_uri.is_none() {
                    tracing::warn!(id = entry.id(), "artwork has no image");
                    self.abort_pending();
                } else {
                    self.fetch(THUMBNAIL_WIDTH, PREVIEW_DELAY);
                }
            }
            Action::ShowFullImage if !self.full && self.base_uri.is_some() => {
                self.full = true;
                self.fetch(FULL_WIDTH, std::time::Duration::ZERO);
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

    fn entry(server: &MockServer, id: u32) -> Entry {
        Entry {
            summary: acres::artworks::ArtworkSummary {
                id,
                image_id: Some(format!("image-{id}")),
                ..Default::default()
            },
            iiif_url: format!("{}/iiif", server.uri()),
        }
    }

    fn image(id: u32, delay: Duration) -> Mock {
        sized(id, THUMBNAIL_WIDTH, delay)
    }

    fn sized(id: u32, width: u32, delay: Duration) -> Mock {
        Mock::given(method("GET"))
            .and(path(format!(
                "/iiif/image-{id}/full/{width},/0/default.jpg"
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![id as u8])
                    .set_delay(delay),
            )
    }

    fn show(iiif: &mut Iiif, entry: Entry) {
        assert_eq!(iiif.update(Action::ShowArtwork(entry)).unwrap(), None);
    }

    /// The images that arrive `within` a time, as `(id, full)`.
    async fn arrivals(
        rx: &mut mpsc::UnboundedReceiver<Action>,
        within: Duration,
    ) -> Vec<(u32, bool)> {
        let mut ids = Vec::new();
        let _ = tokio::time::timeout(within, async {
            while let Some(action) = rx.recv().await {
                if let Action::UpdateImage { id, full, .. } = action {
                    ids.push((id, full));
                }
            }
        })
        .await;
        ids
    }

    async fn images(rx: &mut mpsc::UnboundedReceiver<Action>, within: Duration) -> Vec<u32> {
        arrivals(rx, within)
            .await
            .into_iter()
            .map(|(id, _)| id)
            .collect()
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

    #[tokio::test]
    async fn moving_on_within_the_preview_delay_fetches_only_the_last() {
        let server = MockServer::start().await;
        image(1, Duration::ZERO).expect(0).mount(&server).await;
        image(2, Duration::ZERO).expect(1).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        show(&mut iiif, entry(&server, 2));
        assert_eq!(images(&mut rx, Duration::from_millis(600)).await, [2]);
    }

    #[tokio::test]
    async fn an_aborted_fetch_never_reaches_the_art() {
        let server = MockServer::start().await;
        image(1, Duration::from_millis(800)).mount(&server).await;
        image(2, Duration::ZERO).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        // Past the preview delay, so the first image is on the wire.
        tokio::time::sleep(PREVIEW_DELAY * 2).await;
        show(&mut iiif, entry(&server, 2));
        assert_eq!(images(&mut rx, Duration::from_millis(1200)).await, [2]);
    }

    #[tokio::test]
    async fn the_full_image_comes_only_when_asked_for_and_only_once() {
        let server = MockServer::start().await;
        image(1, Duration::ZERO).expect(1).mount(&server).await;
        sized(1, FULL_WIDTH, Duration::ZERO)
            .expect(1)
            .mount(&server)
            .await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(600)).await,
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
    async fn asking_for_the_full_image_skips_the_thumbnail_still_waiting() {
        let server = MockServer::start().await;
        image(1, Duration::ZERO).expect(0).mount(&server).await;
        sized(1, FULL_WIDTH, Duration::ZERO).mount(&server).await;
        let (mut iiif, mut rx) = iiif(&server);
        show(&mut iiif, entry(&server, 1));
        iiif.update(Action::ShowFullImage).unwrap();
        assert_eq!(
            arrivals(&mut rx, Duration::from_millis(600)).await,
            [(1, true)]
        );
    }
}
