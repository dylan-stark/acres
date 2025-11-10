#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! This library powers the [`acres-cli`] and [`acres-tui`].
//!
//! We currently have support for the endpoints in the [Artworks collection].
//! You can create requests for endpoints and use the built-in [API helper] and [fetch function] to retrieve resources.
//!
//! For instance, you can [get artwork by id].
//!
//! ```rust
//! # use serde_json::json;
//! # use anyhow::Result;
//! use acres::artworks::request::artwork;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let body = json!(
//! # {
//! #     "data": {
//! #         "id": 4,
//! #         "api_model": "artworks",
//! #         "api_link": "https://api.artic.edu/api/v1/artworks/4",
//! #         "is_boosted": false,
//! #         "title": "Priest and Boy",
//! #         "alt_titles": null,
//! #     },
//! #     "info": {
//! #         "license_text": "The `description` field in this response is licensed under a Creative Commons Attribution 4.0 Generic License (CC-By) and the Terms and Conditions of artic.edu. All other data in this response is licensed under a Creative Commons Zero (CC0) 1.0 designation and the Terms and Conditions of artic.edu.",
//! #         "license_links": [
//! #             "https://creativecommons.org/publicdomain/zero/1.0/",
//! #             "https://www.artic.edu/terms"
//! #         ],
//! #         "version": "1.13"
//! #     },
//! #     "config": {
//! #         "iiif_url": "https://www.artic.edu/iiif/2",
//! #         "website_url": "https://www.artic.edu"
//! #     }
//! # }
//! # );
//!
//! # let mock_server = wiremock::MockServer::start().await;
//! # let mock_uri = format!("{}/api/v1", mock_server.uri());
//! # wiremock::Mock::given(wiremock::matchers::any())
//! #     .and(wiremock::matchers::path(format!("/api/v1/artworks/{}", 4)))
//! #     .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
//! #     .expect(1)
//! #     .mount(&mock_server)
//! #     .await;
//!
//! let api = acres::Api::new();
//! # let api = acres::Api::builder().base_uri(&mock_uri).use_cache(false).build();
//!
//! let request = artwork::Request::new(api.base_uri(), 4);
//! let json: acres::Cached = api.fetch(request.to_string(), None as Option<usize>).await?;
//! # Ok(())
//! # }
//! ```
//!
//! We provide [builders] for most everything. Where possible, sensible defaults are used so that
//! you only need provide specific details for your use case.
//!
//! [Artworks collection]: https://api.artic.edu/docs/#artworks
//! [API helper]: struct.Api.html
//! [fetch function]: fn.fetch.html
//! [get artwork by id]: https://api.artic.edu/docs/#get-artworks-id
//! [public APIs]: https://api.artic.edu/docs/#introduction
//! [`acres-cli`]: ../acres_cli/index.html
//! [`acres-tui`]: ../acres_tui/index.html
//! [builders]: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html

mod api;
pub mod artworks;
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
