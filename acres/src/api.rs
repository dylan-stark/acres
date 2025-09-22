use std::fmt::Debug;

use crate::{AcresError, common::FromBytes, config::Config};
use anyhow::{Context, anyhow};
use bytes::Bytes;
use reqwest::StatusCode;
use serde::Serialize;

/// The top-level API client.
///
/// All access to the [AIC public APIs] goes through one of these.
///
/// # Examples
///
/// ```
/// let api = acres::Api::new();
/// ```
///
/// [AIC public APIs]: https://api.artic.edu/docs/#introduction
#[derive(Clone, Debug, PartialEq)]
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
    /// Fetch
    pub async fn fetch<T>(
        &self,
        endpoint: String,
        query_params: Option<impl Serialize + Debug>,
    ) -> Result<T, AcresError>
    where
        T: FromBytes<T>,
    {
        let cached: Option<Bytes> = self.from_cache(&endpoint, &query_params)?;
        let results: Result<Bytes, AcresError> = match cached {
            Some(results) => Ok(results),
            None => Ok(fetch(&endpoint, &query_params).await?),
        };
        let results = self.to_cache(&endpoint, query_params, results.unwrap())?;
        T::from_bytes(results)
    }

    /// Stores an item in cache.
    pub fn to_cache(
        &self,
        endpoint: &String,
        query_params: impl Debug,
        data: Bytes,
    ) -> Result<Bytes, AcresError> {
        if !self.use_cache {
            return Ok(data);
        }
        let id =
            xxhash_rust::xxh3::xxh3_64(format!("{:?}-{:?}", endpoint, query_params).as_bytes())
                .to_string();
        tracing::debug!("Looking to store id '{}' to cache", &id);
        let cache_file_path = match Config::new() {
            Ok(config) => config.cache_dir.join(&id),
            Err(_) => return Ok(data),
        };
        tracing::debug!(?cache_file_path);
        if cache_file_path.is_file() {
            return Ok(data);
        }
        std::fs::write(&cache_file_path, data.clone()).with_context(|| "writing data to file")?;
        tracing::info!(
            "Wrote '{}' to cache at '{}'",
            id,
            cache_file_path.to_str().unwrap_or("???")
        );
        Ok(data)
    }

    /// Loads an item from cache.
    ///
    /// If `id` is not in the cache, returns Ok(None). Otherwise, loads
    /// the data and returns result of applying the provided closure f().
    pub fn from_cache(
        &self,
        endpoint: &String,
        query_params: &Option<impl Serialize + Debug>,
    ) -> Result<Option<Bytes>, AcresError> {
        if !self.use_cache {
            return Ok(None);
        }
        let id =
            xxhash_rust::xxh3::xxh3_64(format!("{:?}-{:?}", endpoint, query_params).as_bytes())
                .to_string();
        tracing::debug!("Looking to load id '{}' from cache", id);
        let cache_file_path = match Config::new() {
            Ok(config) => config.cache_dir.join(&id),
            Err(_) => return Ok(None),
        };
        if !cache_file_path.is_file() {
            return Ok(None);
        }
        let data =
            std::fs::read(&cache_file_path).with_context(|| "failed to read cached file from")?;
        tracing::info!(
            "Loaded '{}' from cache at '{}'",
            id,
            cache_file_path.to_str().unwrap_or("???")
        );
        Ok(Some(data.into()))
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

async fn fetch(
    endpoint: &String,
    query_params: &Option<impl Serialize>,
) -> Result<Bytes, AcresError> {
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
    let mut request = client.get(endpoint).headers(headers);
    if query_params.is_some() {
        request = request.query(&query_params);
    }
    let response = request
        .send()
        .await
        .with_context(|| format!("GET {}", endpoint))?;
    let value = match response.status() {
        StatusCode::OK => Ok(response
            .bytes()
            //.json::<serde_json::Value>()
            .await
            .with_context(|| format!("awaiting JSON from GET {}", endpoint))?),
        _ => Err(response
            .json::<serde_json::Value>()
            .await
            .map(|value| anyhow!("{}: {}", value["error"], value["detail"]))
            .with_context(|| format!("awaiting errror from GET {}", endpoint))?),
    };
    Ok(value?)
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
