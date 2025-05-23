use std::fmt::Display;

use anyhow::Context;
use bytes::Bytes;
use tracing::debug;

use crate::AcresError;

use super::artwork_builder::ArtworkBuilder;

/// Artwork from the AIC collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Artwork(serde_json::Value);

impl Display for Artwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl Artwork {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Artwork(response)
    }

    /// Constructs the IIIF image URL.
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

    /// Renders the artwork as image (bytes).
    #[cfg(feature = "image")]
    pub async fn to_image(&self) -> Result<Bytes, AcresError> {
        let iiif_url = self.to_iiif()?;
        let bytes = reqwest::get(iiif_url)
            .await
            .context("getting image via IIIF url")?
            .bytes()
            .await
            .context("getting bytes for image")?;
        Ok(bytes)
    }

    /// Renders the artwork as ASCII art.
    #[cfg(feature = "ascii-art")]
    pub async fn to_ascii(&self) -> Result<String, AcresError> {
        let bytes = self.to_image().await?; // Produces image bytes, not an image::Image
        let ascii =
            ascii_art::bytes_to_ascii(bytes).context("failed to render image bytes to ASCII")?;
        Ok(ascii)
    }

    /// Creates an artwork builder.
    pub fn builder() -> ArtworkBuilder {
        ArtworkBuilder::default()
    }
}

#[cfg(feature = "ascii-art")]
mod ascii_art {
    use std::io::Cursor;

    use anyhow::{Context, Result};
    use bytes::Bytes;
    use image::io::Reader;
    use img_to_ascii::{
        convert::{
            char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
            img_to_char_rows,
        },
        font::Font,
        image::LumaImage,
    };

    pub fn bytes_to_ascii(bytes: Bytes) -> Result<String> {
        const ALPHABET: &[u8] = include_bytes!("../../.data/alphabet.txt");
        const BITOCRA_13: &[u8] = include_bytes!("../../.data/bitocra-13.bdf");
        let dyn_img = Reader::new(Cursor::new(bytes))
            .with_guessed_format()
            .context("image reader failed")?
            .decode()
            .context("image decode failed")?;
        let alphabet = &ALPHABET.iter().map(|&c| c as char).collect::<Vec<char>>();
        let font = Font::from_bdf_stream(BITOCRA_13, alphabet, false);
        let luma_img = LumaImage::from(&dyn_img);
        let convert = get_converter("direction-and-intensity");
        let out_width_chars = Some(80);
        let brightness_offset = 0.;
        let algorithm = get_conversion_algorithm("edge-augmented");
        let char_rows = img_to_char_rows(
            &font,
            &luma_img,
            convert,
            out_width_chars,
            brightness_offset,
            &algorithm,
        );
        Ok(char_rows_to_terminal_color_string(&char_rows, &dyn_img))
    }
}
