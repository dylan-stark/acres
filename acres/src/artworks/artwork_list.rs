use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A list of artworks
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Artworks {
    /// Config
    pub config: Config,
    /// Data
    pub data: Vec<Data>,
    /// Info
    pub info: Info,
    /// Pagination
    pub pagination: Pagination,
}

impl FromStr for Artworks {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub iiif_url: String,
    website_url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub id: u64,
    pub image_id: Option<String>,
    pub title: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Info {
    license_text: String,
    license_links: Vec<String>,
    version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pagination {
    total: u64,
    limit: u64,
    offset: u64,
    total_pages: u64,
    current_page: u64,
}
