#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
//! Simple, ergonomic builder for the [`image-to-ascii` crate].
//!
//! This crate adds a [builder] and types to make generating ASCII easier.
//!
//! ```rust
//! # use anyhow::Result;
//! # use std::str::FromStr;
//! use image_to_ascii_builder::{Alphabet, Ascii, Font};
//!
//! # fn main() -> Result<()> {
//! let ascii = Ascii::builder()
//!     .alphabet(Alphabet::Fast)
//!     .font(Font::Courier)
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! [`image-to-ascii` crate]: https://crates.io/crates/image-to-ascii
//! [builder]: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html

mod ascii;
use std::num::ParseFloatError;

pub use self::ascii::Ascii;
pub use self::ascii::{ALPHABETS, CONVERSION_ALGORITHMS, FONTS, METRICS};
pub use self::ascii::{
    Alphabet, BrightnessOffset, CharWidth, ConversionAlgorithm, Font, Metric, Offset,
};

/// The ASCII builder.
pub mod builder {
    pub use super::ascii::Builder;
}

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
