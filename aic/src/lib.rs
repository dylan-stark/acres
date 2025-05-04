#![deny(missing_docs)]

//! Simple and ergonomic access to the Art Institute of Chicago's [public APIs].
//!
//! Create an API client and list artworks with
//!
//! ```
//! # use eyre::Result;
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let api = aic::Api::new();
//! let artworks_listing = api.artworks().await?;
//! println!("{}", artworks_listing);
//! # Ok(())
//! # }
//! ```
//!
//! [public APIs]: https://api.artic.edu/docs/#introduction

mod artworks;
mod config;

use eyre::{Context, Result};

use crate::artworks::ArtworksListing;
use crate::config::Config;

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

    /// Pulls a listing of all artworks.
    ///
    /// # Examples
    ///
    /// ```
    /// # use eyre::Result;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let api = aic::Api::new();
    /// let listing = api.artworks().await?;
    /// println!("{}", listing);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn artworks(&self) -> Result<ArtworksListing> {
        // TODO: Move config into `Api`
        let config = Config::new().wrap_err("failed to load config")?;
        let artworks_json_path = config.aic_cache_dir.join("artworks.json");
        if self.use_cache && artworks_json_path.is_file() {
            let json = std::fs::read_to_string(&artworks_json_path).wrap_err_with(|| {
                format!(
                    "failed to read cached file from {}",
                    artworks_json_path.display()
                )
            })?;
            Ok(serde_json::from_str(&json)?)
        } else {
            let artworks_path = format!("{}/artworks", self.base_uri);
            let client = reqwest::Client::new();
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "user-agent",
                format!("ACRES/{}", env!("CARGO_PKG_VERSION"),)
                    .parse()
                    .wrap_err("failed constructing user-agent header")?,
            );
            headers.insert(
                "ACRES-User-Agent",
                "ACRES (dylan.stark@gmail.com)"
                    .parse()
                    .wrap_err("failed constructing ACRES-User-Agent header")?,
            );

            let listing = client
                .get(&artworks_path)
                .headers(headers)
                .send()
                .await
                .wrap_err_with(|| format!("failed to GET {}", artworks_path))?
                .json::<ArtworksListing>()
                .await
                .wrap_err_with(|| format!("failed to get JSON from GET {}", artworks_path))?;
            std::fs::create_dir_all(artworks_json_path.parent().expect("path has parent"))
                .wrap_err_with(|| {
                    format!(
                        "failed to create parent directory for {}",
                        artworks_json_path.display()
                    )
                })?;
            // TODO: handle ?
            std::fs::write(&artworks_json_path, listing.to_string())
                .wrap_err_with(|| format!("failed to write {}", artworks_json_path.display()))?;
            Ok(listing)
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
    use wiremock::matchers::{any, path};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;

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
        let mock_listing: ArtworksListing =
            serde_json::from_str(r#"{ "data": [ { "id": 1, "title": "Numero uno" } ] }"#).unwrap();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_listing))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let listing: ArtworksListing = api.artworks().await.unwrap();

        assert_eq!(listing.data.len(), 1);
        assert_eq!(listing.data[0].id, 1);
        assert_eq!(listing.data[0].title, "Numero uno");
    }
}
