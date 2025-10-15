use serde::{Deserialize, Serialize};

/// A list of artworks
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Artworks {
    config: Config,
    data: Vec<Data>,
    info: Info,
    pagination: Pagination,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Config {
    iiif_url: String,
    website_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Data {
    id: u64,
    title: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Info {
    license_text: String,
    license_links: Vec<String>,
    version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Pagination {
    total: u64,
    limit: u64,
    offset: u64,
    total_pages: u64,
    current_page: u64,
}
