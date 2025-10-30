//! [IIIF information request](https://iiif.io/api/image/3.0/#51-image-information-request).
//!
//! - [Image Information](https://iiif.io/api/image/3.0/#5-image-information)

use bytes::{Buf, Bytes};
use serde::Deserialize;
use std::fmt::Display;

use crate::uri::Uri;

/// An IIIF instance.
#[derive(Clone, Debug, PartialEq)]
pub struct InformationRequest {
    uri: Uri,
}

impl InformationRequest {
    /// Create a new information request.
    pub fn new(uri: Uri) -> Self {
        InformationRequest { uri }
    }
}

impl Display for InformationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/info.json", self.uri)
    }
}

impl From<Uri> for InformationRequest {
    fn from(value: Uri) -> Self {
        InformationRequest { uri: value }
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
