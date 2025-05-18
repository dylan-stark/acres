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
        ApiBuilder {
            base_uri: String::from("https://api.artic.edu/api/v1"),
            use_cache: true,
        }
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
