//! Artworks.

mod artwork;
mod artwork_info;
mod artwork_list;
mod collection;
mod manifest;
mod search;

pub use artwork::Artwork;
pub use artwork_info::ArtworkInfo;
pub use artwork_list::Artworks;
pub use collection::Collection;
pub use manifest::Manifest;
pub use search::Search;

/// Various requests for resources from the Art Institute.
pub mod request {
    /// Artwork request.
    pub mod artwork {
        pub use crate::artworks::artwork::Request;
    }

    /// Collection request.
    pub mod artworks {
        pub use crate::artworks::collection::{Builder, Request};
    }

    /// Manifest request.
    pub mod manifest {
        pub use crate::artworks::manifest::Request;
    }

    /// Collection request.
    pub mod search {
        pub use crate::artworks::search::{Builder, Request};
    }
}
