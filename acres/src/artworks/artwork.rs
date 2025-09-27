use std::fmt::Display;

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use serde::Deserialize;
use tracing::debug;

#[cfg(feature = "image")]
use bytes::Bytes;

#[cfg(feature = "image")]
use std::path::PathBuf;

use crate::{AcresError, common::FromBytes};

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

impl FromBytes<Artwork> for Artwork {
    fn from_bytes(data: Bytes) -> Result<Artwork, AcresError> {
        let reader = data.reader();
        let artwork = serde_json::from_reader(reader).unwrap();
        Ok(artwork)
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

    /// Renders the artwork as image (bytes).
    #[cfg(feature = "image")]
    pub async fn to_image(&self) -> Result<Bytes, AcresError> {
        use crate::config::Config;

        let config = Config::new().context("failed to load config")?;
        let identifier = self.0["data"]["image_id"]
            .as_str()
            .context("artwork JSON is missing .data.image_id")?;
        let image_filename = format!("{}.jpg", identifier);
        let image_path = config.cache_dir.join("images").join(image_filename);
        if config.use_cache && image_path.is_file() {
            tracing::info!(msg = "Using cached file", ?image_path);
            Ok(Self::load_image(&image_path)?)
        } else {
            let iiif_url = self.to_iiif()?;
            let bytes = reqwest::get(iiif_url)
                .await
                .context("getting image via IIIF url")?
                .bytes()
                .await
                .context("getting bytes for image")?;
            if config.use_cache {
                Self::store_image(&bytes, &image_path)?;
            }
            Ok(bytes)
        }
    }

    #[cfg(feature = "image")]
    fn load_image(file_path: &PathBuf) -> anyhow::Result<Bytes> {
        let image = std::fs::read(file_path).with_context(|| {
            format!(
                "failed to read cached image file from {}",
                file_path.display()
            )
        })?;
        Ok(image.into())
    }

    #[cfg(feature = "image")]
    fn store_image(image: &Bytes, file_path: &PathBuf) -> anyhow::Result<()> {
        std::fs::create_dir_all(file_path.parent().expect("path has parent")).with_context(
            || {
                format!(
                    "failed to create parent directory for {}",
                    file_path.display()
                )
            },
        )?;
        std::fs::write(file_path, image)
            .with_context(|| format!("failed to write {}", file_path.display()))?;
        Ok(())
    }

    /// Renders the artwork as ASCII art.
    #[cfg(feature = "ascii-art")]
    pub async fn to_ascii(&self, chars_wide: usize) -> Result<String, AcresError> {
        use crate::config::Config;

        let config = Config::new().context("failed to load config")?;
        let identifier = self.0["data"]["image_id"]
            .as_str()
            .context("artwork JSON is missing .data.image_id")?;
        let ascii_filename = format!("{}.{}.ascii", identifier, chars_wide);
        let ascii_path = config.cache_dir.join("ascii").join(ascii_filename);
        if config.use_cache && ascii_path.is_file() {
            tracing::info!(msg = "Using cached file", ?ascii_path);
            Ok(Self::load_ascii(&ascii_path)?)
        } else {
            let bytes = self.to_image().await?; // Produces image bytes, not an image::Image
            let art = ascii_art::AsciiArt { chars_wide };
            let ascii = art
                .bytes_to_ascii(bytes)
                .context("failed to render image bytes to ASCII")?;
            if config.use_cache {
                Self::store_ascii(&ascii, &ascii_path)?;
            }
            Ok(ascii)
        }
    }

    #[cfg(feature = "ascii-art")]
    fn load_ascii(file_path: &PathBuf) -> anyhow::Result<String> {
        let ascii = std::fs::read_to_string(file_path).with_context(|| {
            format!(
                "failed to read cached ascii file from {}",
                file_path.display()
            )
        })?;
        Ok(ascii)
    }

    #[cfg(feature = "ascii-art")]
    fn store_ascii(ascii: &String, file_path: &PathBuf) -> anyhow::Result<()> {
        std::fs::create_dir_all(file_path.parent().expect("path has parent")).with_context(
            || {
                format!(
                    "failed to create parent directory for {}",
                    file_path.display()
                )
            },
        )?;
        std::fs::write(file_path, ascii)
            .with_context(|| format!("failed to write {}", file_path.display()))?;
        Ok(())
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

    #[derive(Debug)]
    pub struct AsciiArt {
        pub chars_wide: usize,
    }

    impl AsciiArt {
        pub fn bytes_to_ascii(self, bytes: Bytes) -> Result<String> {
            tracing::info!("converting bytes to ascii");
            tracing::debug!(?self);
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
            let brightness_offset = 0.;
            let algorithm = get_conversion_algorithm("edge-augmented");
            let char_rows = img_to_char_rows(
                &font,
                &luma_img,
                convert,
                Some(self.chars_wide),
                brightness_offset,
                &algorithm,
            );
            Ok(char_rows_to_terminal_color_string(&char_rows, &dyn_img))
        }
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

impl FromBytes<Manifest> for Manifest {
    fn from_bytes(data: Bytes) -> Result<Manifest, AcresError> {
        let reader = data.reader();
        let manifest = serde_json::from_reader(reader).unwrap();
        Ok(manifest)
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
