use std::fmt::Display;

use super::artwork_builder::ArtworkBuilder;

/// Artwork from the AIC collection.
#[derive(Clone, Debug, PartialEq)]
pub struct Artwork(serde_json::Value);

impl Display for Artwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl Artwork {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Artwork(response)
    }

    /// Creates an artwork builder.
    pub fn builder() -> ArtworkBuilder {
        ArtworkBuilder::default()
    }
}
