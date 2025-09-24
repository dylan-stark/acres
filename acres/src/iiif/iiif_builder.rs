use crate::{
    AcresError,
    iiif::{Iiif, Region},
};
use anyhow::Context;
use serde::Deserialize;

/// An IIIF builder.
#[derive(Debug, Default)]
pub struct IiifBuilder {
    artwork: String,
    region: Option<Region>,
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

        Ok(Iiif(format!("{}://{}{}/{}/{}", scheme, server, prefix, identifier, region)))
    }
}
