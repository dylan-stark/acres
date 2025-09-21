use anyhow::{Context, anyhow};
use reqwest::StatusCode;

use crate::artworks::collection_query_params::CollectionQueryParams;
use crate::{AcresError, api::Api, artworks::Collection, config::Config};

/// An artworks collection collection operation.
///
/// This corresonds to the [`GET /artworks`] endpoint on the public API.
///
/// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionBuilder {
    api: Api,
    ids: Option<Vec<u32>>,
    limit: Option<u32>,
    page: Option<u32>,
    fields: Vec<String>,
    include: Vec<String>,
}

impl CollectionBuilder {
    /// Creates a new collection builder.
    pub fn new() -> Self {
        CollectionBuilder::default()
    }

    /// Sets API.
    ///
    /// Use this when you want to directly construct the collection operation, but also want customize
    /// the API.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::Api;
    /// use acres::artworks::CollectionBuilder;
    ///
    /// let api = Api::builder().use_cache(false).build();
    /// CollectionBuilder::new().api(api);
    /// ```
    pub fn api(mut self, api: Api) -> Self {
        self.api = api;
        self
    }

    /// Sets the artwork ids to retrieve.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::CollectionBuilder;
    ///
    /// CollectionBuilder::new().ids(Some(vec![256, 1024, 4096]));
    /// ```
    pub fn ids(mut self, ids: Option<Vec<u32>>) -> Self {
        tracing::info!(msg = "Settings ids", ?ids);
        self.ids = ids;
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
    /// use acres::artworks::CollectionBuilder;
    ///
    /// CollectionBuilder::new().limit(Some(10));
    /// ```
    ///
    /// [pagination section]: https://api.artic.edu/docs/#pagination
    pub fn limit(mut self, limit: Option<u32>) -> Self {
        tracing::info!(msg = "Settings limit", limit);
        self.limit = limit;
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
    /// use acres::artworks::CollectionBuilder;
    ///
    /// CollectionBuilder::new().page(Some(2));
    /// ```
    ///
    /// [pagination section]: https://api.artic.edu/docs/#pagination
    pub fn page(mut self, page: Option<u32>) -> Self {
        tracing::info!(msg = "Settings page", page);
        self.page = page;
        self
    }

    /// Sets the artwork fields to retrieve.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::CollectionBuilder;
    ///
    /// CollectionBuilder::new().fields(Some(vec!["title".into(), "description".into()]));
    /// ```
    pub fn fields(mut self, fields: Option<Vec<String>>) -> Self {
        tracing::info!(msg = "Settings fields", ?fields);
        if let Some(fields) = fields {
            self.fields = fields;
        }
        self
    }

    /// Sets the sub-resources to include.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::CollectionBuilder;
    ///
    /// CollectionBuilder::new().include(Some(vec!["place_pivots".into()]));
    /// ```
    pub fn include(mut self, include: Option<Vec<String>>) -> Self {
        tracing::info!(msg = "Settings include", ?include);
        if let Some(include) = include {
            self.include = include;
        }
        self
    }

    /// Builds artworks collection.
    ///
    /// This will fetch backing data from the AIC API.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::CollectionBuilder;
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
    /// let builder = CollectionBuilder::new();
    /// # let api = acres::Api::builder().base_uri(&mock_uri).use_cache(false).build();
    /// # let builder = CollectionBuilder::new().api(api);
    /// let collection = builder.build().await?;
    /// println!("{}", collection);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(&self) -> Result<Collection, AcresError> {
        tracing::info!(msg = "Getting artworks collection", ?self);
        // TODO: Move config into `Api`
        let config = Config::new().context("failed to load config")?;
        let artworks_json_path = config.cache_dir.join("artworks.json");
        if config.use_cache && self.api.use_cache && artworks_json_path.is_file() {
            tracing::info!(msg = "Using cached file", ?artworks_json_path);
            let json = std::fs::read_to_string(&artworks_json_path).with_context(|| {
                format!(
                    "failed to read cached file from {}",
                    artworks_json_path.display()
                )
            })?;
            Ok(Collection::new(
                serde_json::from_str(&json).context("failed to serialie JSON")?,
            ))
        } else {
            tracing::info!(msg = "Not using cache");
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
            tracing::debug!(?headers);
            let query_params = CollectionQueryParams {
                ids: self.ids.clone(),
                limit: self.limit,
                page: self.page,
                fields: self.fields.clone(),
                include: self.include.clone(),
            };
            tracing::debug!(?query_params);

            let response = client
                .get(&artworks_path)
                .headers(headers)
                .query(&query_params)
                .send()
                .await
                .with_context(|| format!("failed to GET {}", artworks_path))?;
            let collection = match response.status() {
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

            if let Ok(collection) = &collection {
                std::fs::create_dir_all(artworks_json_path.parent().expect("path has parent"))
                    .with_context(|| {
                        format!(
                            "failed to create parent directory for {}",
                            artworks_json_path.display()
                        )
                    })?;
                std::fs::write(&artworks_json_path, collection.to_string())
                    .with_context(|| format!("failed to write {}", artworks_json_path.display()))?;
            }

            match collection {
                Ok(collection) => Ok(Collection::new(collection)),
                Err(error) => Err(error.into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use wiremock::matchers::{any, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;
    use crate::artworks::Collection;
    use crate::common;

    #[tokio::test]
    async fn api_artworks_collection() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_collection = common::tests::collection_with_numero_uno();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_collection.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let collection: Collection = CollectionBuilder::new().api(api).build().await.unwrap();

        assert_eq!(collection.to_string(), mock_collection.to_string());
    }

    #[tokio::test]
    async fn api_artworks_collection_by_ids() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_collection = common::tests::collection_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("ids", "1,3"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_collection.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let collection: Collection = CollectionBuilder::new()
            .api(api)
            .ids(Some(vec![1, 3]))
            .build()
            .await
            .unwrap();

        assert_eq!(collection.to_string(), mock_collection.to_string());
    }

    #[tokio::test]
    async fn api_artworks_collection_with_limit() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_collection = common::tests::collection_with_numeros_uno_and_tres();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .and(query_param("limit", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_collection.to_string()))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let collection: Collection = CollectionBuilder::new()
            .api(api)
            .limit(Some(2))
            .build()
            .await
            .unwrap();

        assert_eq!(collection.to_string(), mock_collection.to_string());
    }

    #[tokio::test]
    async fn error_from_api_artworks_collection_with_limit() {
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

        let error = CollectionBuilder::new()
            .api(api)
            .limit(Some(1000))
            .build()
            .await
            .err()
            .unwrap();

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
    async fn api_artworks_collection_with_page() {
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

        CollectionBuilder::new()
            .api(api)
            .page(Some(2))
            .build()
            .await
            .unwrap();

        // Then page query param is set to 2, as asserted by the mock
    }

    #[tokio::test]
    async fn api_artworks_collection_with_fields() {
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

        CollectionBuilder::new()
            .api(api)
            .fields(Some(vec!["title".into(), "description".into()]))
            .build()
            .await
            .unwrap();

        // Then fields query param is set to "title,description", as asserted by the mock
    }

    #[tokio::test]
    async fn api_artworks_collection_with_include() {
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

        CollectionBuilder::new()
            .api(api)
            .include(Some(vec!["date".into(), "place_pivots".into()]))
            .build()
            .await
            .unwrap();

        // Then include query param is set to "date,place_pivots", as asserted by the mock
    }
}
