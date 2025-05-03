use std::fmt::Display;

use serde::{Deserialize, Serialize};

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
    fn artworks_listing_to_string() {
        let listing: ArtworksListing =
            serde_json::from_str(r#"{ "data": [ { "id": 1, "title": "Numero uno" } ] }"#).unwrap();
        let _listing_string: String = listing.to_string();
    }
}
