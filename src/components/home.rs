use std::{fs, io::Cursor};

use ansi_to_tui::IntoText;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use image::DynamicImage;
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
    text: Text<'a>,
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
        image_path.push(format!("{}.txt", image_id));
        let raw_text =
            fs::read_to_string(image_path).expect("couldn't read downloaded image ASCII");
        self.text = raw_text.into_text().unwrap();
        Ok(())
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

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
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

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let text = self.text.clone();
        let widget = Paragraph::new(text).centered();
        frame.render_widget(widget, area);
        Ok(())
    }
}

async fn image_from_identifier(image_id: String) -> Result<DynamicImage, image::ImageError> {
    info!("image_from_identifier({})", image_id);
    let url = Iiif2Url::new().identifier(image_id.as_str()).to_string();
    info!("url: {}", url);
    let response = reqwest::get(url).await.expect("get failed");
    let bytes = response.bytes().await.expect("failed to get bytes");
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("cursor io never fails");
    reader.decode()
}

pub async fn image_ascii(image_id: String) -> String {
    info!("image_ascii({})", image_id);

    let mut data_dir = get_data_dir().clone();
    data_dir.push(format!("{}.txt", image_id.clone()));

    if data_dir.exists() {
        info!("Reading {} from disk", image_id);
        return fs::read_to_string(data_dir).unwrap();
    }

    let mut font_path = get_data_dir();
    font_path.push("bitocra-13.bdf");
    let mut alphabet_path = get_data_dir();
    alphabet_path.push("alphabet.txt");
    let alphabet = &fs::read(alphabet_path)
        .unwrap()
        .iter()
        .map(|&c| c as char)
        .collect::<Vec<char>>();
    let font = Font::from_bdf(&font_path, alphabet);

    info!("Calling image_from_identifier() ...");
    let dyn_img = image_from_identifier(image_id.clone()).await.unwrap();
    let luma_img = LumaImage::from(&dyn_img);

    let metric = String::from("direction-and-intensity");
    let convert = get_converter(&metric);

    let out_width = None;

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

    let ascii = char_rows_to_terminal_color_string(&char_rows, &dyn_img);

    info!("Writing imate to {:?}", data_dir);
    let _ = fs::write(data_dir, ascii.clone());

    ascii
}
