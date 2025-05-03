use std::{env, fmt::Display, path::PathBuf};

use directories::ProjectDirs;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize)]
struct Config {
    #[serde(default)]
    aic_cache_dir: PathBuf,
}

lazy_static! {
    pub static ref AIC_CACHE_DIR: Option<PathBuf> =
        env::var("AIC_CACHE_DIR").ok().map(PathBuf::from);
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let aic_cache_dir = get_aic_cache_dir();
        let builder = config::Config::builder()
            .set_default("aic_cache_dir", aic_cache_dir.to_str().unwrap())?;
        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }
}

fn get_aic_cache_dir() -> PathBuf {
    let directory = if let Some(s) = AIC_CACHE_DIR.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".cache")
    };
    directory
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "aic", env!("CARGO_PKG_NAME"))
}

pub struct Api {}

impl Api {
    pub fn artworks() -> ArtworksListing {
        let config = Config::new().unwrap();
        let artworks_json_path = config.aic_cache_dir.join("artworks.json");
        if artworks_json_path.is_file() {
            let json = std::fs::read_to_string(artworks_json_path).unwrap();
            serde_json::from_str(&json).unwrap()
        } else {
            serde_json::from_str(r#"{ "data": [ { "id": 0, "title": "Nothingness" } ] }"#).unwrap()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Artwork {
    id: usize,
    title: String,
}

#[derive(Serialize, Deserialize)]
pub struct ArtworksListing {
    data: Vec<Artwork>,
}

impl Display for ArtworksListing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(serde_json::to_string(self).unwrap().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_artworks_listing() {
        let _listing: ArtworksListing = Api::artworks();
    }

    #[test]
    fn artworks_listing_to_string() {
        let listing: ArtworksListing =
            serde_json::from_str(r#"{ "data": [ { "id": 1, "title": "Numero uno" } ] }"#).unwrap();
        let _listing_string: String = listing.to_string();
    }
}
