//! What the explorer lists: one artwork, and the IIIF server its image is on.

use acres::{ApiConfig, artworks::ArtworkSummary};
use base64::Engine;

/// The built-in Highlights, the start screen's well-known public-domain works (ADR 0013).
///
/// The weekly canary workflow checks these against the live API, so a stale id fails a run
/// instead of quietly shortening the list. `highlights` in the config replaces them.
pub const HIGHLIGHTS: &str = include_str!("../highlights.json");

/// The built-in Highlights ids, in the order they are listed.
pub fn highlights() -> Vec<u32> {
    serde_json::from_str(HIGHLIGHTS).expect("highlights.json is a list of ids")
}

/// One artwork in a list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub summary: ArtworkSummary,
    /// The image server and website its page came with.
    pub config: ApiConfig,
}

impl Entry {
    pub fn id(&self) -> u32 {
        self.summary.id
    }

    pub fn title(&self) -> &str {
        &self.summary.title
    }

    /// The image's IIIF base URI, if the artwork has an image.
    pub fn base_uri(&self) -> Option<iiif::BaseUri> {
        let image_id = self.summary.image_id.as_deref()?;
        self.config.image_base_uri(image_id).ok()
    }

    /// The artwork's page on the museum's website.
    pub fn link(&self) -> String {
        self.config.artwork_url(self.summary.id)
    }

    /// The record's `lqip` placeholder, a GIF a few pixels across, decoded from its data URI.
    ///
    /// It comes with the list, so showing it costs no request. A saved `Artworks` file has none.
    pub fn placeholder(&self) -> Option<Vec<u8>> {
        let uri = self.summary.thumbnail.as_ref()?.lqip.as_deref()?;
        let (_, data) = uri.split_once(";base64,")?;
        base64::engine::general_purpose::STANDARD.decode(data).ok()
    }

    /// The artworks with images in a saved `Artworks` file, as `acres-cli artworks` writes it.
    pub fn from_file(artworks: acres::artworks::Artworks) -> Vec<Entry> {
        let config = ApiConfig {
            iiif_url: artworks.config.iiif_url.clone(),
            website_url: artworks.config.website_url.clone(),
        };
        artworks
            .data
            .into_iter()
            .filter(|data| data.image_id.is_some())
            .map(|data| Entry {
                summary: ArtworkSummary {
                    id: u32::try_from(data.id).unwrap_or(u32::MAX),
                    title: data.title,
                    image_id: data.image_id,
                    ..ArtworkSummary::default()
                },
                config: config.clone(),
            })
            .collect()
    }

    /// Each record on a page, with the page's config.
    pub fn from_page(page: acres::Page<ArtworkSummary>) -> Vec<Entry> {
        let config = page.config;
        page.data
            .into_iter()
            .map(|summary| Entry {
                summary,
                config: config.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_highlights_parse() {
        let ids = highlights();
        assert_eq!(ids.first(), Some(&24645), "The Great Wave leads");
        assert!(ids.len() >= 5);
    }

    #[test]
    fn base_uri_and_link_come_from_the_config() {
        let entry = Entry {
            summary: ArtworkSummary {
                id: 1,
                image_id: Some("abc".into()),
                ..ArtworkSummary::default()
            },
            config: ApiConfig {
                iiif_url: "https://www.artic.edu/iiif/2/".into(),
                website_url: "https://www.artic.edu".into(),
            },
        };
        assert_eq!(entry.link(), "https://www.artic.edu/artworks/1");
        assert_eq!(
            entry.base_uri().unwrap().to_string(),
            "https://www.artic.edu/iiif/2/abc"
        );
        let no_image = Entry {
            summary: ArtworkSummary::default(),
            ..entry
        };
        assert_eq!(no_image.base_uri(), None);
    }

    #[test]
    fn placeholder_decodes_the_lqip_data_uri() {
        let with = |lqip: &str| Entry {
            summary: ArtworkSummary {
                thumbnail: Some(acres::artworks::Thumbnail {
                    lqip: Some(lqip.into()),
                    ..Default::default()
                }),
                ..ArtworkSummary::default()
            },
            ..Entry::default()
        };
        assert_eq!(
            with("data:image/gif;base64,R0lGODlh").placeholder(),
            Some(b"GIF89a".to_vec())
        );
        assert_eq!(with("not a data uri").placeholder(), None);
        assert_eq!(Entry::default().placeholder(), None);
    }
}
