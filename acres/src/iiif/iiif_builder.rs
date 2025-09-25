use crate::{
    AcresError,
    iiif::{Format, Iiif, Quality, Region, Rotation, Size},
};
use anyhow::Context;
use serde::Deserialize;

/// An IIIF builder.
#[derive(Debug, Default)]
pub struct IiifBuilder {
    artwork: String,
    region: Option<Region>,
    size: Option<Size>,
    rotation: Option<Rotation>,
    quality: Option<Quality>,
    format: Option<Format>,
}

#[derive(Deserialize)]
struct ArtworkConfig {
    iiif_url: url::Url,
}

#[derive(Deserialize)]
struct ArtworkData {
    image_id: String,
}

#[derive(Deserialize)]
struct Artwork {
    config: ArtworkConfig,
    data: ArtworkData,
}

impl IiifBuilder {
    /// Artwork details.
    pub fn artwork(mut self, artwork: String) -> Self {
        self.artwork = artwork;
        self
    }

    /// Region of image to return.
    pub fn region(mut self, region: Option<Region>) -> Self {
        self.region = region;
        self
    }

    /// Size of the image to return.
    pub fn size(mut self, size: Option<Size>) -> Self {
        self.size = size;
        self
    }

    /// Rotation of the image to return.
    pub fn rotation(mut self, rotation: Option<Rotation>) -> Self {
        self.rotation = rotation;
        self
    }

    /// Quality of the image to return.
    pub fn quality(mut self, quality: Option<Quality>) -> Self {
        self.quality = quality;
        self
    }

    /// Format of the image to return.
    pub fn format(mut self, format: Option<Format>) -> Self {
        self.format = format;
        self
    }

    /// Build the IIIF instance.
    pub async fn build(&self) -> Result<Iiif, AcresError> {
        tracing::info!(msg = "Building IIIF instance", ?self);

        let artwork: Artwork =
            serde_json::from_str(self.artwork.as_str()).context("failed to serialize JSON")?;
        let scheme = artwork.config.iiif_url.scheme();
        let server = artwork
            .config
            .iiif_url
            .host_str()
            .context("failed to parse host from URL")?;
        let prefix = artwork.config.iiif_url.path();
        let identifier = artwork.data.image_id;

        let region = self.region.as_ref().unwrap_or(&Region::Full);
        let size = self.size.as_ref().unwrap_or(&Size::Width(843));
        let rotation = self.rotation.as_ref().unwrap_or(&Rotation::Degrees(0.0));
        let quality = self.quality.as_ref().unwrap_or(&Quality::Default);
        let format = self.format.as_ref().unwrap_or(&Format::Jpg);

        Ok(Iiif(format!(
            "{}://{}{}/{}/{}/{}/{}/{}.{}",
            scheme, server, prefix, identifier, region, size, rotation, quality, format
        )))
    }
}
