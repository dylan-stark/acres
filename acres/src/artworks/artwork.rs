use std::fmt::Display;

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

use crate::AcresError;

/// Artwork from the AIC collection.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Artwork(serde_json::Value);

impl Display for Artwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl From<Bytes> for Artwork {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl TryFrom<Artwork> for iiif::ImageRequest {
    type Error = AcresError;

    fn try_from(value: Artwork) -> Result<Self, Self::Error> {
        let ifff_url = value.0["config"]["iiif_url"]
            .as_str()
            .context("artwork JSON is missing .config.iiif_url")?;
        let identifier = value.0["data"]["image_id"]
            .as_str()
            .context("artwork JSON is missing .data.image_id")?;
        let uri: iiif::Uri = format!("{}/{}", ifff_url, identifier)
            .parse()
            .context("failed to parse IIIF URI")?;
        Ok(iiif::ImageRequest::builder()
            .uri(uri)
            .region(iiif::Region::Full)
            .size(iiif::Size::Width(843))
            .rotation(iiif::Rotation::default())
            .quality(iiif::Quality::Default)
            .format(iiif::Format::Jpg)
            .build())
    }
}

impl Artwork {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Artwork(response)
    }
}

pub mod requests {
    use serde::{Deserialize, Serialize};
    use std::fmt::Display;

    /// Defines a request for artwork.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct Artwork {
        base_uri: String,
        id: u32,
    }

    impl Display for Artwork {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!("{}/artworks/{}", self.base_uri, self.id))
        }
    }

    impl Artwork {
        /// Constructs a new artwork request.
        pub fn new(base_uri: String, id: u32) -> Self {
            Self { base_uri, id }
        }
    }
}
