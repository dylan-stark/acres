use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::errors::IiifError;

/// A [base URI] for an IIIF image API.
///
/// The base URI must use either HTTP or HTTPS, include the host server, and provide the requested image identifier. It may also include a
/// path segement.
///
/// ```rust
/// # use anyhow::Result;
/// use std::str::FromStr;
/// use iiif::Uri;
///
/// # fn main() -> Result<()> {
/// let uri = Uri::from_str("https://example.org/images/12345")?;
/// let uri = Uri::from_str("https://example.org/12345")?;
/// let uri = "http://127.0.0.1:5555/some/path/to/12345".parse::<Uri>()?;
/// # Ok(())
/// # }
/// ```
///
/// If parsing fails, you'll get an [IiifError] that should help narrow down where the issue is and
/// how to correct it.
///
/// [Base URI]: https://iiif.io/api/image/3.0/#2-uri-syntax
/// [IiifError]: enum.IiifError
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Uri {
    scheme: String,
    server: String,
    prefix: String,
    identifier: String,
}

impl FromStr for Uri {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = url::Url::parse(s).map_err(IiifError::from)?;

        let scheme = url.scheme().to_string();
        let server = url.host_str().ok_or(IiifError::MissingServer)?.to_string();
        let server = if url.port().is_some() {
            format!("{}:{}", server, url.port().unwrap_or_default())
        } else {
            server
        };
        let mut path_segments = url
            .path_segments()
            .ok_or(IiifError::MissingIdentifier(s.into()))?
            .collect::<Vec<&str>>();
        let identifier = path_segments
            .pop()
            .take_if(|v| !v.is_empty())
            .ok_or(IiifError::MissingIdentifier(s.into()))?
            .to_string();
        tracing::debug!(path_segments = ?path_segments);
        let prefix = path_segments.join("/");
        tracing::debug!(prefix = %prefix);
        let prefix = if !prefix.is_empty() {
            format!("/{}", prefix)
        } else {
            prefix
        };

        Ok(Self {
            scheme,
            server,
            prefix,
            identifier,
        })
    }
}

impl Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}://{}{}/{}",
            self.scheme, self.server, self.prefix, self.identifier
        ))
    }
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("http://example.org/images/12345", "http://example.org/images/12345")]
    #[case("http://example.org/12345", "http://example.org/12345")]
    #[case(
        "http://127.0.0.1:80/some/path/to/12345",
        "http://127.0.0.1/some/path/to/12345" // Swallows default ports
    )]
    #[case("http://example.org:5555/12345", "http://example.org:5555/12345")]
    fn parse(#[case] s: &str, #[case] expected: &str) {
        assert_eq!(format!("{}", s.parse::<Uri>().unwrap()), expected);
    }

    #[rstest]
    // TODO: Test errors if missing host
    // TODO: Test errors if scheme not HTTP or HTTPS
    // TODO: Test errors if prefix is not empty and not URL dead_code
    #[case("", "unable to parse URI: relative URL without a base")]
    #[case("http:", "unable to parse URI: empty host")]
    #[case("http://", "unable to parse URI: empty host")]
    #[case("http://0.0.0.999", "unable to parse URI: invalid IPv4 address")]
    #[case(
        "http://example.com:999999999999",
        "unable to parse URI: invalid port number"
    )]
    fn parse_error(#[case] s: &str, #[case] expected: &str) {
        assert_eq!(format!("{}", s.parse::<Uri>().unwrap_err()), expected);
    }
}
