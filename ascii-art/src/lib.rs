#![deny(missing_docs)]

//! Simple ASCII art generator.

use anyhow::Context;
use bytes::{Buf, Bytes};

use std::{
    fmt::Display,
    io::{Cursor, Read},
};

use anyhow::Result;
use image::io::Reader;
use img_to_ascii::{
    convert::{
        char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
        img_to_char_rows,
    },
    font,
    image::LumaImage,
};

/// Built-in alphabets.
#[derive(Default)]
pub enum Alphabet {
    /// The alphabet alphabet.
    #[default]
    Alphabet,
    /// The letters alphabet.
    Letters,
    /// The lowercase alphabet.
    Lowercase,
    /// The minimal alphabet.
    Minimal,
    /// The symbols alphabet.
    Symbols,
    /// The uppercase alphabet.
    Uppercase,
}

impl From<Alphabet> for Bytes {
    fn from(value: Alphabet) -> Self {
        match value {
            Alphabet::Alphabet => Bytes::from_static(include_bytes!("../.data/alphabet.txt")),
            Alphabet::Letters => Bytes::from_static(include_bytes!("../.data/letters.txt")),
            Alphabet::Lowercase => Bytes::from_static(include_bytes!("../.data/lowercase.txt")),
            Alphabet::Minimal => Bytes::from_static(include_bytes!("../.data/minimal.txt")),
            Alphabet::Symbols => Bytes::from_static(include_bytes!("../.data/symbols.txt")),
            Alphabet::Uppercase => Bytes::from_static(include_bytes!("../.data/uppercase.txt")),
        }
    }
}

/// Sets the alphabet from reader.
impl From<Alphabet> for Vec<char> {
    fn from(value: Alphabet) -> Self {
        let mut reader = Bytes::from(value).reader();
        let mut bytes: Vec<u8> = Vec::new();
        let _n = reader
            .read_to_end(&mut bytes)
            .context("failed to read to end of alphabet");
        bytes.iter().map(|&c| c as char).collect()
    }
}

/// Built-in fonts.
#[derive(Default)]
pub enum Font {
    /// The courier font
    Courier,
    /// The bitocra-13 font
    #[default]
    BitOcra13,
}

impl From<Font> for Bytes {
    fn from(value: Font) -> Self {
        match value {
            Font::Courier => Bytes::from_static(include_bytes!("../.data/courier.bdf")),
            Font::BitOcra13 => Bytes::from_static(include_bytes!("../.data/bitocra-13.bdf")),
        }
    }
}

/// An ASCII Art error.
#[derive(Debug, thiserror::Error)]
pub enum AsciiArtError {
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// ASCII Art.
#[derive(Debug)]
pub struct AsciiArt(String);

impl Display for AsciiArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl AsciiArt {
    /// Creates a new ASCII art builder.
    pub fn builder() -> AsciiArtBuilder {
        AsciiArtBuilder::default()
    }
}

/// An ASCII art builder.
#[derive(Default)]
pub struct AsciiArtBuilder {
    alphabet: Alphabet,
    font: Font,
    chars_wide: usize,
    input_bytes: Bytes,
}

impl AsciiArtBuilder {
    /// Creates a new ASCII art builder.
    pub fn new() -> Self {
        AsciiArtBuilder::default()
    }

    /// Sets the alphabet from built-in.
    pub fn alphabet(mut self, alphabet: Alphabet) -> Self {
        //self.alphabet_reader(Bytes::from(alphabet).reader())
        self.alphabet = alphabet;
        self
    }

    /// Sets the font from built-in.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the input image bytes from reader.
    pub fn input_reader(mut self, mut reader: impl Read) -> Result<Self, AsciiArtError> {
        let mut bytes: Vec<u8> = Vec::new();
        let _n = reader
            .read_to_end(&mut bytes)
            .context("failed to read bytes from reader")?;
        let bytes = Bytes::from(bytes);
        self.input_bytes = bytes;
        Ok(self)
    }

    /// Sets desired width in chars.
    pub fn chars_wide(mut self, size: usize) -> Self {
        self.chars_wide = size;
        self
    }

    /// Builds ASCII art.
    pub fn build(self) -> Result<AsciiArt, AsciiArtError> {
        tracing::info!("converting bytes to ascii");
        let dyn_img = Reader::new(Cursor::new(self.input_bytes))
            .with_guessed_format()
            .context("image reader failed")?
            .decode()
            .context("image decode failed")?;
        let font = font::Font::from_bdf_stream(
            Bytes::from(self.font).reader(),
            &Vec::from(self.alphabet),
            false,
        );
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
        Ok(AsciiArt(char_rows_to_terminal_color_string(
            &char_rows, &dyn_img,
        )))
    }
}
