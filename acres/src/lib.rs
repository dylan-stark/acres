#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! This library powers the [`acres-cli`] and [`acres-tui`].
//!
//! We provide [builders] for most everything. Where possible, sensible defaults are used so that
//! you only need provide specific details for your use case.
//!
//! For instance, if you need a specific artwork, you can get that with just the ID.
//!
//! ```ignore,no_run
//! # use anyhow::Result;
//! use acres::artworks::Artwork;
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let collection = Artwork::builder().id(77733).build();
//! println!("{}", collection);
//! # Ok(())
//! # }
//! ```
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction
//! [`acres-cli`]: ../acres_cli/index.html
//! [`acres-tui`]: ../acres_tui/index.html

mod api;
pub mod artworks;
mod common;
mod config;

pub use api::fetch;
pub use api::{Api, Cached};

/// An Acres error.
#[derive(Debug, thiserror::Error)]
pub enum AcresError {
    /// An artwork-related error
    #[error("unable to load artwork info")]
    LoadArtworkInfo,
    /// Unable to parse IIIF scheme
    #[error("IIIF error: {0}")]
    Iiif(#[from] iiif::IiifError),
    /// A search query parameter error
    #[error("search query parameters error: {0}")]
    InvalidSearchQueryParams(String),
    /// An unexpected error.
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
