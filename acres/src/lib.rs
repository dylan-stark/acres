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

/// An Acres error.
#[derive(Debug, thiserror::Error)]
pub enum AcresError {
    /// An artwork-related error
    #[error("Artwork-related error: {0}")]
    ArtworkError(String),
    /// An IIIF-related error
    #[error("IIIF-related error: {0}")]
    IiifError(String),
    /// A search query parameter error
    #[error("Search query parameters error: {0}")]
    SearchQueryParamsError(String),
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
