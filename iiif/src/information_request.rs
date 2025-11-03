use std::{fmt::Display, str::FromStr};

use crate::{IiifError, uri::Uri};

/// Defines an [information request] for the IIIF Image API 2.0.
///
/// You can create a new information request with a URI.
///
/// ```rust
/// # use anyhow::Result;
/// use iiif::InformationRequest;
///
/// # fn main() -> Result<()> {
/// let info_request = InformationRequest::new("https://example.org/images/12345".parse()?);
/// assert_eq!(info_request.to_string(), "https://example.org/images/12345/info.json");
/// # Ok(())
/// # }
/// ```
///
/// If you already have a URI, then you can construct one directly from that.
///
/// ```rust
/// # use anyhow::Result;
/// # use iiif::InformationRequest;
/// use iiif::Uri;
///
/// # fn main() -> Result<()> {
/// let uri: Uri = "https://example.org/images/12345".parse()?;
/// let info_request: InformationRequest = uri.into();
/// # assert_eq!(info_request.to_string(), "https://example.org/images/12345/info.json");
/// # Ok(())
/// # }
/// ```
///
/// And if you have a string and want an [`InformationRequest`], you can use that, too.
///
/// ```rust
/// # use anyhow::Result;
/// # use iiif::InformationRequest;
/// #
/// # fn main() -> Result<()> {
/// let info_request: InformationRequest = "https://example.org/images/12345/info.json".parse()?;
/// # assert_eq!(info_request.to_string(), "https://example.org/images/12345/info.json");
/// # Ok(())
/// # }
/// ```
///
/// [`InformationRequest`]: struct.InformationRequest.html
/// [information request]: https://iiif.io/api/image/2.0/#information-request
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl FromStr for InformationRequest {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(uri) = s.strip_suffix("/info.json") {
            let uri = Uri::from_str(uri)?;
            Ok(InformationRequest::from(uri))
        } else {
            Err(IiifError::MissingInfoPart(s.to_string()))
        }
    }
}

impl From<Uri> for InformationRequest {
    fn from(value: Uri) -> Self {
        InformationRequest { uri: value }
    }
}
