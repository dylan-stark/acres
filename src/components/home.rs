use std::fs;

use ansi_to_tui::IntoText;
use bytes::Bytes;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use tracing::info;

use super::Component;
use crate::{action::Action, aic::iiif, ascii_art::ArtBuilder, config::get_data_dir};

#[derive(Default)]
pub struct Home {
    action_tx: Option<UnboundedSender<Action>>,
    art_builder: ArtBuilder,
    text: Text<'static>,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads the image to display and sends them to be processed to text.
    ///
    /// If the image has already been fetched, the bytes will be on disk
    /// at a known location, and we should prefer those. Otherwise, we'll
    /// spawn a task to fetch the bytes and write them to disk. In either
    /// case we send the bytes along to get processed after we have them.
    fn load_image(&mut self, image_id: String) -> Result<()> {
        let tx = self.action_tx.clone().expect("no sender");

        let mut data_dir = get_data_dir().clone();
        data_dir.push(format!("{}.jpg", image_id.clone()));

        if data_dir.exists() {
            info!("Reading {} from disk", image_id);
            let image_bytes = Bytes::from(fs::read(data_dir).unwrap());
            tx.send(Action::ToAscii(image_bytes)).unwrap();
        } else {
            info!("Spawning task to fetch {}", image_id);
            tokio::spawn(async move {
                let image_bytes = iiif::Client::builder()
                    .build()
                    .image()
                    .with_image_id(image_id)
                    .request()
                    .await
                    .unwrap()
                    .result()
                    .await
                    .unwrap();
                let _ = fs::write(data_dir, image_bytes.clone());
                tx.send(Action::ToAscii(image_bytes)).unwrap();
            });
        }

        Ok(())
    }

    /// Converts image bytes to TUI text.
    fn image_bytes_to_ascii(&mut self, bytes: Bytes) -> Result<()> {
        self.art_builder = self.art_builder.clone().with_bytes(bytes);
        Ok(())
    }

    fn resize(&mut self, size: Size) -> Result<()> {
        info!("Resizing to {:?}", size);
        self.art_builder = self.art_builder.clone().with_size(size.width, size.height);
        Ok(())
    }

    fn to_text(&mut self) -> Result<()> {
        self.text = self.art_builder.clone().into_ascii().into_text().unwrap();
        Ok(())
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        self.art_builder = self.art_builder.clone().with_size(area.width, area.height);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        #[allow(clippy::single_match)]
        match action {
            Action::LoadImage(image_id) => {
                self.load_image(image_id)?;
                // Above sends Action::ToAscii from async context
            }
            Action::ToAscii(bytes) => {
                self.image_bytes_to_ascii(bytes)?;
                return Ok(Some(Action::ToText));
            }
            Action::Resize(width, height) => {
                self.resize(Size { width, height })?;
                return Ok(Some(Action::ToText));
            }
            Action::ToText => self.to_text()?,
            _ => {}
        }
        Ok(None)
    }

    /// Draws the ASCII image (`text`) centered in the viewing area.
    ///
    /// This is called either while Rendering or Resizing. In the case
    /// of Resizing, it is called after the TUI is resized but before
    /// the component receives the Resize action.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Min(self.text.width() as u16),
            Constraint::Fill(1),
        ])
        .areas(area);
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Min(self.text.height() as u16),
            Constraint::Fill(1),
        ])
        .areas(middle);
        let widget = Paragraph::new(self.text.clone());
        frame.render_widget(widget, middle);
        Ok(())
    }
}
