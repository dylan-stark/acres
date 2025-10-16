use acres::Api;
use ansi_to_tui::IntoText;
use bytes::Buf;
use color_eyre::eyre::{self, Context};
use ratatui::{Frame, prelude::Rect, text::Text, widgets::Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, components::Component};

pub struct Ascii<'a> {
    text: Text<'a>,
    action_tx: UnboundedSender<Action>,
}

impl<'a> Ascii<'a> {
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            text: Text::default(),
            action_tx,
        }
    }
}

impl<'a> Component for Ascii<'a> {
    fn update(
        &mut self,
        action: crate::action::Action,
    ) -> eyre::Result<Option<crate::action::Action>> {
        match action {
            Action::View(artwork) => {
                let base_uri: iiif::BaseUri = artwork.try_into()?;
                let image_request = iiif::ImageRequest::builder().base_uri(base_uri).build()?;
                let action_tx = self.action_tx.clone();
                tokio::spawn(async move {
                    let response: iiif::ImageResponse = Api::new()
                        .fetch(image_request.to_string(), None as Option<()>)
                        .await
                        // TODO: Handle errors by sending an appropriate error action through the
                        // channel, instead of just expecting and hoping not to panic.
                        .expect("got bytes");
                    action_tx.send(Action::RenderAscii(response.into()))
                });
                Ok(None)
            }
            Action::RenderAscii(image) => {
                let action_tx = self.action_tx.clone();
                tokio::spawn(async move {
                    // TODO: Handle errors by sending an appropriate error action through the
                    // channel, instead of just discarding.
                    let _ = action_tx.send(Action::StartingRenderAscii);
                    let ascii = image_to_ascii_builder::Ascii::builder()
                        .input_reader(image.reader())
                        // TODO: Handle errors by sending an appropriate error action through the
                        // channel, instead of just expecting and hoping not to panic.
                        .expect("can read image bytes")
                        .build()
                        // TODO: Handle errors by sending an appropriate error action through the
                        // channel, instead of just expecting and hoping not to panic.
                        .expect("can build ascii art");
                    action_tx.send(Action::UpdateAscii(ascii))
                });
                Ok(None)
            }
            Action::UpdateAscii(ascii) => {
                let text = ascii
                    .to_string()
                    .as_bytes()
                    .into_text()
                    .context("failed to convert ascii bytes to text")?;
                self.text = text;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> eyre::Result<()> {
        let paragraph = Paragraph::new(self.text.clone());
        frame.render_widget(paragraph, area);
        Ok(())
    }
}
