//! IIIF information request.

use bytes::{Buf, Bytes};
use serde::Deserialize;
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

impl From<BaseUri> for InformationRequest {
    fn from(value: BaseUri) -> Self {
        InformationRequest { base_uri: value }
    }
}

/// An IIIF information request response.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct InformationResponse(serde_json::Value);

impl From<Bytes> for InformationResponse {
    fn from(value: Bytes) -> Self {
        let reader = value.reader();
        serde_json::from_reader(reader).unwrap()
    }
}

impl From<InformationResponse> for Bytes {
    fn from(value: InformationResponse) -> Self {
        Bytes::from(value.0.to_string())
    }
}
