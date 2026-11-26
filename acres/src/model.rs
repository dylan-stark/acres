//! Typed responses (ADR 0016).
//!
//! Only the fields the binaries use are modeled. Unknown fields are ignored, so a field the API
//! adds later never breaks parsing, and fields that records often leave empty are `Option`s.

use serde::{Deserialize, Serialize};

/// One page of a listing or search: `GET /artworks` or `GET /artworks/search`.
///
/// `T` is the record type, usually [`ArtworkSummary`]. Ask for only its fields with
/// [`ArtworkSummary::FIELDS`], so the API sends no more than is parsed.
///
/// ```
/// use acres::{Page, artworks::ArtworkSummary};
///
/// let body = r#"{
///     "pagination": {"total": 1, "limit": 12, "offset": 0, "total_pages": 1, "current_page": 1},
///     "data": [{"id": 4, "title": "Priest and Boy", "image_id": null}],
///     "config": {"iiif_url": "https://www.artic.edu/iiif/2", "website_url": "https://www.artic.edu"}
/// }"#;
/// let page: Page<ArtworkSummary> = serde_json::from_str(body).unwrap();
/// assert_eq!(page.data[0].title, "Priest and Boy");
/// assert_eq!(page.pagination.total, 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Where this page sits in the whole result. A listing by `ids` has none, so it defaults to
    /// zeros.
    #[serde(default)]
    pub pagination: Pagination,
    /// The records on this page.
    pub data: Vec<T>,
    /// Base URLs for images and the website.
    #[serde(default)]
    pub config: ApiConfig,
}

/// One record: `GET /artworks/{id}`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item<T> {
    /// The record.
    pub data: T,
    /// Base URLs for images and the website.
    #[serde(default)]
    pub config: ApiConfig,
}

/// Where a page sits in the whole result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pagination {
    /// Records matching, across all pages.
    pub total: u64,
    /// Records per page.
    pub limit: u64,
    /// Records before this page.
    pub offset: u64,
    /// Pages in the whole result.
    pub total_pages: u64,
    /// This page, counted from 1.
    pub current_page: u64,
}

/// Base URLs returned with every response.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// The IIIF image server. Take image URLs from here rather than hard-coding it.
    pub iiif_url: String,
    /// The museum's website.
    pub website_url: String,
}

/// The museum's website, for a config that names none, such as one read from an old file.
pub const DEFAULT_WEBSITE_URL: &str = "https://www.artic.edu";

impl ApiConfig {
    /// The IIIF base URI of the image `image_id` on this response's image server.
    ///
    /// ```
    /// use acres::ApiConfig;
    ///
    /// let config = ApiConfig {
    ///     iiif_url: "https://www.artic.edu/iiif/2".into(),
    ///     ..ApiConfig::default()
    /// };
    /// assert_eq!(
    ///     config.image_base_uri("abc").unwrap().to_string(),
    ///     "https://www.artic.edu/iiif/2/abc"
    /// );
    /// ```
    pub fn image_base_uri(&self, image_id: &str) -> Result<iiif::BaseUri, crate::Error> {
        format!("{}/{image_id}", self.iiif_url.trim_end_matches('/'))
            .parse::<iiif::BaseUri>()
            .map_err(|e| crate::RequestError::Iiif(e.into()).into())
    }

    /// The artwork's page on the museum's website, falling back to [`DEFAULT_WEBSITE_URL`].
    ///
    /// ```
    /// use acres::ApiConfig;
    ///
    /// let config = ApiConfig::default();
    /// assert_eq!(config.artwork_url(27992), "https://www.artic.edu/artworks/27992");
    /// ```
    pub fn artwork_url(&self, id: u32) -> String {
        let website = match self.website_url.trim_end_matches('/') {
            "" => DEFAULT_WEBSITE_URL,
            url => url,
        };
        format!("{website}/artworks/{id}")
    }
}

/// A museum department, as `GET /departments` lists them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Department {
    /// The department's ID, such as `PC-3` (European Painting and Sculpture). Filter on it with
    /// `department_id`.
    pub id: String,
    /// Its name.
    #[serde(default)]
    pub title: String,
}

impl Department {
    /// The `fields` to request so the API returns exactly this type.
    pub const FIELDS: &'static [&'static str] = &["id", "title"];
}

/// A kind of artwork, such as Painting or Print, as `GET /artwork-types` lists them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtworkType {
    /// The type's ID. Filter on it with `artwork_type_id`.
    pub id: u32,
    /// Its name.
    #[serde(default)]
    pub title: String,
}

impl ArtworkType {
    /// The `fields` to request so the API returns exactly this type.
    pub const FIELDS: &'static [&'static str] = &["id", "title"];
}

/// What a results list shows for one artwork: enough for a gallery wall label.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtworkSummary {
    /// The artwork's ID.
    pub id: u32,
    /// Its title.
    #[serde(default)]
    pub title: String,
    /// The artist with nationality and life dates, as on a gallery label.
    #[serde(default)]
    pub artist_display: Option<String>,
    /// The date as written on a label, such as `1906` or `c. 1890`.
    #[serde(default)]
    pub date_display: Option<String>,
    /// The IIIF image ID, if the artwork has an image.
    #[serde(default)]
    pub image_id: Option<String>,
    /// Medium, such as `Oil on canvas`.
    #[serde(default)]
    pub medium_display: Option<String>,
    /// How it came to the museum.
    #[serde(default)]
    pub credit_line: Option<String>,
    /// The accession number, such as `1933.1157`.
    #[serde(default)]
    pub main_reference_number: Option<String>,
    /// The artist's ID, for finding more by the same artist.
    #[serde(default)]
    pub artist_id: Option<u32>,
    /// The artist's name alone, such as `Claude Monet`.
    #[serde(default)]
    pub artist_title: Option<String>,
    /// The department's ID, such as `PC-3`, for finding more from the same department.
    #[serde(default)]
    pub department_id: Option<String>,
    /// The museum department holding it.
    #[serde(default)]
    pub department_title: Option<String>,
}

impl ArtworkSummary {
    /// The `fields` to request so the API returns exactly this type.
    pub const FIELDS: &'static [&'static str] = &[
        "id",
        "title",
        "artist_display",
        "date_display",
        "image_id",
        "medium_display",
        "credit_line",
        "main_reference_number",
        "artist_id",
        "artist_title",
        "department_id",
        "department_title",
    ];
}

/// The full record of one artwork, for a detail view.
///
/// It holds everything in [`ArtworkSummary`] plus the rest of the label and record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artwork {
    /// The fields shared with a results list.
    #[serde(flatten)]
    pub summary: ArtworkSummary,
    /// Dimensions as written on a label.
    #[serde(default)]
    pub dimensions: Option<String>,
    /// Where it was made.
    #[serde(default)]
    pub place_of_origin: Option<String>,
    /// Whether it is in the public domain.
    #[serde(default)]
    pub is_public_domain: bool,
    /// Whether it is on view now.
    #[serde(default)]
    pub is_on_view: bool,
    /// The gallery it hangs in, when on view.
    #[serde(default)]
    pub gallery_title: Option<String>,
    /// A longer description, as HTML.
    #[serde(default)]
    pub description: Option<String>,
}

impl Artwork {
    /// The `fields` to request so the API returns exactly this type.
    pub const FIELDS: &'static [&'static str] = &[
        "id",
        "title",
        "artist_display",
        "date_display",
        "image_id",
        "medium_display",
        "credit_line",
        "main_reference_number",
        "artist_id",
        "artist_title",
        "department_id",
        "department_title",
        "dimensions",
        "place_of_origin",
        "is_public_domain",
        "is_on_view",
        "gallery_title",
        "description",
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_fields_start_with_the_summary_fields() {
        assert!(Artwork::FIELDS.starts_with(ArtworkSummary::FIELDS));
    }

    #[test]
    fn missing_optional_fields_parse_as_none() {
        let artwork: Artwork = serde_json::from_str(r#"{"id": 4}"#).unwrap();
        assert_eq!(artwork.summary.id, 4);
        assert_eq!(artwork.summary.title, "");
        assert_eq!(artwork.summary.medium_display, None);
        assert!(!artwork.is_on_view);
    }

    #[test]
    fn nulls_parse_as_none() {
        let summary: ArtworkSummary = serde_json::from_str(
            r#"{"id": 4, "title": "T", "artist_display": null, "image_id": null}"#,
        )
        .unwrap();
        assert_eq!(summary.artist_display, None);
        assert_eq!(summary.image_id, None);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let summary: ArtworkSummary =
            serde_json::from_str(r#"{"id": 4, "title": "T", "is_boosted": true}"#).unwrap();
        assert_eq!(summary.id, 4);
    }

    #[test]
    fn artwork_reads_summary_and_detail_fields_from_one_object() {
        let artwork: Artwork = serde_json::from_str(
            r#"{"id": 4, "title": "T", "artist_display": "A", "medium_display": "Oil on canvas",
                "is_on_view": true, "artist_id": 35}"#,
        )
        .unwrap();
        assert_eq!(artwork.summary.artist_display.as_deref(), Some("A"));
        assert_eq!(
            artwork.summary.medium_display.as_deref(),
            Some("Oil on canvas")
        );
        assert!(artwork.is_on_view);
        assert_eq!(artwork.summary.artist_id, Some(35));
    }
}
