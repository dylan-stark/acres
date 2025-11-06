#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
// TODO: Fill in an example or two.
//! Simple, ergonomic builder for the [`image-to-ascii` crate].
//!
//! [`image-to-ascii` crate]: https://crates.io/crates/image-to-ascii

mod ascii;
use std::num::ParseFloatError;

pub use self::ascii::Ascii;
pub use self::ascii::{ALPHABETS, CONVERSION_ALGORITHMS, FONTS, METRICS};
pub use self::ascii::{Alphabet, BrightnessOffset, CharWidth, ConversionAlgorithm, Font, Metric};

/// Custom error for this crate.
#[derive(Debug, thiserror::Error)]
pub enum ImageToAsciiBuilderError {
    /// A validation error.
    #[error("validation error: {0}")]
    ValidationError(String),
    /// Unusable percentage string.
    #[error("unable to parse offset: {0}")]
    InvalidOffset(#[from] ParseFloatError),
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
