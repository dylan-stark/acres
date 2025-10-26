use serde::Serialize;
use serde::ser::SerializeSeq;

use crate::AcresError;

#[derive(Debug)]
pub(super) struct SearchQueryParams {
    pub(super) q: Option<String>,
    pub(super) query: Option<String>,
    pub(super) sort: Option<String>,
    pub(super) from: Option<u32>,
    pub(super) size: Option<u32>,
    pub(super) facets: Option<Vec<String>>,
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

#[cfg(test)]
mod tests {
    use crate::{AcresError, artworks::search_query_params::SearchQueryParams};

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
