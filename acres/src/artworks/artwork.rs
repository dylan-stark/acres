use std::fmt::Display;

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use serde::Deserialize;
use tracing::debug;

use crate::AcresError;

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
#[derive(Debug, Deserialize)]
pub struct ArtworkInfoConfig {
    /// IIIF URL.
    pub iiif_url: url::Url,
}

/// Artwork data.
#[derive(Debug, Deserialize)]
pub struct ArtworkInfoData {
    /// ID.
    pub id: u32,
    /// Image ID.
    pub image_id: String,
}

/// Artwork.
#[derive(Debug, Deserialize)]
pub struct ArtworkInfo {
    /// Config.
    pub config: ArtworkInfoConfig,
    /// Data.
    pub data: ArtworkInfoData,
}

impl TryFrom<ArtworkInfo> for iiif::BaseUri {
    type Error = AcresError;

    fn try_from(artwork: ArtworkInfo) -> std::result::Result<Self, Self::Error> {
        iiif::BaseUri::builder()
            .scheme(
                iiif::Scheme::parse(artwork.config.iiif_url.scheme())
                    .map_err(AcresError::IiifError)?,
            )
            .server(
                artwork
                    .config
                    .iiif_url
                    .host_str()
                    .context("failed to parse host from URL")?,
            )
            .prefix(artwork.config.iiif_url.path())
            .identifier(&artwork.data.image_id)
            .build()
            .map_err(|error| AcresError::IiifError(error.to_string()))
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
