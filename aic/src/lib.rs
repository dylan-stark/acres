mod config;

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::config::Config;

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
