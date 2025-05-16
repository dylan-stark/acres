#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! Create an API client and list artworks with
//!
//! ```
//! # use anyhow::Result;
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let mock_server = wiremock::MockServer::start().await;
//! # let mock_uri = format!("{}/api/v1", mock_server.uri());
//! # wiremock::Mock::given(wiremock::matchers::any())
//! #     .and(wiremock::matchers::path("/api/v1/artworks"))
//! #     .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("{}"))
//! #     .expect(1)
//! #     .mount(&mock_server)
//! #     .await;
//! let api = acres::Api::new();
//! # let api = acres::Api::builder().base_uri(&mock_uri).use_cache(false).build();
//! let artworks_list = api.artworks().list().get().await?;
//! println!("{}", artworks_list);
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
