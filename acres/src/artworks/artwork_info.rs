use std::fmt::Display;

use iiif::IiifError;
use serde::{Deserialize, Serialize};

use crate::{AcresError, artworks::Artworks};

/// Artwork config.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArtworkInfoConfig {
    /// IIIF URL.
    pub iiif_url: url::Url,
}

/// Artwork data.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArtworkInfoData {
    /// ID.
    pub id: u32,
    /// Image ID.
    pub image_id: String,
    /// Title.
    pub title: String,
}

/// Artwork.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ArtworkInfo {
    /// Config.
    pub config: ArtworkInfoConfig,
    /// Data.
    pub data: ArtworkInfoData,
}

impl Display for ArtworkInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} ({})", self.data.title, self.data.id))
    }
}

impl From<Artworks> for Vec<ArtworkInfo> {
    fn from(value: Artworks) -> Self {
        let iiif_url = url::Url::parse(&value.config.iiif_url).expect("received valid URL");
        value
            .data
            .iter()
            // ArtworkInfos must have IIIF URIs, so they must have image IDs
            .filter_map(|data| {
                data.image_id.clone().map(|image_id| ArtworkInfo {
                    config: ArtworkInfoConfig {
                        iiif_url: iiif_url.clone(),
                    },
                    data: ArtworkInfoData {
                        id: data.id as u32,
                        image_id: image_id.clone(),
                        title: data.title.clone(),
                    },
                })
            })
            .collect()
    }
}

impl TryFrom<ArtworkInfo> for iiif::Uri {
    type Error = AcresError;

    fn try_from(artwork: ArtworkInfo) -> std::result::Result<Self, Self::Error> {
        artwork
            .config
            .iiif_url
            .join(&artwork.data.image_id)
            .map_err(IiifError::InvalidUri)
            .map_err(AcresError::Iiif)?
            .as_str()
            .parse::<iiif::Uri>()
            .map_err(AcresError::Iiif)
    }
}

impl ArtworkInfo {
    /// Load from reader.
    pub fn load<R: std::io::Read>(reader: R) -> Option<Self> {
        serde_json::from_reader(reader).ok()
    }
}
