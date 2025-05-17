//! Artworks.

mod list;
mod list_op;
mod list_op_query_params;

pub use self::list::List;
pub use self::list_op::ListOp;
use crate::api::Api;

/// The [artworks collection].
///
/// [artworks collection]: https://api.artic.edu/docs/#artworks
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArtworksCollection {
    pub(crate) api: Api,
}

impl ArtworksCollection {
    /// Returns an artworks collection list.
    pub fn list(&self) -> ListOp {
        tracing::info!(
            msg = "Creating default list op with API",
            self.api.base_uri,
            self.api.use_cache,
        );
        ListOp::default().api(Api {
            base_uri: self.api.base_uri.clone(),
            use_cache: self.api.use_cache,
        })
    }
}
