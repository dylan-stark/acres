use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// A [`GET /artworks/{id}/manifest.json`] request.
///
/// ```rust
/// # use anyhow::Result;
/// use acres::artworks::request::manifest;
///
/// # fn main() -> Result<()> {
/// let request = manifest::Request::new(acres::DEFAULT_BASE_URI, 2);
/// # Ok(())
/// # }
/// ```
///
/// [`GET /artworks/{id}/manifest.json`]: https://api.artic.edu/docs/#get-artworks-id-manifest-json
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Request(String);

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Request {
    /// Constructs a new manifest request.
    pub fn new(base_uri: impl Into<String>, id: u32) -> Self {
        Request(format!("{}/artworks/{}/manifest", base_uri.into(), id))
    }
}
