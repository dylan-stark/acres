use acres::Api;
use color_eyre::eyre::{self};
use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
use ratatui::{Frame, prelude::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::Component};

pub struct Iiif {
    base_uri: Option<Uri>,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
    action_tx: UnboundedSender<Action>,
}

impl Iiif {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            base_uri: None,
            region: Region::Full,
            size: Size::Width(843),
            rotation: Rotation::Degrees(0.0.try_into().expect("0 degrees is a valid setting")),
            quality: Quality::Default,
            format: Format::Jpg,
            action_tx,
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
                self.base_uri = Some(artwork.try_into()?);
                tracing::debug!(base_uri = ?self.base_uri);
                Ok(Some(Action::IiifRequestImage))
            }
            Action::IiifRequestImage => {
                if let Some(uri) = &self.base_uri {
                    tracing::debug!(uri = %uri, raw_uri = ?uri);
                    let image_request = ImageRequest::builder()
                        .uri(uri.clone())
                        .region(self.region.clone())
                        .size(self.size.clone())
                        .rotation(self.rotation.clone())
                        .quality(self.quality.clone())
                        .format(self.format.clone())
                        .build();
                    tracing::debug!(image_request = %image_request, raw_image_request = ?image_request);

                    let action_tx = self.action_tx.clone();
                    tokio::spawn(async move {
                        let response: Option<bytes::Bytes> = Api::new()
                            .fetch(image_request.to_string())
                            .await
                            .inspect_err(|e| tracing::error!("failed to get image: {e}"))
                            .ok();
                        if let Some(response) = response {
                            let _ =
                                action_tx.send(Action::ImageToAsciiBuilderUpdateImage(response));
                        }
                    });
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
