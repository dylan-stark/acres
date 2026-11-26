use std::fmt;

/// Errors from building a [`Ramp`](crate::Ramp) or a [`View`](crate::View), or loading an image
/// into a [`Converter`](crate::Converter).
#[derive(Debug, Clone, PartialEq)]
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
    /// A [`View`](crate::View) that is not a finite, non-empty rectangle within the image.
    InvalidView {
        /// Left edge, or center for [`View::centered`](crate::View::centered).
        x: f64,
        /// Top edge, or center.
        y: f64,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
    },
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
            Error::InvalidView {
                x,
                y,
                width,
                height,
            } => write!(
                f,
                "view {width} x {height} at {x}, {y} is not a rectangle within the image"
            ),
        }
    }
}

impl std::error::Error for Error {}
