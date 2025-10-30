use std::fmt::Display;

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use iiif::IiifError;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{AcresError, artworks::Artworks};

use super::artwork_builder::{ArtworkBuilder, ManifestBuilder};

/// Artwork from the AIC collection.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Artwork(serde_json::Value);

impl Display for Artwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl Artwork {
    /// Load from reader.
    pub fn load<R: std::io::Read>(reader: R) -> Option<Self> {
        serde_json::from_reader(reader).ok()
    }
}

impl From<Bytes> for Artwork {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl Artwork {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Artwork(response)
    }

    /// Constructs the IIIF image URL.
    ///
    /// The AIC uses IIIF's Image API 2.0: <https://iiif.io/api/image/2.0/>.
    pub fn to_iiif(&self) -> Result<String, AcresError> {
        let ifff_url = self.0["config"]["iiif_url"]
            .as_str()
            .context("artwork JSON is missing .config.iiif_url")?;
        let identifier = self.0["data"]["image_id"]
            .as_str()
            .context("artwork JSON is missing .data.image_id")?;
        let region = "full";
        let size = "843,";
        let rotation = 0;
        let quality = "default";
        let format = "jpg";
        debug!(
            ifff_url,
            identifier, region, size, rotation, quality, format
        );
        Ok(format!(
            "{ifff_url}/{identifier}/{region}/{size}/{rotation}/{quality}.{format}"
        ))
    }

    /// Creates an artwork builder.
    pub fn builder() -> ArtworkBuilder {
        ArtworkBuilder::default()
    }
}

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
            .map_err(IiifError::ParseUri)
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

/// Artwork manifest from the AIC collection.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Manifest(serde_json::Value);

impl Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl From<Bytes> for Manifest {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl Manifest {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Manifest(response)
    }

    /// Returns a new manifest builder.
    pub fn builder() -> ManifestBuilder {
        ManifestBuilder::default()
    }
}
