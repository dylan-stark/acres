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
//! let api = aic::Api::new();
//! # let api = aic::Api::builder().base_uri(&mock_uri).use_cache(false).build();
//! let artworks_listing = api.artworks().list().get().await?;
//! println!("{}", artworks_listing);
//! # Ok(())
//! # }
//! ```
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

mod artworks;
mod config;

use anyhow::{Context, anyhow};
use reqwest::StatusCode;
use serde::Serialize;
use serde::ser::SerializeSeq;

pub use crate::artworks::ArtworksListing;
use crate::config::Config;

/// An AIC error.
#[derive(Debug, thiserror::Error)]
pub enum AicError {
    /// An unexpected error.
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

struct ArtworksListingQueryParams {
    ids: Option<Vec<u32>>,
    limit: Option<u32>,
    page: Option<u32>,
    fields: Vec<String>,
    include: Vec<String>,
}

impl Serialize for ArtworksListingQueryParams {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if let Some(ids) = &self.ids {
            let ids_string = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            seq.serialize_element(&("ids", ids_string))?
        }
        if let Some(limit) = &self.limit {
            seq.serialize_element(&("limit", limit))?
        }
        if let Some(page) = &self.page {
            seq.serialize_element(&("page", page))?
        }
        if !self.fields.is_empty() {
            seq.serialize_element(&("fields", self.fields.join(",")))?;
        }
        if !self.include.is_empty() {
            seq.serialize_element(&("include", self.include.join(",")))?;
        }
        seq.end()
    }
}

/// An artworks collection listing.
///
/// This corresonds to the [`GET /artworks`] endpoint on the public API.
///
/// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
#[derive(Clone, Debug, Default)]
pub struct ArtworksCollectionListing {
    api: Api,
    ids: Option<Vec<u32>>,
    limit: Option<u32>,
    page: Option<u32>,
    fields: Vec<String>,
    include: Vec<String>,
}

impl ArtworksCollectionListing {
    /// Sets the artwork ids to retrieve.
    ///
    /// # Examples
    ///
    /// ```
    /// let listing = aic::Api::new().artworks().list().ids(vec![256, 1024, 4096]);
    /// ```
    pub fn ids(mut self, ids: Vec<u32>) -> Self {
        self.ids = Some(ids);
        self
    }

    /// Sets limit on number of artworks to return per page.
    ///
    /// See [pagination section] for additional information on valid settings
    /// for `limit` and interactions with related options.
    ///
    /// # Examples
    ///
    /// ```
    /// let listing = aic::Api::new().artworks().list().limit(10);
    /// ```
    ///
    /// [pagination section]: https://api.artic.edu/docs/#pagination
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets page number to return.
    ///
    /// See [pagination section] for additional information on valid settings
    /// for `page` and interactions with related options.
    ///
    /// # Examples
    ///
    /// ```
    /// let listing = aic::Api::new().artworks().list().page(2);
    /// ```
    ///
    /// [pagination section]: https://api.artic.edu/docs/#pagination
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the artwork fields to retrieve.
    ///
    /// # Examples
    ///
    /// ```
    /// let listing = aic::Api::new().artworks().list().fields(vec!["title".into(), "description".into()]);
    /// ```
    pub fn fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }

    /// Sets the sub-resources to include.
    ///
    /// # Examples
    ///
    /// ```
    /// let listing = aic::Api::new().artworks().list().include(vec!["place_pivots".into()]);
    /// ```
    pub fn include(mut self, include: Vec<String>) -> Self {
        self.include = include;
        self
    }

    /// Pulls a listing of all artworks.
    ///
    /// # Examples
    ///
    /// ```
    /// # use anyhow::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let mock_server = wiremock::MockServer::start().await;
    /// # let mock_uri = format!("{}/api/v1", mock_server.uri());
    /// # wiremock::Mock::given(wiremock::matchers::any())
    /// #     .and(wiremock::matchers::path("/api/v1/artworks"))
    /// #     .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("{}"))
    /// #     .expect(1)
    /// #     .mount(&mock_server)
    /// #     .await;
    /// # let api = aic::Api::builder().base_uri(&mock_uri).use_cache(false).build();
    /// let listing = api.artworks().list().get().await?;
    /// println!("{}", listing);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self) -> Result<ArtworksListing, AicError> {
        // TODO: Move config into `Api`
        let config = Config::new().context("failed to load config")?;
        let artworks_json_path = config.cache_dir.join("artworks.json");
        if config.use_cache && self.api.use_cache && artworks_json_path.is_file() {
            let json = std::fs::read_to_string(&artworks_json_path).with_context(|| {
                format!(
                    "failed to read cached file from {}",
                    artworks_json_path.display()
                )
            })?;
            Ok(ArtworksListing::new(
                serde_json::from_str(&json).context("failed to serialie JSON")?,
            ))
        } else {
            let artworks_path = format!("{}/artworks", self.api.base_uri);
            let client = reqwest::Client::new();
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "user-agent",
                format!("ACRES/{}", env!("CARGO_PKG_VERSION"),)
                    .parse()
                    .context("failed constructing user-agent header")?,
            );
            headers.insert(
                "ACRES-User-Agent",
                "ACRES (dylan.stark@gmail.com)"
                    .parse()
                    .context("failed constructing ACRES-User-Agent header")?,
            );
            let query_params = ArtworksListingQueryParams {
                ids: self.ids.clone(),
                limit: self.limit,
                page: self.page,
                fields: self.fields.clone(),
                include: self.include.clone(),
            };

            let response = client
                .get(&artworks_path)
                .headers(headers)
                .query(&query_params)
                .send()
                .await
                .with_context(|| format!("failed to GET {}", artworks_path))?;
            let listing = match response.status() {
                StatusCode::OK => Ok(response
                    .json::<serde_json::Value>()
                    .await
                    .with_context(|| format!("failed to get JSON from GET {}", artworks_path))?),
                _ => Err(response
                    .json::<serde_json::Value>()
                    .await
                    .map(|value| anyhow!("{}: {}", value["error"], value["detail"]))
                    .with_context(|| format!("failed to get JSON from GET {}", artworks_path))?),
            };

            if let Ok(listing) = &listing {
                std::fs::create_dir_all(artworks_json_path.parent().expect("path has parent"))
                    .with_context(|| {
                        format!(
                            "failed to create parent directory for {}",
                            artworks_json_path.display()
                        )
                    })?;
                std::fs::write(&artworks_json_path, listing.to_string())
                    .with_context(|| format!("failed to write {}", artworks_json_path.display()))?;
            }

            match listing {
                Ok(listing) => Ok(ArtworksListing::new(listing)),
                Err(error) => Err(error.into()),
            }
        }
    }
}

/// The [artworks collection].
///
/// [artworks collection]: https://api.artic.edu/docs/#artworks
#[derive(Clone, Debug, Default)]
pub struct ArtworksCollection {
    api: Api,
}

impl ArtworksCollection {
    /// Returns an artworks collection listing.
    pub fn list(&self) -> ArtworksCollectionListing {
        ArtworksCollectionListing {
            ids: None,
            limit: None,
            page: None,
            fields: vec!["id".into(), "title".into()],
            include: vec![],
            api: Api {
                base_uri: self.api.base_uri.clone(),
                use_cache: self.api.use_cache,
            },
        }
    }
}

/// The top-level API client.
///
/// All access to the [AIC public APIs] goes through one of these.
///
/// # Examples
///
/// ```
/// let api = aic::Api::new();
/// ```
///
/// [AIC public APIs]: https://api.artic.edu/docs/#introduction
#[derive(Clone, Debug)]
pub struct Api {
    base_uri: String,
    use_cache: bool,
}

impl Api {
    /// Creates a new instance of the API client.
    ///
    /// # Examples
    ///
    /// ```
    /// let api = aic::Api::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an API client builder.
    ///
    /// This is useful for tailoring client behavior. See the [`ApiBuilder`]
    /// struct for more information.
    ///
    /// # Examples
    ///
    /// Caching is on by default but you can disable it with
    ///
    /// ```
    /// let api = aic::Api::builder()
    ///     .use_cache(false)
    ///     .build();
    /// assert!(!api.use_cache());
    /// ```
    ///
    /// [`ApiBuilder`]: ./struct.ApiBuilder.html
    pub fn builder() -> ApiBuilder {
        ApiBuilder::default()
    }

    /// Returns the base URI.
    ///
    /// # Examples
    ///
    /// The default base URI is `https://api.artic.edu/api/v1`.
    ///
    /// ```
    /// let api = aic::Api::new();
    /// assert_eq!(api.base_uri(), "https://api.artic.edu/api/v1");
    /// ```
    pub fn base_uri(&self) -> String {
        self.base_uri.to_string()
    }

    /// Returns whether or not caching is enabled or disabled for the API client.
    ///
    /// # Examples
    ///
    /// Caching is on by default.
    ///
    /// ```
    /// let api = aic::Api::new();
    /// assert!(api.use_cache());
    /// ```
    pub fn use_cache(&self) -> bool {
        self.use_cache
    }

    /// Returns an artworks collection.
    pub fn artworks(&self) -> ArtworksCollection {
        ArtworksCollection {
            api: Api {
                base_uri: self.base_uri.clone(),
                use_cache: self.use_cache,
            },
        }
    }
}

impl Default for Api {
    fn default() -> Self {
        ApiBuilder::default().build()
    }
}

/// An API client builder.
///
/// Use one of these to tailor the client; e.g., to disable caching:
///
/// ```
/// let api = aic::Api::builder()
///     .use_cache(false)
///     .build();
/// assert!(!api.use_cache());
/// ```
#[derive(Clone, Debug)]
pub struct ApiBuilder {
    base_uri: String,
    use_cache: bool,
}

impl ApiBuilder {
    /// Changes the base URI.
    ///
    /// This default is `https://api.artic.edu/api/v1`. If you need to change it for some reason,
    /// you can with
    ///
    /// ```
    /// let api = aic::Api::builder()
    ///     .base_uri("https://127.0.0.1:8443/api/v1")
    ///     .build();
    /// assert_eq!(api.base_uri(), "https://127.0.0.1:8443/api/v1");
    /// ```
    pub fn base_uri(mut self, base_uri: &str) -> Self {
        self.base_uri = base_uri.to_string();
        self
    }

    /// Sets whether or not to cache the response.
    ///
    /// The default is to always cache, but you can turn off caching with
    ///
    /// ```
    /// let api = aic::Api::builder()
    ///     .use_cache(false)
    ///     .build();
    /// assert!(!api.use_cache());
    /// ```
    pub fn use_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    /// Builds the actual API client.
    pub fn build(self) -> Api {
        Api {
            base_uri: self.base_uri,
            use_cache: self.use_cache,
        }
    }
}

impl Default for ApiBuilder {
    fn default() -> Self {
        ApiBuilder {
            base_uri: String::from("https://api.artic.edu/api/v1"),
            use_cache: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use wiremock::matchers::{any, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::artworks::tests::*;

    #[test]
    fn base_uri_default() {
        assert_eq!(Api::new().base_uri(), "https://api.artic.edu/api/v1");
    }

    #[test]
    fn use_cache_by_default() {
        assert!(Api::new().use_cache());
    }

    #[test]
    fn custom_base_uri() {
        let custom_uri = "http://localhost:80/api/v1";
        let api = Api::builder().base_uri(custom_uri).build();
        assert_eq!(api.base_uri, custom_uri);
    }

    #[tokio::test]
    async fn api_artworks_listing() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_listing = listing_with_numero_uno();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_listing.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let listing: ArtworksListing = api.artworks().list().get().await.unwrap();

        assert_eq!(listing.to_string(), mock_listing.to_string());
    }

    #[tokio::test]
    async fn api_artworks_listing_by_ids() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_listing = listing_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("ids", "1,3"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_listing.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let listing: ArtworksListing = api.artworks().list().ids(vec![1, 3]).get().await.unwrap();

        assert_eq!(listing.to_string(), mock_listing.to_string());
    }

    #[tokio::test]
    async fn api_artworks_listing_with_limit() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_listing = listing_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("limit", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_listing.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let listing: ArtworksListing = api.artworks().list().limit(2).get().await.unwrap();

        assert_eq!(listing.to_string(), mock_listing.to_string());
    }

    #[tokio::test]
    async fn error_from_api_artworks_listing_with_limit() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_error: Value =
            serde_json::from_str(r#"{"status":403,"error":"Invalid limit","detail":"You have requested too many resources per page. Please set a smaller limit."}"#).unwrap();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("limit", "1000"))
            .respond_with(ResponseTemplate::new(403).set_body_json(&mock_error))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let error = api.artworks().list().limit(1000).get().await.err().unwrap();

        assert_eq!(
            error.to_string(),
            format!(
                "{}: {}",
                mock_error.get("error").unwrap(),
                mock_error.get("detail").unwrap()
            )
        );
    }

    #[tokio::test]
    async fn api_artworks_listing_with_page() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        api.artworks().list().page(2).get().await.unwrap();

        // Then page query param is set to 2, as asserted by the mock
    }

    #[tokio::test]
    async fn api_artworks_listing_with_fields() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("fields", "title,description"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        api.artworks()
            .list()
            .fields(vec!["title".into(), "description".into()])
            .get()
            .await
            .unwrap();

        // Then fields query param is set to "title,description", as asserted by the mock
    }

    #[tokio::test]
    async fn api_artworks_listing_with_include() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("include", "date,place_pivots"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        api.artworks()
            .list()
            .include(vec!["date".into(), "place_pivots".into()])
            .get()
            .await
            .unwrap();

        // Then include query param is set to "date,place_pivots", as asserted by the mock
    }
}
