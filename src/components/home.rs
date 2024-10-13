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
use crate::{
    action::Action,
    config::{get_data_dir, Config},
    iiif::Iiif2Url,
};

#[derive(Default)]
pub struct Home<'a> {
    area: Size,
    text: Text<'a>,
    bytes: Bytes,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}

impl<'a> Home<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    fn display(&mut self, image_id: String) -> Result<()> {
        let tx = self.command_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterImageDownload(image_id.clone()))
                .unwrap();
            image_ascii(image_id.clone()).await;
            tx.send(Action::ExitImageDownload(image_id)).unwrap();
        });
        Ok(())
    }

    fn load(&mut self, image_id: String) -> Result<()> {
        let mut image_path = get_data_dir();
        image_path.push(format!("{}.jpg", image_id));
        self.bytes = Bytes::from(fs::read(image_path).unwrap());
        self.text = Home::bytes_as_text(self.bytes.clone(), self.area);
        Ok(())
    }

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
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        self.area = area;

        let font_path = get_data_dir().join("bitocra-13.bdf");
        if !font_path.exists() {
            panic!("Didn't find font file: {:?}", font_path)
        }

        let alphabet_path = get_data_dir().join("alphabet.txt");
        if !alphabet_path.exists() {
            panic!("Didn't find alphabet file: {:?}", alphabet_path)
        }

        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if !(action == Action::Render || action == Action::Tick) {
            info!("Updating Home component: {:?}", action);
        }

        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::Resize(w, h) => {
                self.resize(Size::new(w, h))?;
            }
            Action::Display(image_id) => {
                self.display(image_id)?;
            }
            Action::EnterImageDownload(image_id) => {
                info!("Downloading {} ...", image_id);
            }
            Action::ExitImageDownload(image_id) => {
                self.load(image_id)?;
            }
            _ => {}
        }
        Ok(None)
    }

    /// Draws the ASCII image (`text`) centered in the viewing area.
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
        let text = self.text.clone();
        let widget = Paragraph::new(text);
        frame.render_widget(widget, middle);
        Ok(())
    }
}

async fn image_from_identifier(image_id: String) -> bytes::Bytes {
    info!("image_from_identifier({})", image_id);
    let url = Iiif2Url::new().identifier(image_id.as_str()).to_string();
    info!("url: {}", url);
    let response = reqwest::get(url).await.expect("get failed");
    response.bytes().await.expect("failed to get bytes")
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
    let mut alphabet_path = get_data_dir();
    alphabet_path.push("alphabet.txt");
    let alphabet = &fs::read(alphabet_path)
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect::<Vec<char>>();
    let mut font_path = get_data_dir();
    font_path.push("bitocra-13.bdf");
    Font::from_bdf(&font_path, alphabet)
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

pub async fn image_ascii(image_id: String) -> Bytes {
    info!("image_ascii({})", image_id);

    let mut data_dir = get_data_dir().clone();
    data_dir.push(format!("{}.jpg", image_id.clone()));

    if data_dir.exists() {
        info!("Reading {} from disk", image_id);
        //return fs::read_to_string(data_dir).unwrap();
        return Bytes::from(fs::read(data_dir).unwrap());
    }

    info!("Calling image_from_identifier() ...");
    let bytes = image_from_identifier(image_id.clone()).await;

    info!("Writing image to {:?}", data_dir);
    let _ = fs::write(data_dir, bytes.clone());

    bytes
}
