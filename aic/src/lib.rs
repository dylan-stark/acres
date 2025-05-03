mod artworks;
mod config;

use crate::artworks::ArtworksListing;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_artworks_listing() {
        let _listing: ArtworksListing = Api::artworks();
    }
}
