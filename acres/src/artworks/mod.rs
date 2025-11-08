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

pub use artwork::Artwork;
pub use artwork_info::ArtworkInfo;
pub use artwork_list::Artworks;
pub use collection::Collection;
pub use collection_builder::CollectionBuilder;
pub use manifest::Manifest;
pub use search::Search;
pub use search_builder::SearchBuilder;

/// Various requests for resources from the Art Institute.
pub mod request {
    /// Artwork request.
    pub mod artwork {
        pub use crate::artworks::artwork::Request;
    }
}
