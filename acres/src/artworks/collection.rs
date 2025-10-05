//! Artworks collections.

use std::fmt::Display;

use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

use super::collection_builder::CollectionBuilder;

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
    pub fn builder() -> CollectionBuilder {
        CollectionBuilder::default()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common;

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
}
