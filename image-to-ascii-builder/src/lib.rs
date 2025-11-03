#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
// TODO: Fill in an example or two.
//! Simple, ergonomic builder for the [`image-to-ascii` crate].
//!
//! [`image-to-ascii` crate]: https://crates.io/crates/image-to-ascii

use anyhow::Context;
use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

use std::{
    fmt::Display,
    io::{Cursor, Read},
    str::FromStr,
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

/// Custom error for this crate.
#[derive(Debug, thiserror::Error)]
pub enum ImageToAsciiBuilderError {
    /// A validation error.
    #[error("validation error: {0}")]
    ValidationError(String),
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

/// Defines a conversion algorithm.
///
/// There are four options: `base`, `edge`, `edge-augmented` and `two-pass`; and `edge-augmented`
/// is the default.
///
/// You can create one from a string, if that's what you have, or directly if you know what you
/// want.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use image_to_ascii_builder::ConversionAlgorithm;
///
/// # fn main() -> Result<()> {
/// let algo: ConversionAlgorithm = "two-pass".parse()?;
/// assert_eq!(algo, ConversionAlgorithm::TwoPass);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub enum ConversionAlgorithm {
    /// Base.
    Base,
    /// Edge.
    Edge,
    /// EdgeAugmented.
    #[default]
    EdgeAugmented,
    /// TwoPass.
    TwoPass,
}

/// All conversion algorithms
pub const CONVERSION_ALGORITHMS: &[ConversionAlgorithm] = &[
    ConversionAlgorithm::Base,
    ConversionAlgorithm::Edge,
    ConversionAlgorithm::EdgeAugmented,
    ConversionAlgorithm::TwoPass,
];

impl Display for ConversionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionAlgorithm::Base => f.write_str("base"),
            ConversionAlgorithm::Edge => f.write_str("edge"),
            ConversionAlgorithm::EdgeAugmented => f.write_str("edge-augmented"),
            ConversionAlgorithm::TwoPass => f.write_str("two-pass"),
        }
    }
}

impl FromStr for ConversionAlgorithm {
    type Err = ImageToAsciiBuilderError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            _ if s == "base" => Ok(ConversionAlgorithm::Base),
            _ if s == "edge" => Ok(ConversionAlgorithm::Edge),
            _ if s == "edge-augmented" => Ok(ConversionAlgorithm::EdgeAugmented),
            _ if s == "two-pass" => Ok(ConversionAlgorithm::TwoPass),
            _ => Err(ImageToAsciiBuilderError::ValidationError(format!(
                "{} is not a supported metric",
                s
            ))),
        }
    }
}

/// Defines an alphabet.
///
/// There are seven options: `alphabet`, `fast`, `letters`, `lowercase`, `minimal`, `symbols`, and
/// `uppercase`; and `alphabet` is the default.
///
/// You can create one from a string, if that's what you have, or directly if you know what you
/// want.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use image_to_ascii_builder::Alphabet;
///
/// # fn main() -> Result<()> {
/// let alpha: Alphabet = "fast".parse()?;
/// assert_eq!(alpha, Alphabet::Fast);
/// # Ok(())
/// # }
/// ```
///
/// We implement conversion to `bytes::Bytes` and `Vec<char>` so that can easily get the backing content for any of the variants.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use image_to_ascii_builder::Alphabet;
/// # fn main() -> Result<()> {
/// let content: bytes::Bytes = Alphabet::Fast.into();
/// let content: Vec<char> = Alphabet::Fast.into();
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Alphabet {
    /// The alphabet alphabet.
    #[default]
    Alphabet,
    /// The fast alphabet.
    Fast,
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

/// All alphabets.
pub const ALPHABETS: &[Alphabet] = &[
    Alphabet::Alphabet,
    Alphabet::Fast,
    Alphabet::Letters,
    Alphabet::Lowercase,
    Alphabet::Minimal,
    Alphabet::Symbols,
    Alphabet::Uppercase,
];

impl Display for Alphabet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Alphabet::Alphabet => f.write_str("alphabet"),
            Alphabet::Fast => f.write_str("fast"),
            Alphabet::Letters => f.write_str("letters"),
            Alphabet::Lowercase => f.write_str("lowercase"),
            Alphabet::Minimal => f.write_str("minimal"),
            Alphabet::Symbols => f.write_str("symbols"),
            Alphabet::Uppercase => f.write_str("uppercase"),
        }
    }
}

impl FromStr for Alphabet {
    type Err = ImageToAsciiBuilderError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            _ if s == "alphabet" => Ok(Alphabet::Alphabet),
            _ if s == "fast" => Ok(Alphabet::Fast),
            _ if s == "letters" => Ok(Alphabet::Letters),
            _ if s == "lowercase" => Ok(Alphabet::Lowercase),
            _ if s == "minimal" => Ok(Alphabet::Minimal),
            _ if s == "symbols" => Ok(Alphabet::Symbols),
            _ if s == "uppercase" => Ok(Alphabet::Uppercase),
            _ => Err(ImageToAsciiBuilderError::ValidationError(format!(
                "{} is not a supported alphabet",
                s
            ))),
        }
    }
}

impl From<Alphabet> for Bytes {
    fn from(value: Alphabet) -> Self {
        match value {
            Alphabet::Alphabet => Bytes::from_static(include_bytes!("../.data/alphabet.txt")),
            Alphabet::Fast => Bytes::from_static(include_bytes!("../.data/fast.txt")),
            Alphabet::Letters => Bytes::from_static(include_bytes!("../.data/letters.txt")),
            Alphabet::Lowercase => Bytes::from_static(include_bytes!("../.data/lowercase.txt")),
            Alphabet::Minimal => Bytes::from_static(include_bytes!("../.data/minimal.txt")),
            Alphabet::Symbols => Bytes::from_static(include_bytes!("../.data/symbols.txt")),
            Alphabet::Uppercase => Bytes::from_static(include_bytes!("../.data/uppercase.txt")),
        }
    }
}

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

/// Brightness offset.
#[derive(Clone, Debug)]
pub struct BrightnessOffset(f32);

impl Default for BrightnessOffset {
    fn default() -> Self {
        Self(0.0)
    }
}

impl From<BrightnessOffset> for f32 {
    fn from(value: BrightnessOffset) -> Self {
        value.0
    }
}

impl TryFrom<&str> for BrightnessOffset {
    type Error = ImageToAsciiBuilderError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.parse::<f32>() {
            Ok(offset) => Ok(BrightnessOffset(offset)),
            Err(error) => Err(ImageToAsciiBuilderError::ValidationError(error.to_string())),
        }
    }
}

impl BrightnessOffset {
    /// Creates a new brightness offset value.
    pub fn new(offset: f32) -> Result<Self, ImageToAsciiBuilderError> {
        match offset {
            _ if (0.0..=255.0).contains(&offset) => Ok(Self(offset)),
            _ => Err(ImageToAsciiBuilderError::ValidationError(String::from(
                "brightness offset must be between 0 and 225",
            ))),
        }
    }

    /// Brightness offset parser.
    pub fn parse(value: &str) -> Result<BrightnessOffset, String> {
        match BrightnessOffset::try_from(value) {
            Ok(offset) => Ok(offset),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Built-in fonts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Font {
    /// The courier font
    Courier,
    /// The bitocra-13 font
    #[default]
    BitOcra13,
}

/// All fonts.
pub const FONTS: &[Font] = &[Font::Courier, Font::BitOcra13];

impl From<Font> for Bytes {
    fn from(value: Font) -> Self {
        match value {
            Font::Courier => Bytes::from_static(include_bytes!("../.data/courier.bdf")),
            Font::BitOcra13 => Bytes::from_static(include_bytes!("../.data/bitocra-13.bdf")),
        }
    }
}

impl TryFrom<&str> for Font {
    type Error = ImageToAsciiBuilderError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == "courier" => Ok(Font::Courier),
            _ if value == "bitocra-13" => Ok(Font::BitOcra13),
            _ => Err(ImageToAsciiBuilderError::ValidationError(format!(
                "{} is not a supported font",
                value
            ))),
        }
    }
}

impl Font {
    /// Font parser.
    pub fn parse(value: &str) -> Result<Font, String> {
        match value.try_into() {
            Ok(font) => Ok(font),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl Display for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Font::Courier => f.write_str("courier"),
            Font::BitOcra13 => f.write_str("bitorcra13"),
        }
    }
}

/// Metrics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum Metric {
    /// Dot.
    Dot,
    /// Jaccard.
    Jaccard,
    /// Occlusion.
    Occlusion,
    /// Color.
    Color,
    /// Clear.
    Clear,
    /// Fast.
    Fast,
    /// Intensity.
    Intensity,
    /// Grad.
    Grad,
    /// DirectionAndIntensity.
    #[default]
    DirectionAndIntensity,
    /// Direction.
    Direction,
    /// IntensityJaccard.
    IntensityJaccard,
}

/// All metrics
pub const METRICS: &[Metric] = &[
    Metric::Dot,
    Metric::Jaccard,
    Metric::Occlusion,
    Metric::Color,
    Metric::Clear,
    Metric::Fast,
    Metric::Intensity,
    Metric::Grad,
    Metric::DirectionAndIntensity,
    Metric::Direction,
    Metric::IntensityJaccard,
];

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Metric::Dot => f.write_str("dot"),
            Metric::Jaccard => f.write_str("jaccard"),
            Metric::Occlusion => f.write_str("occlusion"),
            Metric::Color => f.write_str("color"),
            Metric::Clear => f.write_str("clear"),
            Metric::Fast => f.write_str("fast"),
            Metric::Intensity => f.write_str("intensity"),
            Metric::Grad => f.write_str("grad"),
            Metric::DirectionAndIntensity => f.write_str("direction-and-intensity"),
            Metric::Direction => f.write_str("direction"),
            Metric::IntensityJaccard => f.write_str("intensity-jaccard"),
        }
    }
}

impl TryFrom<&str> for Metric {
    type Error = ImageToAsciiBuilderError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            _ if value == "dot" => Ok(Metric::Dot),
            _ if value == "jaccard" => Ok(Metric::Jaccard),
            _ if value == "occlusion" => Ok(Metric::Occlusion),
            _ if value == "color" => Ok(Metric::Color),
            _ if value == "clear" => Ok(Metric::Clear),
            _ if value == "fast" => Ok(Metric::Fast),
            _ if value == "intensity" => Ok(Metric::Intensity),
            _ if value == "grad" => Ok(Metric::Grad),
            _ if value == "direction-and-intensity" => Ok(Metric::DirectionAndIntensity),
            _ if value == "direction" => Ok(Metric::Direction),
            _ if value == "intensity-jaccard" => Ok(Metric::IntensityJaccard),
            _ => Err(ImageToAsciiBuilderError::ValidationError(format!(
                "{} is not a supported metric",
                value
            ))),
        }
    }
}

impl Metric {
    /// Metric parser.
    pub fn parse(value: &str) -> Result<Metric, String> {
        match Metric::try_from(value) {
            Ok(metric) => Ok(metric),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// Width in characters.
#[derive(Clone, Debug, Default)]
pub enum CharWidth {
    /// Use number of chars needed to cover image width
    #[default]
    ImageWidthInChars,
    /// Use this many chars
    CharsWide(usize),
}

impl From<CharWidth> for Option<usize> {
    fn from(value: CharWidth) -> Self {
        match value {
            CharWidth::ImageWidthInChars => None,
            CharWidth::CharsWide(width) => Some(width),
        }
    }
}

impl TryFrom<&str> for CharWidth {
    type Error = ImageToAsciiBuilderError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.parse::<usize>() {
            Ok(width) => Ok(CharWidth::CharsWide(width)),
            Err(error) => Err(ImageToAsciiBuilderError::ValidationError(error.to_string())),
        }
    }
}

impl CharWidth {
    /// Character-width parser.
    pub fn parse(value: &str) -> Result<CharWidth, String> {
        match CharWidth::try_from(value) {
            Ok(width) => Ok(width),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// ASCII.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Ascii(String);

impl Display for Ascii {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl Ascii {
    /// Creates a new ASCII builder.
    pub fn builder() -> AsciiBuilder {
        AsciiBuilder::default()
    }
}

/// An ASCII builder.
#[derive(Debug, Default)]
pub struct AsciiBuilder {
    alphabet: Alphabet,
    brightness_offset: BrightnessOffset,
    chars_wide: CharWidth,
    conversion_algorithm: ConversionAlgorithm,
    font: Font,
    input_bytes: Bytes,
    metric: Metric,
}

impl AsciiBuilder {
    /// Creates a new ASCII builder.
    pub fn new() -> Self {
        AsciiBuilder::default()
    }

    /// Sets the alphabet from built-in.
    pub fn alphabet(mut self, alphabet: Option<Alphabet>) -> Self {
        if let Some(alphabet) = alphabet {
            self.alphabet = alphabet;
        }
        self
    }

    /// Sets the brightness-offset.
    pub fn brightness_offset(mut self, offset: Option<BrightnessOffset>) -> Self {
        if let Some(offset) = offset {
            self.brightness_offset = offset;
        }
        self
    }

    /// Sets desired width in chars.
    pub fn chars_wide(mut self, width: Option<CharWidth>) -> Self {
        if let Some(width) = width {
            self.chars_wide = width;
        }
        self
    }

    /// Sets conversion algorithm.
    pub fn conversion_algorithm(mut self, algorithm: Option<ConversionAlgorithm>) -> Self {
        if let Some(algorithm) = algorithm {
            self.conversion_algorithm = algorithm;
        }
        self
    }

    /// Sets the font from built-in.
    pub fn font(mut self, font: Option<Font>) -> Self {
        if let Some(font) = font {
            self.font = font;
        }
        self
    }

    /// Sets the input image bytes from reader.
    pub fn input_reader(mut self, mut reader: impl Read) -> Result<Self, ImageToAsciiBuilderError> {
        let mut bytes: Vec<u8> = Vec::new();
        let _n = reader
            .read_to_end(&mut bytes)
            .context("failed to read bytes from reader")?;
        let bytes = Bytes::from(bytes);
        self.input_bytes = bytes;
        Ok(self)
    }

    /// Sets the input bytes directly.
    pub fn input(mut self, bytes: Bytes) -> Self {
        self.input_bytes = bytes;
        self
    }

    /// Sets the metric.
    pub fn metric(mut self, metric: Option<Metric>) -> Self {
        if let Some(metric) = metric {
            self.metric = metric;
        }
        self
    }

    /// Builds ASCII.
    pub fn build(self) -> Result<Ascii, ImageToAsciiBuilderError> {
        tracing::info!("converting bytes to ascii");
        let dyn_img = Reader::new(Cursor::new(self.input_bytes))
            .with_guessed_format()
            .context("image reader failed")?
            .decode()
            .context("image decode failed")?;
        Ok(Ascii(char_rows_to_terminal_color_string(
            &img_to_char_rows(
                &font::Font::from_bdf_stream(
                    Bytes::from(self.font).reader(),
                    &Vec::from(self.alphabet),
                    false,
                ),
                &LumaImage::from(&dyn_img),
                get_converter(&self.metric.to_string()),
                self.chars_wide.into(),
                self.brightness_offset.into(),
                &get_conversion_algorithm(&self.conversion_algorithm.to_string()),
            ),
            &dyn_img,
        )))
    }
}
