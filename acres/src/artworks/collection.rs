//! Artworks collections.

use std::fmt::Display;
use std::str::FromStr;

use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};

use crate::AcresError;

/// A list of artworks
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Artworks {
    /// Config
    pub config: Config,
    /// Data
    pub data: Vec<Data>,
    /// Info
    pub info: Info,
    /// Pagination
    pub pagination: Pagination,
}

impl FromStr for Artworks {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Config {
    pub iiif_url: String,
    website_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Data {
    pub id: u64,
    pub image_id: Option<String>,
    pub title: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Info {
    license_text: String,
    license_links: Vec<String>,
    version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Pagination {
    total: u64,
    limit: u64,
    offset: u64,
    total_pages: u64,
    current_page: u64,
}
/// An artworks collection request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Request(String);

impl Default for Request {
    fn default() -> Self {
        Self(String::from("https://api.artic.edu/api/v1/artworks"))
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Request {
    /// Constructs a collection request builder.
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
    base_uri: String,
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
    /// Builder::new().base_uri(api.base_uri());
    /// ```
    pub fn base_uri(mut self, base_uri: String) -> Self {
        self.base_uri = base_uri;
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

    /// Builds request for artworks collection.
    pub fn build(&self) -> Result<Request, AcresError> {
        let query_params = CollectionQueryParams {
            ids: self.ids.clone(),
            limit: self.limit,
            page: self.page,
            fields: self.fields.clone(),
            include: self.include.clone(),
        };
        let request = format!("{}/artworks{}", self.base_uri, query_params);
        Ok(Request(request))
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

impl Display for CollectionQueryParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut params: Vec<String> = vec![];
        if let Some(ids) = &self.ids {
            let ids = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            params.push(format!("ids={ids}"));
        }
        if let Some(limit) = &self.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(page) = &self.page {
            params.push(format!("page={page}"));
        }
        if !self.fields.is_empty() {
            let fields = self.fields.join(",");
            params.push(format!("fields={fields}"));
        }
        if !self.include.is_empty() {
            let include = self.include.join(",");
            params.push(format!("include={include}"));
        }
        if params.is_empty() {
            Ok(())
        } else {
            f.write_str(format!("?{}", params.join("&")).as_str())
        }
    }
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

    #[test]
    fn api_artworks_collection() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new().base_uri(base_uri.clone()).build().unwrap();

        assert_eq!(request.to_string(), format!("{base_uri}/artworks"));
    }

    #[test]
    fn api_artworks_collection_by_ids() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new()
            .base_uri(base_uri.clone())
            .ids(Some(vec![1, 3]))
            .build()
            .unwrap();

        assert_eq!(request.to_string(), format!("{base_uri}/artworks?ids=1,3"));
    }

    #[tokio::test]
    async fn api_artworks_collection_with_limit() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new()
            .base_uri(base_uri.clone())
            .limit(Some(2))
            .build()
            .unwrap();

        assert_eq!(request.to_string(), format!("{base_uri}/artworks?limit=2"));
    }

    #[test]
    fn api_artworks_collection_with_page() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new()
            .base_uri(base_uri.clone())
            .page(Some(2))
            .build()
            .unwrap();

        assert_eq!(request.to_string(), format!("{base_uri}/artworks?page=2"));
    }

    #[test]
    fn api_artworks_collection_with_fields() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new()
            .base_uri(base_uri.clone())
            .fields(Some(vec!["title".into(), "description".into()]))
            .build()
            .unwrap();

        assert_eq!(
            request.to_string(),
            format!("{base_uri}/artworks?fields=title,description")
        );
    }

    #[tokio::test]
    async fn api_artworks_collection_with_include() {
        let base_uri = String::from("https://example.org/api/v1");

        let request = Builder::new()
            .base_uri(base_uri.clone())
            .include(Some(vec!["date".into(), "place_pivots".into()]))
            .build()
            .unwrap();

        assert_eq!(
            request.to_string(),
            format!("{base_uri}/artworks?include=date,place_pivots")
        );
    }
}
