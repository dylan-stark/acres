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
struct Thumbnail {
    lqip: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    alt_text: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct DimensionsDetail {
    depth: Option<u32>,
    width: Option<u32>,
    height: Option<u32>,
    diameter: Option<u32>,
    clarification: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Color {
    h: Option<u32>,
    l: Option<u32>,
    s: Option<u32>,
    percentage: Option<f64>,
    population: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct Artwork {
    id: Option<usize>,
    api_model: Option<String>,
    api_link: Option<String>,
    is_boosted: Option<bool>,
    title: Option<String>,
    alt_titles: Option<Vec<Option<String>>>,
    thumbnail: Option<Thumbnail>,
    main_reference_number: Option<String>,
    has_not_been_viewed_much: Option<bool>,
    boost_rank: Option<u32>,
    date_start: Option<u16>,
    date_end: Option<u16>,
    date_display: Option<String>,
    date_qualifier_title: Option<String>,
    date_qualifier_id: Option<u32>,
    artist_display: Option<String>,
    place_of_origin: Option<String>,
    description: Option<String>,
    short_description: Option<String>,
    dimensions: Option<String>,
    dimensions_detail: Option<Vec<DimensionsDetail>>,
    medium_display: Option<String>,
    inscriptions: Option<String>,
    credit_line: Option<String>,
    catalogue_display: Option<String>,
    publication_history: Option<String>,
    exhibition_history: Option<String>,
    provenance_text: Option<String>,
    edition: Option<String>,
    publishing_verification_level: Option<String>,
    internal_department_id: Option<u32>,
    fiscal_year: Option<u16>,
    fiscal_year_deaccession: Option<u16>,
    is_public_domain: Option<bool>,
    is_zoomable: Option<bool>,
    max_zoom_window_size: Option<u32>,
    copyright_notice: Option<String>,
    has_multimedia_resources: Option<bool>,
    has_educational_resources: Option<bool>,
    has_advanced_imaging: Option<bool>,
    colorfulness: Option<f32>,
    color: Option<Color>,
    latitude: Option<u32>,
    longitude: Option<u32>,
    latlon: Option<String>,
    is_on_view: Option<bool>,
    on_loan_display: Option<String>,
    gallery_title: Option<String>,
    gallery_id: Option<u32>,
    nomisma_id: Option<String>,
    artwork_type_title: Option<String>,
    artwork_type_id: Option<u32>,
    department_title: Option<String>,
    department_id: Option<u32>,
    artist_id: Option<u32>,
    artist_title: Option<String>,
    alt_artist_ids: Option<Vec<u32>>,
    artist_ids: Option<u32>,
    artist_titles: Option<Vec<String>>,
    category_ids: Option<Vec<String>>,
    category_titles: Option<Vec<String>>,
    term_titles: Option<Vec<String>>,
    style_id: Option<String>,
    style_title: Option<String>,
    alt_style_ids: Option<Vec<String>>,
    style_ids: Option<Vec<String>>,
    style_titles: Option<Vec<String>>,
    classification_id: Option<String>,
    classification_title: Option<String>,
    alt_classification_ids: Option<Vec<String>>,
    classification_ids: Option<Vec<String>>,
    classification_titles: Option<Vec<String>>,
    subject_id: Option<String>,
    alt_subject_ids: Option<Vec<String>>,
    subject_ids: Option<Vec<String>>,
    subject_titles: Option<Vec<String>>,
    material_id: Option<String>,
    alt_material_ids: Option<Vec<String>>,
    material_ids: Option<Vec<String>>,
    material_titles: Option<Vec<String>>,
    technique_id: Option<String>,
    alt_technique_ids: Option<Vec<String>>,
    technique_ids: Option<Vec<String>>,
    technique_titles: Option<Vec<String>>,
    theme_titles: Option<Vec<String>>,
    image_id: Option<String>,
    alt_image_ids: Option<Vec<String>>,
    document_ids: Option<Vec<String>>,
    sound_ids: Option<Vec<String>>,
    video_ids: Option<Vec<String>>,
    text_ids: Option<Vec<String>>,
    section_ids: Option<Vec<String>>,
    section_titles: Option<Vec<String>>,
    site_ids: Option<Vec<String>>,
    // NOTE: The following two are documented as "internal" fields that should not be used directly by
    // the user, so we are leaving them as raw values.
    suggest_autocomplete_boosted: Option<serde_json::Value>,
    suggest_autocomplete_all: Option<serde_json::Value>,
    source_updated_at: Option<String>,
    updated_at: Option<String>,
    timestamp: Option<String>,
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
    use predicates::prelude::predicate;
    use predicates::Predicate;

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
        assert!(
            matches!(&listing.data[1].title, Some(title) if predicate::str::contains("Drinking Cup").eval(title))
        );
        assert!(
            matches!(&listing.data[1].description, Some(title) if predicate::str::contains("During the course of the 5th and 4th centuries BCE,").eval(title))
        );
    }
}
