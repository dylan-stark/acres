#![deny(missing_docs)]

//! Simple ASCII art generator.

use anyhow::Context;
use bytes::Bytes;

use std::io::Cursor;

use anyhow::Result;
use image::io::Reader;
use img_to_ascii::{
    convert::{
        char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
        img_to_char_rows,
    },
    font::Font,
    image::LumaImage,
};

/// An ASCII Art error.
#[derive(Debug, thiserror::Error)]
pub enum AsciiArtError {
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// ASCII Art.
#[derive(Debug)]
pub struct AsciiArt {
    /// How many characters wide you want
    pub chars_wide: usize,
}

impl AsciiArt {
    /// Converts bytes to ASCII.
    pub fn bytes_to_ascii(self, bytes: Bytes) -> Result<String, AsciiArtError> {
        tracing::info!("converting bytes to ascii");
        tracing::debug!(?self);
        const ALPHABET: &[u8] = include_bytes!("../.data/alphabet.txt");
        const BITOCRA_13: &[u8] = include_bytes!("../.data/bitocra-13.bdf");
        let dyn_img = Reader::new(Cursor::new(bytes))
            .with_guessed_format()
            .context("image reader failed")?
            .decode()
            .context("image decode failed")?;
        let alphabet = &ALPHABET.iter().map(|&c| c as char).collect::<Vec<char>>();
        let font = Font::from_bdf_stream(BITOCRA_13, alphabet, false);
        let luma_img = LumaImage::from(&dyn_img);
        let convert = get_converter("direction-and-intensity");
        let brightness_offset = 0.;
        let algorithm = get_conversion_algorithm("edge-augmented");
        let char_rows = img_to_char_rows(
            &font,
            &luma_img,
            convert,
            Some(self.chars_wide),
            brightness_offset,
            &algorithm,
        );
        Ok(char_rows_to_terminal_color_string(&char_rows, &dyn_img))
    }
}
