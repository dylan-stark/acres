use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Pagination {
    total: u32,
    limit: u32,
    offset: u32,
    total_pages: u32,
    current_page: u32,
    next_url: String,
}

#[derive(Serialize, Deserialize)]
struct Artwork {
    id: Option<usize>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ArtworksListing {
    pagination: Pagination,
    data: Vec<Artwork>,
}

impl Display for ArtworksListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn basic_pagination() -> serde_json::Value {
        serde_json::json!(
            {
                "total": 1,
                "limit": 12,
                "offset": 0,
                "total_pages": 42,
                "current_page": 1,
                "next_url": "https://www.artic.edu/artworks/?page=2"
            }
        )
    }

    pub fn numero_uno() -> serde_json::Value {
        serde_json::json!(
            {
                "id": 1,
                "title": "Numero uno"
            }
        )
    }

    pub fn numero_tres() -> serde_json::Value {
        serde_json::json!(
            {
                "id": 3,
                "title": "Numero tres"
            }
        )
    }

    pub fn listing_with_numero_uno() -> ArtworksListing {
        ArtworksListing {
            pagination: serde_json::from_str(&basic_pagination().to_string()).unwrap(),
            data: vec![serde_json::from_str(&numero_uno().to_string()).unwrap()],
        }
    }

    pub fn listing_with_numeros_uno_and_tres() -> ArtworksListing {
        ArtworksListing {
            pagination: serde_json::from_str(&basic_pagination().to_string()).unwrap(),
            data: vec![
                serde_json::from_str(&numero_uno().to_string()).unwrap(),
                serde_json::from_str(&numero_tres().to_string()).unwrap(),
            ],
        }
    }

    #[test]
    fn artworks_listing_to_string() {
        let mock_listing = listing_with_numero_uno();
        let _listing_string: String = mock_listing.to_string();
    }

    #[test]
    fn just_title_and_description_fields() {
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
        let listing: ArtworksListing = serde_json::from_str(json).unwrap();

        assert_eq!(listing.data.len(), 2);
        assert!(matches!(&listing.data[0].title, Some(title) if title == "Claude Monet"));
        assert_eq!(listing.data[0].description, None);
    }
}
