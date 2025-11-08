//! Artworks collections.

use std::fmt::Display;

use bytes::{Buf, Bytes};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};

use crate::{AcresError, api::Api};

/// A collection of artworks.
///
/// This is the response from the [`GET /artworks`].
///
/// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Collection(serde_json::Value);

impl Display for Collection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl From<Bytes> for Collection {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl Collection {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Collection(response)
    }

    /// Creates a new collection builder.
    ///
    /// Use the builder to configure the collection that you want to build.
    pub fn builder() -> Builder {
        Builder::default()
    }
}

/// An artworks collection collection operation.
///
/// This corresonds to the [`GET /artworks`] endpoint on the public API.
///
/// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Builder {
    api: Api,
    ids: Option<Vec<u32>>,
    limit: Option<u32>,
    page: Option<u32>,
    fields: Vec<String>,
    include: Vec<String>,
}

impl Builder {
    /// Creates a new collection builder.
    pub fn new() -> Self {
        Builder::default()
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// let api = Api::builder().use_cache(false).build();
    /// Builder::new().api(api);
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// Builder::new().ids(Some(vec![256, 1024, 4096]));
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// Builder::new().limit(Some(10));
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// Builder::new().page(Some(2));
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// Builder::new().fields(Some(vec!["title".into(), "description".into()]));
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
    /// use acres::artworks::request::artworks::Builder;
    ///
    /// Builder::new().include(Some(vec!["place_pivots".into()]));
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
    /// use acres::artworks::request::artworks::Builder;
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
    /// let builder = Builder::new();
    /// # let api = acres::Api::builder().base_uri(&mock_uri).use_cache(false).build();
    /// # let builder = Builder::new().api(api);
    /// let collection = builder.build().await?;
    /// println!("{}", collection);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(&self) -> Result<Collection, AcresError> {
        tracing::info!(msg = "Getting artworks collection", ?self);
        let endpoint = format!("{}/artworks", self.api.base_uri);
        let query_params = CollectionQueryParams {
            ids: self.ids.clone(),
            limit: self.limit,
            page: self.page,
            fields: self.fields.clone(),
            include: self.include.clone(),
        };
        self.api
            .fetch::<Collection>(endpoint, Some(query_params))
            .await
    }
}

#[derive(Debug)]
pub(super) struct CollectionQueryParams {
    pub(super) ids: Option<Vec<u32>>,
    pub(super) limit: Option<u32>,
    pub(super) page: Option<u32>,
    pub(super) fields: Vec<String>,
    pub(super) include: Vec<String>,
}

impl Serialize for CollectionQueryParams {
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
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common;
    use serde_json::Value;
    use wiremock::matchers::{any, path, query_param};
    use wiremock::{Mock, ResponseTemplate};

    #[test]
    fn artworks_collection_to_string() {
        let mock_collection = common::tests::collection_with_numero_uno();
        let _collection_string: String = mock_collection.to_string();
    }

    #[test]
    fn just_title_and_description_fields() {
        // Given response from server looks like this
        let json = r#"
{
  "pagination": {
    "total": 128194,
    "limit": 2,
    "offset": 0,
    "total_pages": 64097,
    "current_page": 1,
    "next_url": "https://api.artic.edu/api/v1/artworks?page=2&limit=2&fields=title%2Cdescription"
  },
  "data": [
    {
      "title": "Claude Monet",
      "description": null
    },
    {
      "title": "Skyphos (Drinking Cup)",
      "description": "\u003Cp\u003EDuring the course of the 5th and 4th centuries BCE, black vessels (commonly called black-glaze vessels) were made with increasing frequency in both Greece and South Italy. Many of them replicate the shape of metal vessels. Others have detailing that is molded or incised. While the quality of these vessels varies greatly, all would have been less expensive than vessels decorated in other contemporary techniques, for example, in red-figure.\u003C/p\u003E\n"
    }
  ],
  "info": {
    "license_text": "The `description` field in this response is licensed under a Creative Commons Attribution 4.0 Generic License (CC-By) and the Terms and Conditions of artic.edu. All other data in this response is licensed under a Creative Commons Zero (CC0) 1.0 designation and the Terms and Conditions of artic.edu.",
    "license_links": [
      "https://creativecommons.org/publicdomain/zero/1.0/",
      "https://www.artic.edu/terms"
    ],
    "version": "1.13"
  },
  "config": {
    "iiif_url": "https://www.artic.edu/iiif/2",
    "website_url": "http://www.artic.edu"
  }
}
            "#;

        // When we create a new artworks collection with it
        let json_value: serde_json::Value = serde_json::from_str(json).unwrap();
        let collection = Collection(json_value.clone());

        // Then the collection "looks like" what we got from the server
        let collection_value: serde_json::Value =
            serde_json::from_str(&format!("{}", collection)).unwrap();
        assert_eq!(collection_value, json_value);
    }

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

        let collection: Collection = Builder::new().api(api).build().await.unwrap();

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

        let collection: Collection = Builder::new()
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

        let collection: Collection = Builder::new()
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

        let error = Builder::new()
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

        Builder::new().api(api).page(Some(2)).build().await.unwrap();

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

        Builder::new()
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

        Builder::new()
            .api(api)
            .include(Some(vec!["date".into(), "place_pivots".into()]))
            .build()
            .await
            .unwrap();

        // Then include query param is set to "date,place_pivots", as asserted by the mock
    }
}
