//! What the explorer lists: one artwork, and the IIIF server its image is on.

use acres::{ApiConfig, artworks::ArtworkSummary};
use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size};

/// The built-in Highlights, the start screen's well-known public-domain works (ADR 0013).
///
/// The weekly canary workflow checks these against the live API, so a stale id fails a run
/// instead of quietly shortening the list. `highlights` in the config replaces them.
pub const HIGHLIGHTS: &str = include_str!("../highlights.json");

/// The built-in Highlights ids, in the order they are listed.
pub fn highlights() -> Vec<u32> {
    serde_json::from_str(HIGHLIGHTS).expect("highlights.json is a list of ids")
}

/// Width of the full image: the AIC's recommended size, which it serves for every artwork.
pub const FULL_WIDTH: u32 = 843;

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

    /// The request for the full image, if the artwork has one. It is the one request the TUI
    /// makes for an image, so it is also what the cache is checked for (ADR 0018).
    pub fn image_request(&self) -> Option<ImageRequest> {
        Some(
            ImageRequest::builder()
                .base_uri(self.base_uri()?)
                .region(Region::Full)
                .size(Size::Width(FULL_WIDTH))
                .rotation(Rotation::Degrees(
                    0.0.try_into().expect("0 degrees is a valid setting"),
                ))
                .quality(Quality::Default)
                .format(Format::Jpg)
                .build(),
        )
    }

    /// The artworks in `text`: a saved list (`saved.json`), or an `Artworks` JSON as
    /// `acres-cli artworks` writes it.
    pub fn from_text(text: &str) -> color_eyre::Result<Vec<Entry>> {
        if crate::saved::is_saved_list(text) {
            return crate::saved::entries_from(text);
        }
        Ok(Entry::from_file(serde_json::from_str(text)?))
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
        assert_eq!(
            entry.image_request().unwrap().to_string(),
            "https://www.artic.edu/iiif/2/abc/full/843,/0/default.jpg"
        );
        let no_image = Entry {
            summary: ArtworkSummary::default(),
            ..entry
        };
        assert_eq!(no_image.base_uri(), None);
        assert!(no_image.image_request().is_none());
    }
}
