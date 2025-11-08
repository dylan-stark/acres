use std::fmt::Display;

use anyhow::Result;
use bytes::{Buf, Bytes};
use serde::Deserialize;

use crate::{AcresError, Api};

/// Artwork manifest from the AIC collection.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Manifest(serde_json::Value);

impl Display for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(&self.0).map_err(|_| std::fmt::Error)?;
        f.write_str(json.as_str())
    }
}

impl From<Bytes> for Manifest {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl Manifest {
    #[doc(hidden)]
    pub fn new(response: serde_json::Value) -> Self {
        Manifest(response)
    }

    /// Returns a new manifest builder.
    pub fn builder() -> ManifestBuilder {
        ManifestBuilder::default()
    }
}

/// An artwork builder.
#[derive(Debug, Default)]
pub struct ManifestBuilder {
    api: Api,
    id: u32,
}

impl ManifestBuilder {
    /// The artwork identifier.
    pub fn id(mut self, id: Option<u32>) -> Self {
        if let Some(id) = id {
            self.id = id;
        }
        self
    }

    /// Build the actual artwork.
    pub async fn build(&self) -> Result<Manifest, AcresError> {
        tracing::info!(msg = "Getting artwork manifest", ?self);
        let endpoint = format!("{}/artworks/{}/manifest", self.api.base_uri, self.id);
        // TODO: Clean up optional query params handling. Passing usize here is a hack.
        self.api.fetch::<Manifest>(endpoint, None::<usize>).await
    }
}
