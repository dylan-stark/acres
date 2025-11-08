use std::fmt::Display;

use bytes::{Buf, Bytes};
use serde::{Deserialize, Serialize};

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
}

/// A manifest request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Request(String);

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Request {
    /// Constructs a new manifest request.
    pub fn new(base_uri: String, id: u32) -> Self {
        Request(format!("{}/artworks/{}/manifest", base_uri, id))
    }
}
