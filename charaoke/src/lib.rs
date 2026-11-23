#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
//! Fast image-to-ASCII conversion, built for live terminal rendering.
//!
//! `charaoke` is a from-scratch replacement for the `image-to-ascii` crate, aimed at re-rendering
//! a full terminal pane within a fraction of a 60 FPS frame (see ADR 0008 in the `acres`
//! repository). An image is loaded once into a [`Converter`], which keeps its RGB pixels. Each
//! [`render`](Converter::render) then fits it to a [`Grid`] of cells, averages each cell's color
//! and picks a character from a [`Ramp`] by that color's brightness. The result is a set of
//! [`Cells`], which can be written as plain text or as ANSI-colored text (see ADR 0010).
//! Re-rendering at a new size reuses every buffer.
//!
//! ```rust
//! use std::num::NonZeroU32;
//! use charaoke::{Cells, Converter, Grid, PixelFormat};
//!
//! let nz = |n| NonZeroU32::new(n).unwrap();
//! let (width, height) = (nz(8), nz(8));
//! // A vertical gradient, dark at the top.
//! let pixels: Vec<u8> = (0..64).map(|i| (i / 8 * 36) as u8).collect();
//!
//! let mut converter = Converter::default();
//! converter.set_image(&pixels, width, height, PixelFormat::Luma8).unwrap();
//!
//! let grid = Grid::fit(width, height, nz(4));
//! assert_eq!((grid.cols(), grid.rows()), (4, 2));
//!
//! let mut cells = Cells::default();
//! converter.render(grid, &mut cells);
//!
//! let mut ascii = String::new();
//! cells.write_plain(&mut ascii);
//! assert_eq!(ascii, "::::\n####");
//! ```

mod cells;
mod convert;
mod error;
mod grid;
mod ramp;

pub use self::cells::{Cells, ColorDepth, ansi256};
pub use self::convert::{Converter, PixelFormat};
pub use self::error::Error;
pub use self::grid::{CELL_ASPECT, Grid};
pub use self::ramp::Ramp;
