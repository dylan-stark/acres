use crate::{
    AcresError,
    api::Api,
    artworks::{Search, search_query_params::SearchQueryParams},
};

/// An artworks collection search.
///
/// This corresponds to the [`GET /artworks/search`] endpoint on the public API.
///
/// [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search-2
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchBuilder {
    api: Api,
    q: Option<String>,
    query: Option<String>,
    sort: Option<String>,
}

impl SearchBuilder {
    /// Creates a new search builder.
    pub fn new() -> Self {
        SearchBuilder::default()
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
    /// use acres::artworks::SearchBuilder;
    ///
    /// let api = Api::builder().use_cache(false).build();
    /// SearchBuilder::new().api(api);
    /// ```
    pub fn api(mut self, api: Api) -> Self {
        self.api = api;
        self
    }

    /// Sets the search query.
    ///
    /// # Examples
    ///
    /// ```
    /// use acres::artworks::SearchBuilder;
    ///
    /// SearchBuilder::new().q(Some("monet".into()));
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

    /// Builds artworks search.
    pub async fn build(&self) -> Result<Search, AcresError> {
        tracing::info!(msg = "Searching artworks collection", ?self);
        let endpoint = format!("{}/artworks/search", self.api.base_uri);
        let query_params = SearchQueryParams {
            q: self.q.clone(),
            query: self.query.clone(),
            sort: self.sort.clone(),
        };
        query_params.valid()?;
        self.api.fetch::<Search>(endpoint, Some(query_params)).await
    }
}
