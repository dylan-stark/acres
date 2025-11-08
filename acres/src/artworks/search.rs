//! Artworks search.

use std::fmt::Display;

use anyhow::Context;
use bytes::{Buf, Bytes};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};

use crate::AcresError;

/// An artworks search.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Search(serde_json::Value);

impl Display for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl From<Bytes> for Search {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl TryFrom<Vec<u8>> for Search {
    type Error = AcresError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(serde_json::from_slice::<Search>(&value).context("loading Search from bytes")?)
    }
}

impl TryFrom<Search> for Vec<u8> {
    type Error = AcresError;

    fn try_from(value: Search) -> Result<Self, Self::Error> {
        Ok(serde_json::to_vec::<Search>(&value).context("dumping Search to bytes")?)
    }
}

impl Search {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Search(response)
    }

    /// Creates a new search builder.
    pub fn builder() -> Builder {
        Builder::default()
    }
}

/// A search request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Request(String);

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Default for Request {
    fn default() -> Self {
        Self(String::from("https://api.artic.edu/api/v1/artworks/search"))
    }
}

impl Request {
    /// Constructs a collection request builder.
    pub fn builder() -> Builder {
        Builder::default()
    }
}

/// An artworks collection search.
///
/// This corresponds to the [`GET /artworks/search`] endpoint on the public API.
///
/// [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search-2
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Builder {
    base_uri: String,
    q: Option<String>,
    query: Option<String>,
    sort: Option<String>,
    from: Option<u32>,
    size: Option<u32>,
    facets: Option<Vec<String>>,
}

impl Builder {
    /// Creates a new search builder.
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
    /// use acres::artworks::request::search::Builder;
    ///
    /// let api = Api::builder().use_cache(false).build();
    /// Builder::new().base_uri(api.base_uri());
    /// ```
    pub fn base_uri(mut self, base_uri: String) -> Self {
        self.base_uri = base_uri;
        self
    }

    /// Sets the search query.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::request::search::Builder;
    ///
    /// Builder::new().q(Some("monet".into()));
    /// ```
    pub fn q(mut self, q: Option<String>) -> Self {
        tracing::info!(msg = "Setting q", ?q);
        self.q = q;
        self
    }

    /// Sets the search more complex query.
    pub fn query(mut self, query: Option<String>) -> Self {
        tracing::info!(msg = "Setting query", ?query);
        self.query = query;
        self
    }

    /// Sets the sort field.
    pub fn sort(mut self, field: Option<String>) -> Self {
        tracing::info!(msg = "Setting sort", ?field);
        self.sort = field;
        self
    }

    /// Sets the from parameter.
    pub fn from(mut self, from: Option<u32>) -> Self {
        tracing::info!(msg = "Setting from", ?from);
        self.from = from;
        self
    }

    /// Sets the size parameter.
    pub fn size(mut self, size: Option<u32>) -> Self {
        tracing::info!(msg = "Setting size", ?size);
        self.size = size;
        self
    }

    /// Sets the facets parameter.
    pub fn facets(mut self, facets: Option<Vec<String>>) -> Self {
        tracing::info!(msg = "Setting facets", ?facets);
        self.facets = facets;
        self
    }

    /// Builds artworks search.
    pub async fn build(&self) -> Result<Request, AcresError> {
        let query_params = SearchQueryParams {
            q: self.q.clone(),
            query: self.query.clone(),
            sort: self.sort.clone(),
            from: self.from,
            size: self.size,
            facets: self.facets.clone(),
        };
        query_params.valid()?;
        let request = format!("{}/artworks/search{}", self.base_uri, query_params);
        Ok(Request(request))
    }
}

#[derive(Debug)]
pub(super) struct SearchQueryParams {
    pub(super) q: Option<String>,
    pub(super) query: Option<String>,
    pub(super) sort: Option<String>,
    pub(super) from: Option<u32>,
    pub(super) size: Option<u32>,
    pub(super) facets: Option<Vec<String>>,
}

impl Display for SearchQueryParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut params: Vec<String> = vec![];
        if let Some(q) = &self.q {
            params.push(format!("q={q}"));
        }
        if let Some(query) = &self.query {
            params.push(format!("query={query}"));
        }
        if let Some(sort) = &self.sort {
            params.push(format!("sort={sort}"));
        }
        if let Some(from) = &self.from {
            params.push(format!("from={from}"));
        }
        if let Some(size) = &self.size {
            params.push(format!("size={size}"));
        }
        if let Some(facets) = &self.facets
            && !facets.is_empty()
        {
            let facets = facets.join(",");
            params.push(format!("facets={facets}"));
        }
        if params.is_empty() {
            Ok(())
        } else {
            f.write_str(format!("?{}", params.join("&")).as_str())
        }
    }
}

impl Serialize for SearchQueryParams {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if let Some(q) = &self.q {
            seq.serialize_element(&("q", q))?
        }
        if let Some(query) = &self.query {
            seq.serialize_element(&("query", query))?
        }
        if let Some(sort) = &self.sort {
            seq.serialize_element(&("sort", sort))?
        }
        if let Some(from) = &self.from {
            seq.serialize_element(&("from", from))?
        }
        if let Some(size) = &self.size {
            seq.serialize_element(&("size", size))?
        }
        if let Some(facets) = &self.facets {
            seq.serialize_element(&("facets", facets.join(",")))?
        }
        seq.end()
    }
}

impl SearchQueryParams {
    pub fn valid(&self) -> Result<(), AcresError> {
        if self.sort.is_some() && self.query.is_none() {
            return Err(AcresError::InvalidSearchQueryParams(
                "sort can only be used if query is also set".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AcresError;

    #[test]
    fn sort_requires_query() {
        let params = SearchQueryParams {
            q: None,
            query: None,
            sort: Some("field".to_string()),
            from: None,
            size: None,
            facets: None,
        };

        let result = params.valid();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(AcresError::InvalidSearchQueryParams(_))
        ));
    }
}
