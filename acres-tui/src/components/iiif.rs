use acres::Api;
use color_eyre::eyre::{self};
use iiif::{BaseUri, ImageRequest};
use ratatui::{Frame, prelude::Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::Component};

pub struct Iiif {
    base_uri: Option<BaseUri>,
    action_tx: UnboundedSender<Action>,
}

impl Iiif {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            base_uri: None,
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
                self.base_uri = Some(artwork.try_into()?);
                Ok(Some(Action::IiifRequestImage))
            }
            Action::IiifRequestImage => {
                if let Some(base_uri) = &self.base_uri {
                    let image_request =
                        ImageRequest::builder().base_uri(base_uri.clone()).build()?;
                    let action_tx = self.action_tx.clone();
                    tokio::spawn(async move {
                        let response: Option<iiif::ImageResponse> = Api::new()
                            .fetch(image_request.to_string(), None as Option<()>)
                            .await
                            .ok();
                        if let Some(response) = response {
                            let _ = action_tx
                                .send(Action::ImageToAsciiBuilderUpdateImage(response.into()));
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
