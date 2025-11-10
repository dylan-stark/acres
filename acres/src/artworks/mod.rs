//! Modules and types for working with the [Artworks Collection].
//!
//! [Artworks Collection]: https://api.artic.edu/docs/#artworks

mod artwork;
mod collection;
mod manifest;
mod search;

pub use artwork::ArtworkInfo;
pub use collection::Artworks;
pub use manifest::Manifest;
pub use search::Search;

/// Modules for requesting items from the [Artworks Collection].
///
/// [Artworks Collection]: https://api.artic.edu/docs/#artworks
pub mod request {
    /// A [`GET /artworks/{id}`] request.
    ///
    /// [`GET /artworks/{id}`]: https://api.artic.edu/docs/#get-artworks-id
    pub mod artwork {
        pub use crate::artworks::artwork::Request;
    }

    /// A [`GET /artworks`] request.
    ///
    /// [`GET /artworks`]: https://api.artic.edu/docs/#get-artworks
    pub mod artworks {
        pub use crate::artworks::collection::{Builder, Request};
    }

    /// A [`GET /artworks/{id}/manifest.json`] request.
    ///
    /// [`GET /artworks/{id}/manifest.json`]: https://api.artic.edu/docs/#get-artworks-id-manifest-json
    pub mod manifest {
        pub use crate::artworks::manifest::Request;
    }

    /// A [`GET /artworks/search`] request.
    ///
    /// [`GET /artworks/search`]: https://api.artic.edu/docs/#get-artworks-search
    pub mod search {
        pub use crate::artworks::search::{Builder, Request};
    }
}
