use acres::Catalog;
use color_eyre::eyre::{self};
use iiif::{BaseUri, Format, ImageRequest, Quality, Region, Rotation, Size};
use ratatui::{Frame, prelude::Rect};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

use crate::{action::Action, components::Component};

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
    /// The image being fetched. A newer request aborts it, which also gives up its place in the
    /// catalog's queue.
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
            Action::IiifUpdateBaseUri(artwork) => {
                tracing::info!("updating base uri");
                tracing::debug!(artwork = ?artwork);
                self.artwork_id = artwork.data.id;
                self.base_uri = Some(artwork.try_into()?);
                tracing::debug!(base_uri = ?self.base_uri);
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
