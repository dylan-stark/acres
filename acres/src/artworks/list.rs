//! List artworks operation.

use std::fmt::Display;

/// A list of artworks.
///
/// This is the response from the [`GET /artworks`].
///
/// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
#[derive(Clone, Debug)]
pub struct ArtworksList(serde_json::Value);

impl Display for ArtworksList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl ArtworksList {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        ArtworksList(response)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::common;

    #[test]
    fn artworks_list_to_string() {
        let mock_list = common::tests::list_with_numero_uno();
        let _list_string: String = mock_list.to_string();
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

        // When we create a new artworks list with it
        let json_value: serde_json::Value = serde_json::from_str(json).unwrap();
        let list = ArtworksList(json_value.clone());

        // Then the list "looks like" what we got from the server
        let list_value: serde_json::Value = serde_json::from_str(&format!("{}", list)).unwrap();
        assert_eq!(list_value, json_value);
    }
}
