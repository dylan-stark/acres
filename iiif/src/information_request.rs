use std::{fmt::Display, str::FromStr};

use crate::{RequestError, uri::BaseUri};

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
/// use iiif::BaseUri;
///
/// # fn main() -> Result<()> {
/// let uri: BaseUri = "https://example.org/images/12345".parse()?;
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
/// [information request]: https://iiif.io/api/image/2.0/#information-request
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InformationRequest {
    uri: BaseUri,
}

impl InformationRequest {
    /// Create a new information request.
    pub fn new(uri: BaseUri) -> Self {
        InformationRequest { uri }
    }

    /// The base URI of the image service.
    pub fn base_uri(&self) -> &BaseUri {
        &self.uri
    }
}

impl Display for InformationRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/info.json", self.uri)
    }
}

impl FromStr for InformationRequest {
    type Err = RequestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(uri) = s.strip_suffix("/info.json") {
            let uri = BaseUri::from_str(uri)?;
            Ok(InformationRequest::from(uri))
        } else {
            Err(RequestError::MissingInfoJson(s.to_string()))
        }
    }
}

impl From<BaseUri> for InformationRequest {
    fn from(value: BaseUri) -> Self {
        InformationRequest { uri: value }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("https://example.org/images/12345/info.json")]
    #[case("http://127.0.0.1:5555/some/path/a%2Fb/info.json")]
    fn uri_is_the_base(#[case] url: &str) {
        let request = InformationRequest::from_str(url).unwrap();
        assert_eq!(
            request.base_uri().to_string(),
            url.strip_suffix("/info.json").unwrap()
        );
    }
}
