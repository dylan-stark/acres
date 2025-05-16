use crate::artworks::ArtworksCollection;

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
    use serde_json::Value;
    use wiremock::matchers::{any, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::artworks::List;
    use crate::common;

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
    async fn api_artworks_list() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_list = common::tests::list_with_numero_uno();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_list.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let list: List = api.artworks().list().get().await.unwrap();

        assert_eq!(list.to_string(), mock_list.to_string());
    }

    #[tokio::test]
    async fn api_artworks_list_by_ids() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_list = common::tests::list_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("ids", "1,3"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_list.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let list: List = api.artworks().list().ids(vec![1, 3]).get().await.unwrap();

        assert_eq!(list.to_string(), mock_list.to_string());
    }

    #[tokio::test]
    async fn api_artworks_list_with_limit() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_list = common::tests::list_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("limit", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_list.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let list: List = api.artworks().list().limit(2).get().await.unwrap();

        assert_eq!(list.to_string(), mock_list.to_string());
    }

    #[tokio::test]
    async fn error_from_api_artworks_list_with_limit() {
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
    async fn api_artworks_list_with_page() {
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
    async fn api_artworks_list_with_fields() {
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
    async fn api_artworks_list_with_include() {
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
