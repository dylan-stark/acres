//! Artworks.

mod artwork;
mod artwork_info;
mod artwork_list;
mod collection;
mod collection_builder;
mod collection_query_params;
mod manifest;
mod search;
mod search_builder;
mod search_query_params;

pub use self::artwork::Artwork;
pub use self::artwork_info::ArtworkInfo;
pub use self::artwork_list::Artworks;
pub use self::collection::Collection;
pub use self::collection_builder::CollectionBuilder;
pub use self::manifest::Manifest;
pub use self::search::Search;
pub use self::search_builder::SearchBuilder;

/// Various requests for resources from the Art Institute.
pub mod requests {
    pub use super::artwork::requests::Artwork;
}
