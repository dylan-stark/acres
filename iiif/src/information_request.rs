//! IIIF information request.

use std::fmt::Display;

use crate::base_uri::BaseUri;

/// An IIIF instance.
#[derive(Clone, Debug, PartialEq)]
pub struct InformationRequest {
    base_uri: BaseUri,
}

impl InformationRequest {
    /// Create a new information request.
    pub fn new(base_uri: BaseUri) -> Self {
        InformationRequest { base_uri }
    }
}

impl Display for InformationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/info.json", self.base_uri)
    }
}
