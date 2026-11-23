use std::fmt;

/// Errors from building a [`Ramp`](crate::Ramp) or loading an image into a
/// [`Converter`](crate::Converter).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The ramp has no characters.
    EmptyRamp,
    /// The ramp holds a character that is not printable ASCII.
    InvalidRampChar(char),
    /// The pixel buffer's length does not match its width, height and format.
    PixelCountMismatch {
        /// Bytes needed for the given width, height and format.
        expected: usize,
        /// Bytes given.
        actual: usize,
    },
    /// The image is wider than a row of cells can sum without overflow.
    ImageTooWide(u32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::EmptyRamp => f.write_str("empty ramp"),
            Error::InvalidRampChar(c) => write!(f, "ramp character {c:?} is not printable ASCII"),
            Error::PixelCountMismatch { expected, actual } => {
                write!(f, "expected {expected} bytes of pixels, got {actual}")
            }
            Error::ImageTooWide(width) => write!(f, "image width {width} is too large"),
        }
    }
}

impl std::error::Error for Error {}
