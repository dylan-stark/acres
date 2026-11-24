#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! This library powers the [`acres-cli`] and [`acres-tui`].
//!
//! We currently have support for the endpoints in the [Artworks collection].
//! You build a request for an endpoint and fetch it with a shared [`Catalog`], which rate-limits,
//! retries and caches (see ADR 0016).
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
//! # let mock_server = wiremock::MockServer::start().await;
//! # let mock_uri = format!("{}/api/v1", mock_server.uri());
//! # wiremock::Mock::given(wiremock::matchers::any())
//! #     .and(wiremock::matchers::path(format!("/api/v1/artworks/{}", 4)))
//! #     .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
//! #     .expect(1)
//! #     .mount(&mock_server)
//! #     .await;
//! let catalog = acres::Catalog::builder().build()?;
//! # let catalog = acres::Catalog::builder().base_uri(&mock_uri).no_cache().build()?;
//!
//! let request = artwork::Request::new(catalog.base_uri(), 4);
//! let json = catalog.raw(request, acres::Resource::Record).await?.value;
//! # Ok(())
//! # }
//! ```
//!
//! We provide [builders] for endpoints with more options. Where possible, sensible defaults are used so that
//! you only need provide specific details for your use case. For example, you can [list artworks]
//! while limiting the number of results and choosing which fields to return.
//!
//! ```rust
//! # use serde_json::json;
//! # use anyhow::Result;
//! use acres::artworks::request::artworks;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! # let body = json!(
//! # {
//! #    "pagination": {
//! #        "total": 116941,
//! #        "limit": 2,
//! #        "offset": 0,
//! #        "total_pages": 58471,
//! #        "current_page": 1,
//! #        "next_url": "https://api.artic.edu/api/v1/artworks?page=2&limit=2"
//! #    },
//! #    "data": [
//! #        {
//! #            "id": 14556,
//! #            "api_link": "https://api.artic.edu/api/v1/artworks/14556",
//! #            "title": "Auvers, Panoramic View",
//! #        },
//! #        {
//! #            "id": 8961,
//! #            "api_link": "https://api.artic.edu/api/v1/artworks/8961",
//! #            "title": "Head of Arthur Jerome Eddy",
//! #        }
//! #    ],
//! #    "info": {
//! #        "license_text": "The `description` field in this response is licensed under a Creative Commons Attribution 4.0 Generic License (CC-By) and the Terms and Conditions of artic.edu. All other data in this response is licensed under a Creative Commons Zero (CC0) 1.0 designation and the Terms and Conditions of artic.edu.",
//! #        "license_links": [
//! #            "https://creativecommons.org/publicdomain/zero/1.0/",
//! #            "https://www.artic.edu/terms"
//! #        ],
//! #        "version": "1.13"
//! #    },
//! #    "config": {
//! #        "iiif_url": "https://www.artic.edu/iiif/2",
//! #        "website_url": "https://www.artic.edu"
//! #    }
//! # }
//! # );
//! # let mock_server = wiremock::MockServer::start().await;
//! # let mock_uri = format!("{}/api/v1", mock_server.uri());
//! # wiremock::Mock::given(wiremock::matchers::any())
//! #     .and(wiremock::matchers::path("/api/v1/artworks"))
//! #     .and(wiremock::matchers::query_param("limit", "2"))
//! #     .and(wiremock::matchers::query_param("fields", "id,title,api_link"))
//! #     .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(body))
//! #     .expect(1)
//! #     .mount(&mock_server)
//! #     .await;
//! let catalog = acres::Catalog::builder().build()?;
//! # let catalog = acres::Catalog::builder().base_uri(&mock_uri).no_cache().build()?;
//!
//! let request = artworks::Request::builder()
//!     .base_uri(catalog.base_uri())
//!     .limit(Some(2))
//!     .fields(Some(vec!["id".into(), "title".into(), "api_link".into()]))
//!     .build()?;
//! let json = catalog.raw(request, acres::Resource::Page).await?.value;
//! # Ok(())
//! # }
//! ```
//!
//! We currently support the following endpoints under the Artworks collection:
//!
//! - [`GET /artworks`] via [`acres::artworks::request::artworks`]
//! - [`GET /artworks/search`] via [`acres::artworks::request::search`]
//! - [`GET /artworks/{id}`] via [`acres::artworks::request::artwork`]
//! - [`GET /artworks/{id}/manifest.json`] via [`acres::artworks::request::manifest`]
//!
//! [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
//! [`acres::artworks::request::artworks`]: artworks/request/artworks/index.html
//! [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search
//! [`acres::artworks::request::search`]: artworks/request/search/index.html
//! [`GET /artworks/{id}`]: https://api.artic.edu/docs/#get-artworks-id
//! [`acres::artworks::request::artwork`]: artworks/request/artwork/index.html
//! [`GET /artworks/{id}/manifest.json`]: https://api.artic.edu/docs/#get-artworks-id-manifest-json
//! [`acres::artworks::request::manifest`]: artworks/request/manifest/index.html
//! [Artworks collection]: https://api.artic.edu/docs/#artworks
//! [get artwork by id]: https://api.artic.edu/docs/#get-artworks-id
//! [list artworks]: https://api.artic.edu/docs/#get-artworks
//! [public APIs]: https://api.artic.edu/docs/#introduction
//! [`acres-cli`]: ../acres_cli/index.html
//! [`acres-tui`]: ../acres_tui/index.html
//! [builders]: https://rust-unofficial.github.io/patterns/patterns/creational/builder.html

pub mod artworks;
mod catalog;
mod config;
mod error;
mod model;

pub use catalog::{Catalog, CatalogBuilder, Fetched, Lane, Resource};
pub use config::DEFAULT_BASE_URI;
pub use error::{BoxError, CacheError, Error, ParseError, RequestError, ResponseError};
pub use model::{ApiConfig, Item, Page, Pagination};
