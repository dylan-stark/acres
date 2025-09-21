//! Artworks search.

use std::fmt::Display;

use anyhow::Context;
use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

use crate::{AcresError, common::FromBytes};

use super::search_builder::SearchBuilder;

/// An artworks search.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Search(serde_json::Value);

impl Display for Search {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl FromBytes<Search> for Search {
    fn from_bytes(data: Bytes) -> Result<Search, AcresError> {
        let reader = data.reader();
        let search = serde_json::from_reader(reader).unwrap();
        Ok(search)
    }
}

impl Search {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Search(response)
    }

    /// Constructs bytes from Search.
    pub fn to_bytes(search: Search) -> Result<Vec<u8>, AcresError> {
        Ok(serde_json::to_vec::<Search>(&search).context("dumping Search to bytes")?)
    }

    /// Constructs Search from bytes.
    pub fn from_bytes(cached: Vec<u8>) -> Result<Self, AcresError> {
        Ok(serde_json::from_slice::<Search>(&cached).context("loading Search from bytes")?)
    }

    /// Creates a new search builder.
    pub fn builder() -> SearchBuilder {
        SearchBuilder::default()
    }
}
