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
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
