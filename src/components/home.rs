use std::{fs, io::Cursor};

use ansi_to_tui::IntoText;
use bytes::Bytes;
use color_eyre::Result;
use image::DynamicImage;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use img_to_ascii::{
    convert::{
        char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
        img_to_char_rows,
    },
    font::Font,
    image::LumaImage,
};
use tracing::info;

use super::Component;
use crate::{action::Action, aic::iiif, ascii_art::ArtBuilder, config::get_data_dir};

const ALPHABET: &[u8] = include_bytes!("../../.data/alphabet.txt");
const BITOCRA_13: &[u8] = include_bytes!("../../.data/bitocra-13.bdf");

#[derive(Default)]
pub struct Home<'a> {
    area: Size,
    text: Text<'a>,
    bytes: Bytes,
    action_tx: Option<UnboundedSender<Action>>,
    art_builder: ArtBuilder,
}

impl<'a> Home<'a> {
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
            tx.send(Action::ToText(image_bytes)).unwrap();
        } else {
            info!("Spawning task to fetch {}", image_id);
            tokio::spawn(async move {
                let image_bytes = image_from_identifier(image_id.clone()).await;
                let _ = fs::write(data_dir, image_bytes.clone());
                tx.send(Action::ToText(image_bytes)).unwrap();
            });
        }

        Ok(())
    }

    fn image_bytes_to_text(&mut self, bytes: Bytes) -> Result<()> {
        self.bytes = bytes.clone();
        self.text = Home::bytes_as_text(bytes, self.area);
        Ok(())
    }

    // TODO: Move this into something else
    fn resize(&mut self, size: Size) -> Result<()> {
        if self.bytes.is_empty() || self.area == size {
            return Ok(());
        }

        info!("Resizing to {:?}", size);
        self.area = size;
        self.text = Home::bytes_as_text(self.bytes.clone(), size);

        Ok(())
    }

    fn bytes_as_text(bytes: Bytes, area: Size) -> Text<'a> {
        let image = bytes_as_dyn_image(bytes).unwrap();
        let ascii = image_as_ascii(image, area);
        ascii.into_text().unwrap()
    }
}

impl<'a> Component for Home<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        self.area = area;
        self.art_builder = self.art_builder.of_size(area.width, area.height);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        #[allow(clippy::single_match)]
        match action {
            // NOTE: This action is triggered when user selects a piece
            Action::LoadImage(image_id) => {
                self.load_image(image_id)?;
            }
            Action::ToText(bytes) => {
                self.image_bytes_to_text(bytes)?;
            }
            Action::Resize(width, height) => self.resize(Size { width, height })?,
            //Action::EnterImageDownload(image_id) => {
            //    info!("Downloading {} ...", image_id);
            //}
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

async fn image_from_identifier(image_id: String) -> bytes::Bytes {
    info!("image_from_identifier({})", image_id);
    iiif::Client::builder()
        .build()
        .image()
        .with_image_id(image_id)
        .request()
        .await
        .unwrap()
        .result()
        .await
        .unwrap()
}

fn bytes_as_dyn_image(bytes: Bytes) -> std::result::Result<image::DynamicImage, image::ImageError> {
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("cursor io never fails");
    reader.decode()
}

fn image_as_ascii(image: DynamicImage, area: Size) -> String {
    let font = font();
    let image_area = PixelArea {
        width: image.width() as usize,
        height: image.height() as usize,
    };
    let frame_area = CharArea {
        width: area.width as usize,
        height: area.height as usize,
    };
    let resized_image_area = image_area.fit_into(frame_area.into_pixels(font.clone()));
    let target_image_width_in_chars = CharArea::from_pixels(resized_image_area, font);
    dyn_image_as_ascii(image, Some(target_image_width_in_chars.width))
}

struct CharArea {
    width: usize,
    height: usize,
}

impl CharArea {
    fn from_pixels(pixels: PixelArea, font: Font) -> CharArea {
        CharArea {
            width: CharArea::round_up(pixels.width, font.width) / font.width,
            height: CharArea::round_up(pixels.height, font.height) / font.width,
        }
    }

    fn into_pixels(self, font: Font) -> PixelArea {
        PixelArea {
            width: self.width * font.width,
            height: self.height * font.height,
        }
    }

    /// Rounds length up to next multiple of segment.
    fn round_up(length: usize, segment: usize) -> usize {
        if length % segment == 0 {
            length
        } else {
            length + 1
        }
    }
}

struct PixelArea {
    width: usize,
    height: usize,
}

impl PixelArea {
    fn fit_into(self, frame: PixelArea) -> PixelArea {
        let scaling_ratio = if frame.width > frame.height {
            // Scale image to frame height
            self.height as f64 / frame.height as f64
        } else {
            // Scale self to frame width
            self.width as f64 / frame.width as f64
        };
        PixelArea {
            width: (self.width as f64 / scaling_ratio) as usize,
            height: (self.height as f64 / scaling_ratio) as usize,
        }
    }
}

fn font() -> Font {
    let alphabet = &ALPHABET.iter().map(|&c| c as char).collect::<Vec<char>>();
    Font::from_bdf_stream(BITOCRA_13, alphabet)
}

fn dyn_image_as_ascii(dyn_img: image::DynamicImage, out_width: Option<usize>) -> String {
    let font = font();

    let luma_img = LumaImage::from(&dyn_img);

    let metric = String::from("direction-and-intensity");
    let convert = get_converter(&metric);

    let brightness_offset = 0.0;

    let algorithm = String::from("edge-augmented");
    let algorithm = get_conversion_algorithm(&algorithm);

    let char_rows = img_to_char_rows(
        &font,
        &luma_img,
        convert,
        out_width,
        brightness_offset,
        &algorithm,
    );

    char_rows_to_terminal_color_string(&char_rows, &dyn_img)
}
