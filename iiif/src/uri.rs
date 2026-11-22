use std::{cell::RefCell, fmt::Display, str::FromStr};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use url::SyntaxViolation;

use crate::{IiifError::InvalidIdentifier, errors::IiifError};

const IDENTIFIER: &AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'%');

const PREFIX: &AsciiSet = &CONTROLS
    .add(b'/')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']')
    .add(b'@')
    .add(b'%');

/// Rejects anything outside RFC 3986: unreserved, reserved, and `%XX` escapes.
fn validate_rfc3986(s: &str) -> Result<(), IiifError> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let ok = bytes
                    .get(i + 1..i + 3)
                    .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit));
                if !ok {
                    return Err(IiifError::InvalidUriEscape(format!("at {}: {}", i, s)));
                }
                i += 3;
            }
            b if is_unreserved(b) || is_reserved(b) => i += 1,
            _ => return Err(IiifError::InvalidUriChar(format!("at {}: {}", i, s))),
        }
    }
    Ok(())
}

const fn is_unreserved(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~')
}

const fn is_reserved(b: u8) -> bool {
    matches!(
        b,
        b':' | b'/'
            | b'?'
            | b'#'
            | b'['
            | b']'
            | b'@'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
    )
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
enum Scheme {
    Http,
    Https,
}

impl Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scheme::Http => f.write_str("http"),
            Scheme::Https => f.write_str("https"),
        }
    }
}

impl FromStr for Scheme {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "http" | "HTTP" => Ok(Scheme::Http),
            "https" | "HTTPS" => Ok(Scheme::Https),
            _ => Err(IiifError::InvalidScheme(s.into())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Server {
    host: String,
    port: Option<u16>,
}

impl Display for Server {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(port) = self.port {
            f.write_str(format!("{}:{}", self.host, port).as_str())
        } else {
            f.write_str(self.host.as_str())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Prefix(Vec<String>);

impl Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encoded())
    }
}

impl FromStr for Prefix {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Prefix::from_encoded(s)
    }
}

impl Prefix {
    fn from_encoded(s: &str) -> Result<Self, IiifError> {
        let segments = s
            .split('/')
            .filter(|part| !part.is_empty())
            .map(|part| {
                percent_decode_str(part)
                    .decode_utf8()
                    .map(|c| c.into_owned())
                    .map_err(|e| IiifError::InvalidPrefix(format!("failed to decode: {}", e)))
            })
            .collect::<Result<Vec<String>, IiifError>>()?;
        Ok(Self(segments))
    }

    fn encoded(&self) -> String {
        let prefix = self
            .0
            .iter()
            .map(|part| utf8_percent_encode(part, PREFIX).to_string())
            .collect::<Vec<String>>()
            .join("/");
        if !prefix.is_empty() && !prefix.starts_with('/') {
            format!("/{}", prefix)
        } else {
            prefix
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Identifier(String);

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encoded())
    }
}

impl FromStr for Identifier {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Identifier::from_encoded(s)
    }
}

impl Identifier {
    fn from_encoded(s: &str) -> Result<Self, IiifError> {
        validate_rfc3986(s)?;
        let invalid_chars: Vec<char> = s
            .chars()
            .filter(|c| matches!(c, ' ' | '<' | '>' | '^' | '`' | '/' | '{' | '}' | '|'))
            .collect();
        if !invalid_chars.is_empty() {
            return Err(InvalidIdentifier(format!(
                "invalid characters ({:?}): {}",
                invalid_chars, s
            )));
        }
        if s.contains("%2E") || s.contains("%2e") {
            return Err(InvalidIdentifier(format!(
                "encoded '.' (%2E) not allowed: {}",
                s
            )));
        }

        let raw = percent_decode_str(s)
            .decode_utf8()
            .map_err(|e| IiifError::InvalidIdentifier(format!("failed to decode: {}", e)))?
            .into_owned();
        Ok(Self(raw))
    }

    fn encoded(&self) -> String {
        utf8_percent_encode(&self.0, IDENTIFIER).to_string()
    }
}

/// A [base URI] for an IIIF image API.
///
/// The base URI must use either Http or Https, include the host server, and provide the requested image identifier. It may also include a
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
    scheme: Scheme,
    server: Server,
    prefix: Prefix,
    identifier: Identifier,
}

impl FromStr for Uri {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_rfc3986(s)?;

        let saw_violations: RefCell<Vec<SyntaxViolation>> = RefCell::new(Vec::new());
        let url = url::Url::options()
            .syntax_violation_callback(Some(&|violation| match violation {
                SyntaxViolation::EmbeddedCredentials | SyntaxViolation::ExpectedDoubleSlash => {
                    saw_violations.borrow_mut().push(violation)
                }
                _ => (),
            }))
            .parse(s)
            .map_err(IiifError::InvalidUri)?;
        if saw_violations
            .borrow()
            .contains(&SyntaxViolation::ExpectedDoubleSlash)
        {
            // Detect missing or additional slashes after scheme. An extra
            // slash like in `https:///foo` is silently dropped by `Url::parse()`.
            return Err(IiifError::BadSlashes(s.into()));
        }
        if saw_violations
            .borrow()
            .contains(&SyntaxViolation::EmbeddedCredentials)
        {
            // Detect userinfo during parsing: username, password, and an
            // empty block like `https://@example.com` are all caught. The
            // latter is silently dropped by `Url::parse()` and can't be
            // caught with `username()` and `password()`.
            return Err(IiifError::InvalidUriWithUserInfo(s.into()));
        }

        let scheme: Scheme = url.scheme().parse()?;

        let host = url
            .host_str()
            .ok_or_else(|| IiifError::MissingServer(s.into()))?;
        let port = url.port();
        let server = Server {
            host: host.into(),
            port,
        };

        if url.path() == "/" {
            return Err(IiifError::MissingIdentifier(s.into()));
        }

        if url.path().ends_with('/') {
            return Err(IiifError::TrailingSlash(s.into()));
        }

        if url.query().is_some() {
            return Err(IiifError::HasQuery(s.into()));
        }

        if url.fragment().is_some() {
            return Err(IiifError::HasFragment(s.into()));
        }

        let mut segments = if let Some(segments) = url.path_segments() {
            segments.rev()
        } else {
            return Err(IiifError::MissingIdentifier(format!("not in path: {}", s)));
        };

        let encoded_identifier = segments
            .next()
            .ok_or_else(|| IiifError::MissingIdentifier(format!("not in path: {}", s)))?;
        let identifier: Identifier = Identifier::from_encoded(encoded_identifier)?;

        let encoded_prefix: Vec<&str> = segments.rev().collect();
        let prefix = Prefix::from_encoded(encoded_prefix.join("/").as_str())?;

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
