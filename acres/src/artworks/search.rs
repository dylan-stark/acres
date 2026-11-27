//! Artworks search.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::{Error, RequestError};

/// A [`GET /artworks/search`] request.
///
/// ```rust
/// # use anyhow::Result;
/// use acres::artworks::request::search;
///
/// # fn main() -> Result<()> {
/// let request = search::Request::builder()
///     .base_uri(acres::DEFAULT_BASE_URI)
///     .q(Some("monet".to_string()))
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search
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

/// A [`GET /artworks/search`] request builder.
///
/// ```rust
/// # use anyhow::Result;
/// use acres::artworks::request::search;
///
/// # fn main() -> Result<()> {
/// let builder = search::Request::builder()
///     .base_uri(acres::DEFAULT_BASE_URI)
///     .q(Some("monet".to_string()));
/// # Ok(())
/// # }
/// ```
///
/// [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Builder {
    base_uri: String,
    q: Option<String>,
    query: Option<String>,
    sort: Option<String>,
    descending: bool,
    from: Option<u32>,
    size: Option<u32>,
    facets: Option<Vec<String>>,
    fields: Option<Vec<String>>,
    limit: Option<u32>,
    page: Option<u32>,
    filters: Filters,
}

impl Builder {
    /// Creates a new search builder.
    pub fn new() -> Self {
        Builder::default()
    }

    /// Sets the API base URI, such as a mock server's in tests. The default is
    /// [`acres::DEFAULT_BASE_URI`](crate::DEFAULT_BASE_URI).
    ///
    /// ```
    /// use acres::artworks::request::search::Builder;
    ///
    /// Builder::new().base_uri("http://127.0.0.1:8080/api/v1");
    /// ```
    pub fn base_uri(mut self, base_uri: impl Into<String>) -> Self {
        self.base_uri = base_uri.into();
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
        self.q = q;
        self
    }

    /// Sets the search more complex query.
    pub fn query(mut self, query: Option<String>) -> Self {
        self.query = query;
        self
    }

    /// Sets the sort field. The API sorts ascending unless [`Builder::descending`] is set.
    pub fn sort(mut self, field: Option<String>) -> Self {
        self.sort = field;
        self
    }

    /// Sorts by the [`Builder::sort`] field from highest to lowest, such as newest first.
    ///
    /// ```
    /// use acres::artworks::request::search::{Builder, Filters};
    ///
    /// let request = Builder::new()
    ///     .base_uri("https://api.artic.edu/api/v1")
    ///     .filters(Filters { has_image: true, ..Filters::default() })
    ///     .sort(Some("updated_at".into()))
    ///     .descending(true)
    ///     .build()
    ///     .unwrap();
    /// assert!(request.to_string().contains("sort[updated_at][order]=desc"));
    /// ```
    pub fn descending(mut self, descending: bool) -> Self {
        self.descending = descending;
        self
    }

    /// Sets the from parameter.
    pub fn from(mut self, from: Option<u32>) -> Self {
        self.from = from;
        self
    }

    /// Sets the size parameter.
    pub fn size(mut self, size: Option<u32>) -> Self {
        self.size = size;
        self
    }

    /// Sets the facets parameter.
    pub fn facets(mut self, facets: Option<Vec<String>>) -> Self {
        self.facets = facets;
        self
    }

    /// Sets which fields each record returns, such as [`ArtworkSummary::FIELDS`].
    ///
    /// [`ArtworkSummary::FIELDS`]: crate::artworks::ArtworkSummary::FIELDS
    pub fn fields(mut self, fields: Option<Vec<String>>) -> Self {
        self.fields = fields;
        self
    }

    /// Sets how many records a page holds, at most 100.
    pub fn limit(mut self, limit: Option<u32>) -> Self {
        self.limit = limit;
        self
    }

    /// Sets which page to return, counted from 1.
    pub fn page(mut self, page: Option<u32>) -> Self {
        self.page = page;
        self
    }

    /// Narrows the search with typed filters.
    ///
    /// ```
    /// use acres::artworks::request::search::{Builder, Filters};
    ///
    /// let request = Builder::new()
    ///     .base_uri("https://api.artic.edu/api/v1")
    ///     .q(Some("monet".into()))
    ///     .filters(Filters { is_public_domain: Some(true), ..Filters::default() })
    ///     .build()
    ///     .unwrap();
    /// assert!(request.to_string().contains("query[bool][filter][0][term][is_public_domain]=true"));
    /// ```
    pub fn filters(mut self, filters: Filters) -> Self {
        self.filters = filters;
        self
    }

    /// Builds artworks search.
    pub fn build(&self) -> Result<Request, Error> {
        let query_params = SearchQueryParams {
            q: self.q.clone(),
            query: self.query.clone(),
            sort: self.sort.clone(),
            descending: self.descending,
            from: self.from,
            size: self.size,
            facets: self.facets.clone(),
            fields: self.fields.clone(),
            limit: self.limit,
            page: self.page,
            filters: self.filters.clone(),
        };
        query_params.valid()?;
        let request = format!("{}/artworks/search{}", self.base_uri, query_params);
        Ok(Request(request))
    }
}

/// The most records one search can reach, across all its pages.
///
/// The API's documentation says 10,000, but it answers 403 for any page that ends past the
/// 1,000th record, such as page 21 of 50.
pub const MAX_RESULTS: u64 = 1_000;

/// The most records one page can hold.
pub const MAX_LIMIT: u32 = 100;

/// Typed filters for a search. Each one that is set narrows the results; unset ones don't.
///
/// They are sent as an Elasticsearch `bool` filter in the API's bracket form, such as
/// `query[bool][filter][0][exists][field]=image_id`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filters {
    /// Only artworks with an image.
    pub has_image: bool,
    /// Only public-domain artworks, or only those that are not.
    pub is_public_domain: Option<bool>,
    /// Only artworks on view, or only those that are not.
    pub is_on_view: Option<bool>,
    /// Only artworks from this department, such as `PC-10`.
    pub department_id: Option<String>,
    /// Only artworks of this type.
    pub artwork_type_id: Option<u32>,
    /// Only artworks by this artist.
    pub artist_id: Option<u32>,
    /// Only artworks made within these years, inclusive. Negative years are BCE.
    pub years: Option<(i32, i32)>,
    /// Only artworks whose title or artist contains every one of these words.
    ///
    /// Unlike `q`, which ranks loosely matching records low but still returns them, this keeps
    /// only close matches, which suits a sort other than relevance.
    pub words: Option<String>,
}

impl Filters {
    /// Whether no filter is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The `bool` filter's clauses, each as `(path, value)` pairs relative to the clause.
    fn clauses(&self) -> Vec<Vec<(String, String)>> {
        let mut clauses = Vec::new();
        if self.has_image {
            clauses.push(vec![("[exists][field]".into(), "image_id".into())]);
        }
        let terms = [
            (
                "is_public_domain",
                self.is_public_domain.map(|v| v.to_string()),
            ),
            ("is_on_view", self.is_on_view.map(|v| v.to_string())),
            ("department_id", self.department_id.clone()),
            (
                "artwork_type_id",
                self.artwork_type_id.map(|v| v.to_string()),
            ),
            ("artist_id", self.artist_id.map(|v| v.to_string())),
        ];
        for (field, value) in terms {
            if let Some(value) = value {
                clauses.push(vec![(format!("[term][{field}]"), value)]);
            }
        }
        if let Some(words) = &self.words {
            let at = "[multi_match]";
            clauses.push(vec![
                (format!("{at}[query]"), words.clone()),
                (format!("{at}[fields][0]"), "title".into()),
                (format!("{at}[fields][1]"), "artist_title".into()),
                (format!("{at}[operator]"), "and".into()),
            ]);
        }
        if let Some((start, end)) = self.years {
            clauses.push(vec![("[range][date_end][gte]".into(), start.to_string())]);
            clauses.push(vec![("[range][date_start][lte]".into(), end.to_string())]);
        }
        clauses
    }
}

#[derive(Debug)]
pub(super) struct SearchQueryParams {
    pub(super) q: Option<String>,
    pub(super) query: Option<String>,
    pub(super) sort: Option<String>,
    pub(super) descending: bool,
    pub(super) from: Option<u32>,
    pub(super) size: Option<u32>,
    pub(super) facets: Option<Vec<String>>,
    pub(super) fields: Option<Vec<String>>,
    pub(super) limit: Option<u32>,
    pub(super) page: Option<u32>,
    pub(super) filters: Filters,
}

/// Percent-encodes a query value, so spaces, `&` and `#` in a search don't break the URL.
fn encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

impl Display for SearchQueryParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut params: Vec<String> = vec![];
        if let Some(q) = &self.q {
            params.push(format!("q={}", encode(q)));
        }
        if let Some(query) = &self.query {
            params.push(format!("query={}", encode(query)));
        }
        for (i, clause) in self.filters.clauses().iter().enumerate() {
            for (path, value) in clause {
                params.push(format!("query[bool][filter][{i}]{path}={}", encode(value)));
            }
        }
        match &self.sort {
            Some(sort) if self.descending => {
                params.push(format!("sort[{}][order]=desc", encode(sort)));
            }
            Some(sort) => params.push(format!("sort={}", encode(sort))),
            None => {}
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
        if let Some(fields) = &self.fields
            && !fields.is_empty()
        {
            params.push(format!("fields={}", fields.join(",")));
        }
        if let Some(limit) = &self.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(page) = &self.page {
            params.push(format!("page={page}"));
        }
        if params.is_empty() {
            Ok(())
        } else {
            f.write_str(format!("?{}", params.join("&")).as_str())
        }
    }
}

impl SearchQueryParams {
    pub fn valid(&self) -> Result<(), RequestError> {
        let has_query = self.query.is_some() || !self.filters.is_empty();
        if self.sort.is_some() && !has_query {
            return Err(RequestError::InvalidQuery(
                "sort can only be used if query is also set".to_string(),
            ));
        }
        if self.query.is_some() && !self.filters.is_empty() {
            return Err(RequestError::InvalidQuery(
                "query and filters can't both be set".to_string(),
            ));
        }
        for (name, value) in [("limit", self.limit), ("size", self.size)] {
            if let Some(value) = value
                && value > MAX_LIMIT
            {
                return Err(RequestError::OutOfRange {
                    name,
                    value: value.into(),
                });
            }
        }
        // The last record a request reaches, by page and limit (default 12) or by from and size.
        let by_page = self
            .page
            .map(|page| u64::from(page) * u64::from(self.limit.unwrap_or(12)));
        let by_from = self
            .from
            .map(|from| u64::from(from) + u64::from(self.size.unwrap_or(10)));
        for (name, last) in [("page", by_page), ("from", by_from)] {
            if let Some(last) = last
                && last > MAX_RESULTS
            {
                return Err(RequestError::OutOfRange { name, value: last });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> SearchQueryParams {
        SearchQueryParams {
            q: None,
            query: None,
            sort: None,
            descending: false,
            from: None,
            size: None,
            facets: None,
            fields: None,
            limit: None,
            page: None,
            filters: Filters::default(),
        }
    }

    #[test]
    fn sort_requires_query() {
        let params = SearchQueryParams {
            sort: Some("field".to_string()),
            ..params()
        };

        let result = params.valid();
        assert!(matches!(result, Err(RequestError::InvalidQuery(_))));
    }

    #[test]
    fn sort_works_with_filters() {
        let params = SearchQueryParams {
            sort: Some("title".to_string()),
            filters: Filters {
                has_image: true,
                ..Filters::default()
            },
            ..params()
        };
        assert!(params.valid().is_ok());
    }

    #[test]
    fn query_and_filters_conflict() {
        let params = SearchQueryParams {
            query: Some("{}".to_string()),
            filters: Filters {
                has_image: true,
                ..Filters::default()
            },
            ..params()
        };
        assert!(matches!(params.valid(), Err(RequestError::InvalidQuery(_))));
    }

    #[test]
    fn limit_over_100_is_refused() {
        let params = SearchQueryParams {
            limit: Some(101),
            ..params()
        };
        assert!(matches!(
            params.valid(),
            Err(RequestError::OutOfRange {
                name: "limit",
                value: 101
            })
        ));
    }

    #[test]
    fn last_page_within_1000_is_allowed() {
        let params = SearchQueryParams {
            limit: Some(50),
            page: Some(20),
            ..params()
        };
        assert!(params.valid().is_ok());
    }

    #[test]
    fn page_past_1000_is_refused() {
        let params = SearchQueryParams {
            limit: Some(50),
            page: Some(21),
            ..params()
        };
        assert!(matches!(
            params.valid(),
            Err(RequestError::OutOfRange {
                name: "page",
                value: 1_050
            })
        ));
    }

    #[test]
    fn q_is_percent_encoded() {
        let params = SearchQueryParams {
            q: Some("water lilies & bridges".into()),
            ..params()
        };
        assert_eq!(params.to_string(), "?q=water+lilies+%26+bridges");
    }

    #[test]
    fn filters_become_bool_filter_clauses() {
        let params = SearchQueryParams {
            filters: Filters {
                has_image: true,
                is_on_view: Some(true),
                artist_id: Some(35),
                years: Some((1800, 1900)),
                ..Filters::default()
            },
            ..params()
        };
        assert_eq!(
            params.to_string(),
            "?query[bool][filter][0][exists][field]=image_id\
             &query[bool][filter][1][term][is_on_view]=true\
             &query[bool][filter][2][term][artist_id]=35\
             &query[bool][filter][3][range][date_end][gte]=1800\
             &query[bool][filter][4][range][date_start][lte]=1900"
        );
    }

    #[test]
    fn words_must_all_match_the_title_or_artist() {
        let params = SearchQueryParams {
            filters: Filters {
                words: Some("water lilies".into()),
                ..Filters::default()
            },
            ..params()
        };
        assert_eq!(
            params.to_string(),
            "?query[bool][filter][0][multi_match][query]=water+lilies\
             &query[bool][filter][0][multi_match][fields][0]=title\
             &query[bool][filter][0][multi_match][fields][1]=artist_title\
             &query[bool][filter][0][multi_match][operator]=and"
        );
    }

    #[test]
    fn fields_limit_and_page_are_sent() {
        let params = SearchQueryParams {
            fields: Some(vec!["id".into(), "title".into()]),
            limit: Some(50),
            page: Some(2),
            ..params()
        };
        assert_eq!(params.to_string(), "?fields=id,title&limit=50&page=2");
    }
}
