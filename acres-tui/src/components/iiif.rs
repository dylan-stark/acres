use acres::Catalog;
use color_eyre::eyre::{self};
use iiif::{BaseUri, Format, ImageRequest, Quality, Region, Rotation, Size};
use ratatui::{Frame, prelude::Rect};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{action::Action, components::Component};

/// How long the selection must rest on an artwork before its image is fetched, so holding `j`
/// through a list does not fetch every row (ADR 0013).
pub const PREVIEW_DELAY: std::time::Duration = std::time::Duration::from_millis(150);

pub struct Iiif {
    base_uri: Option<BaseUri>,
    artwork_id: u32,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
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
            region: Region::Full,
            size: Size::Width(843),
            rotation: Rotation::Degrees(0.0.try_into().expect("0 degrees is a valid setting")),
            quality: Quality::Default,
            format: Format::Jpg,
            action_tx,
            catalog,
            pending: None,
        }
    }
}

impl Component for Iiif {
    fn update(
        &mut self,
        action: crate::action::Action,
    ) -> eyre::Result<Option<crate::action::Action>> {
        match action {
            Action::ShowArtwork(entry) => {
                self.artwork_id = entry.id();
                self.base_uri = entry.base_uri();
                if self.base_uri.is_none() {
                    tracing::warn!(id = entry.id(), "artwork has no image");
                    if let Some(previous) = self.pending.take() {
                        previous.abort();
                    }
                    return Ok(None);
                }
                Ok(Some(Action::IiifRequestImage))
            }
            Action::IiifRequestImage => {
                if let Some(uri) = &self.base_uri {
                    tracing::debug!(uri = %uri, raw_uri = ?uri);
                    let image_request = ImageRequest::builder()
                        .base_uri(uri.clone())
                        .region(self.region.clone())
                        .size(self.size.clone())
                        .rotation(self.rotation.clone())
                        .quality(self.quality)
                        .format(self.format)
                        .build();
                    tracing::debug!(image_request = %image_request, raw_image_request = ?image_request);

                    let action_tx = self.action_tx.clone();
                    let catalog = self.catalog.clone();
                    let id = self.artwork_id;
                    if let Some(previous) = self.pending.take() {
                        previous.abort();
                    }
                    self.pending = Some(tokio::spawn(async move {
                        tokio::time::sleep(PREVIEW_DELAY).await;
                        let start = std::time::Instant::now();
                        match catalog.image(&image_request).await {
                            Ok(fetched) => {
                                let _ = action_tx.send(Action::UpdateImage {
                                    id,
                                    image: fetched.value,
                                    fetch: start.elapsed(),
                                });
                            }
                            Err(e) => tracing::error!("failed to get image: {e}"),
                        }
                    }));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
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
        Mock::given(method("GET"))
            .and(path(format!("/iiif/image-{id}/full/843,/0/default.jpg")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![id as u8])
                    .set_delay(delay),
            )
    }

    /// Shows `entry` the way the app does: the request comes back as an action to apply.
    fn show(iiif: &mut Iiif, entry: Entry) {
        let next = iiif.update(Action::ShowArtwork(entry)).unwrap();
        assert_eq!(next, Some(Action::IiifRequestImage));
        iiif.update(Action::IiifRequestImage).unwrap();
    }

    async fn images(rx: &mut mpsc::UnboundedReceiver<Action>, within: Duration) -> Vec<u32> {
        let mut ids = Vec::new();
        let _ = tokio::time::timeout(within, async {
            while let Some(action) = rx.recv().await {
                if let Action::UpdateImage { id, .. } = action {
                    ids.push(id);
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
}
