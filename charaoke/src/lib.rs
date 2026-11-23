#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
//! Fast image-to-ASCII conversion, built for live terminal rendering.
//!
//! `charaoke` is a from-scratch replacement for the `image-to-ascii` crate, aimed at re-rendering
//! a full terminal pane within a fraction of a 60 FPS frame (see ADR 0008 in the `acres`
//! repository). It is early: for now it only sizes the character grid an image maps onto.
//!
//! ```rust
//! use std::num::NonZeroU32;
//! use charaoke::Grid;
//!
//! let width = NonZeroU32::new(843).unwrap();
//! let height = NonZeroU32::new(632).unwrap();
//! let cols = NonZeroU32::new(160).unwrap();
//!
//! let grid = Grid::fit(width, height, cols);
//! assert_eq!(grid.cols(), 160);
//! assert_eq!(grid.rows(), 60);
//! ```

mod grid;

pub use self::grid::{CELL_ASPECT, Grid};
