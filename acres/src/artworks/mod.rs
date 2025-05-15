//! Artworks.

mod list;
mod list_op;
mod list_op_query_params;

pub use self::list::List;
pub use self::list_op::ListOp;

/// The [artworks collection].
///
/// [artworks collection]: https://api.artic.edu/docs/#artworks
#[derive(Clone, Debug, Default)]
pub struct ArtworksCollection {
    pub(crate) api: crate::Api,
}

impl ArtworksCollection {
    /// Returns an artworks collection list.
    pub fn list(&self) -> ListOp {
        ListOp::default().api(crate::Api {
            base_uri: self.api.base_uri.clone(),
            use_cache: self.api.use_cache,
        })
    }
}
