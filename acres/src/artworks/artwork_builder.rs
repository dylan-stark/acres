use std::path::PathBuf;

use crate::{AcresError, Api, config::Config};
use anyhow::{Context, anyhow};
use reqwest::StatusCode;

use super::Artwork;

/// An artwork builder.
#[derive(Debug, Default)]
pub struct ArtworkBuilder {
    api: Api,
    id: u32,
}

impl ArtworkBuilder {
    /// The artwork identifier.
    pub fn id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Build the actual artwork.
    ///
    ///
    pub async fn build(&self) -> Result<Artwork, AcresError> {
        tracing::info!(msg = "Getting artwork", ?self);

        let config = Config::new().context("failed to load config")?;
        let artwork_json_filename = format!("artwork.{}.json", self.id);
        let artwork_json_path = config.cache_dir.join(artwork_json_filename);
        if config.use_cache && self.api.use_cache && artwork_json_path.is_file() {
            tracing::info!(msg = "Using cached file", ?artwork_json_path);
            Ok(Self::load_artwork(&artwork_json_path)?)
        } else {
            tracing::info!("Not using cache");
            match Self::request_artwork(&self.api.base_uri, self.id).await {
                Ok(artwork) => {
                    Self::store_artwork(&artwork, &artwork_json_path)?;
                    Ok(artwork)
                }
                Err(error) => Err(error),
            }
        }
    }

    fn load_artwork(file_path: &PathBuf) -> anyhow::Result<Artwork> {
        let json = std::fs::read_to_string(file_path)
            .with_context(|| format!("failed to read cached file from {}", file_path.display()))?;
        Ok(Artwork::new(
            serde_json::from_str(&json).context("failed to serialize JSON")?,
        ))
    }

    fn store_artwork(artwork: &Artwork, file_path: &PathBuf) -> anyhow::Result<()> {
        std::fs::create_dir_all(file_path.parent().expect("path has parent")).with_context(
            || {
                format!(
                    "failed to create parent directory for {}",
                    file_path.display()
                )
            },
        )?;
        std::fs::write(file_path, artwork.to_string())
            .with_context(|| format!("failed to write {}", file_path.display()))?;
        Ok(())
    }

    async fn request_artwork(base_uri: &String, id: u32) -> Result<Artwork, AcresError> {
        let artwork_path = format!("{}/artworks/{}", base_uri, id);
        tracing::debug!(artwork_path);
        let client = reqwest::Client::new();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "user-agent",
            format!("ACRES/{}", env!("CARGO_PKG_VERSION"),)
                .parse()
                .context("failed constructing user-agent header")?,
        );
        headers.insert(
            "ACRES-User-Agent",
            "ACRES (dylan.stark@gmail.com)"
                .parse()
                .context("failed constructing ACRES-User-Agent header")?,
        );
        tracing::debug!(?headers);

        let response = client
            .get(&artwork_path)
            .headers(headers)
            .send()
            .await
            .with_context(|| format!("failed to GET {}", artwork_path))?;
        match response.status() {
            StatusCode::OK => {
                let artwork = response
                    .json::<serde_json::Value>()
                    .await
                    .with_context(|| format!("failed to get JSON from GET {}", artwork_path))?;
                Ok(Artwork::new(artwork))
            }
            StatusCode::NOT_FOUND => Err(anyhow!("Could not find artwork {}", id).into()),
            _ => Err(response
                .json::<serde_json::Value>()
                .await
                .map(|value| anyhow!("{}: {}", value["error"], value["detail"]))
                .with_context(|| format!("failed to get JSON from GET {}", artwork_path))?
                .into()),
        }
    }
}
