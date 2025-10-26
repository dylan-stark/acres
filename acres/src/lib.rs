#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! Create an API client and list artworks with
//!
//! ```no_run
//! use acres::artworks;
//! # use anyhow::Result;
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let collection = artworks::CollectionBuilder::new().build().await?;
//! println!("{}", collection);
//! # Ok(())
//! # }
//! ```
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

mod api;
pub mod artworks;
mod common;
mod config;

pub use api::Api;
pub use api::fetch;

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
