use std::fmt::{Debug, Display};

use crate::{CacheError, Error, ParseError, ResponseError, catalog::Cache, config::Config};
use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Cached(serde_json::Value);

impl Display for Cached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl TryFrom<Bytes> for Cached {
    type Error = Error;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        let reader = value.reader();
        serde_json::from_reader(reader).map_err(|e| ParseError::body("response", e).into())
    }
}

/// A helper for fetching resources form the [AIC public APIs].
///
/// Responses are cached by default, but this behavior can be turned off.
///
/// ```rust
/// let api_sans_caching = acres::Api::builder().use_cache(false).build();
/// ```
///
/// [AIC public APIs]: https://api.artic.edu/docs/#introduction
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Api {
    pub(crate) base_uri: String,
    pub(crate) use_cache: bool,
}

impl Api {
    /// Creates a new instance of the API client.
    ///
    /// # Examples
    ///
    /// ```
    /// let api = acres::Api::new();
    /// ```
    pub fn new() -> Self {
        tracing::info!(msg = "Creating new API.");
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
    /// let api = acres::Api::builder()
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
    /// let api = acres::Api::new();
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
    /// let api = acres::Api::new();
    /// assert!(api.use_cache());
    /// ```
    pub fn use_cache(&self) -> bool {
        self.use_cache
    }
}

impl Api {
    /// Fetches `endpoint`, from the cache when it holds it, and converts the body to `T`.
    pub async fn fetch<T>(&self, endpoint: String) -> Result<T, Error>
    where
        T: TryFrom<Bytes>,
        T::Error: std::error::Error + Send + Sync + 'static,
    {
        let cached: Option<Bytes> = self.load_from_cache(&endpoint)?;
        let results: Bytes = match cached {
            Some(results) => results,
            None => {
                let results = fetch(&endpoint).await?;
                self.store_in_cache(&endpoint, results.clone())?;
                results
            }
        };
        T::try_from(results).map_err(|e| ParseError::body(endpoint, e).into())
    }

    fn cache(&self) -> Option<Cache> {
        if !self.use_cache {
            return None;
        }
        Config::new()
            .ok()
            .map(|config| Cache::new(Some(config.cache_dir), false))
    }

    /// Stores an item in cache.
    pub fn store_in_cache(&self, endpoint: &str, data: Bytes) -> Result<Bytes, CacheError> {
        if let Some(cache) = self.cache() {
            cache.store(endpoint, &data)?;
        }
        Ok(data)
    }

    /// Loads an item from cache.
    pub fn load_from_cache(&self, endpoint: &str) -> Result<Option<Bytes>, CacheError> {
        match self.cache() {
            Some(cache) => cache.load(endpoint, None),
            None => Ok(None),
        }
    }
}

impl Display for Api {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "API {{ base_uri: {:?}, use_cache: {:?} }}",
            self.base_uri, self.use_cache
        ))
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
/// let api = acres::Api::builder()
///     .use_cache(false)
///     .build();
/// assert!(!api.use_cache());
/// ```
#[derive(Clone, Debug, PartialEq)]
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
    /// let api = acres::Api::builder()
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
    /// let api = acres::Api::builder()
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
        // TODO: Check ACRES_* for overrides
        let config = Config::new().unwrap_or_default();
        ApiBuilder {
            base_uri: config.base_uri,
            use_cache: config.use_cache,
        }
    }
}

/// Helper for fetching resources.
///
/// This method does not implement response caching. Use [`Api::fetch()`] fetch
/// with caching.
///
/// [`Api::fetch()`]: struct.Api.html#method.fetch
pub async fn fetch(endpoint: &String) -> Result<Bytes, Error> {
    let transport = |e: reqwest::Error| ResponseError::Transport {
        endpoint: endpoint.clone(),
        source: e.into(),
    };
    let contact = Config::new().ok().and_then(|config| config.contact);
    let client = crate::catalog::http_client(contact.as_deref())?;
    let response = client.get(endpoint).send().await.map_err(transport)?;
    let status = response.status();
    let body = response.bytes().await.map_err(transport)?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(crate::catalog::status_error(endpoint, status, &body).into())
    }
}

#[cfg(test)]
mod tests {
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
}
