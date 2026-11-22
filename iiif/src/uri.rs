use std::{cell::RefCell, fmt::Display, str::FromStr};

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};
use url::SyntaxViolation;

use crate::{IiifError::InvalidIdentifier, errors::IiifError};

/// What `Display` percent-encodes in the prefix and identifier: IIIF §9 special characters,
/// plus ASCII that RFC 3986 doesn't allow raw anywhere. (Non-ASCII is always encoded.)
const TO_ENCODE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'\\')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
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

/// Whether an encoded path segment is `.` or `..`, with any dots percent-encoded (RFC 3986
/// §2.3, §6.2.2.3).
fn is_dot_segment(segment: &str) -> bool {
    matches!(
        segment.to_ascii_lowercase().replace("%2e", ".").as_str(),
        "." | ".."
    )
}

/// The last segment of the raw (un-normalised) path in `s`, if `s` has a path.
fn raw_last_path_segment(s: &str) -> Option<&str> {
    let (_, after_scheme) = s.split_once("://")?;
    let before_query = after_scheme.split(['?', '#']).next()?;
    let (_, path) = before_query.split_once('/')?;
    path.rsplit('/').next()
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

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
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

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
struct Prefix(Vec<String>);

impl Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.encoded())
    }
}

impl FromStr for Prefix {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self(Vec::new()));
        }
        Prefix::from_segments(&s.split('/').collect::<Vec<_>>())
    }
}

impl Prefix {
    /// Parses encoded prefix segments. Takes segments rather than a joined string so that a
    /// lone empty segment (from `https://host//id`) isn't mistaken for no prefix.
    fn from_segments(encoded: &[&str]) -> Result<Self, IiifError> {
        let s = encoded.join("/");
        let segments = encoded
            .iter()
            .map(|part| {
                // An empty segment comes from `//`. RFC 3986 treats `a//b` and `a/b` as different
                // paths, so reject rather than silently collapse.
                if part.is_empty() {
                    return Err(IiifError::InvalidUriPrefix(format!("empty segment: {}", s)));
                }
                // IIIF §2, §9: special characters must be encoded. `Url::parse` has already split
                // off `?` and `#`, and `/` delimits segments, but `[ ] @` are RFC 3986 reserved
                // so they get through to here raw.
                if let Some(c) = part.chars().find(|c| matches!(c, '[' | ']' | '@')) {
                    return Err(IiifError::InvalidUriPrefix(format!(
                        "unencoded '{}': {}",
                        c, s
                    )));
                }
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
            .map(|part| utf8_percent_encode(part, TO_ENCODE).to_string())
            .collect::<Vec<String>>()
            .join("/");
        if !prefix.is_empty() && !prefix.starts_with('/') {
            format!("/{}", prefix)
        } else {
            prefix
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
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
        // IIIF §3, §9: special characters must be encoded. `[ ] @` are RFC 3986 reserved, so
        // `validate_rfc3986` lets them through.
        if let Some(c) = s.chars().find(|c| matches!(c, '[' | ']' | '@')) {
            return Err(InvalidIdentifier(format!("unencoded '{}': {}", c, s)));
        }
        if is_dot_segment(s) {
            return Err(InvalidIdentifier(format!("dot segment: {}", s)));
        }

        let raw = percent_decode_str(s)
            .decode_utf8()
            .map_err(|e| IiifError::InvalidIdentifier(format!("failed to decode: {}", e)))?
            .into_owned();
        Ok(Self(raw))
    }

    fn encoded(&self) -> String {
        utf8_percent_encode(&self.0, TO_ENCODE).to_string()
    }
}

/// A [base URI] for an IIIF image API.
///
/// The base URI must use either Http or Https, include the host server, and provide the requested image identifier. It may also include a
/// path prefix.
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
/// If parsing fails, you'll get an [`IiifError`] that should help narrow down where the issue is and
/// how to correct it.
///
/// [base URI]: https://iiif.io/api/image/2.0/#uri-syntax
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

        // RFC 3986 §5.2.4: `Url::parse` removes a trailing `.` or `..` segment (encoded or not),
        // so check the raw input before that hides it behind a trailing-slash or
        // missing-identifier error.
        if raw_last_path_segment(s).is_some_and(is_dot_segment) {
            return Err(InvalidIdentifier(format!("dot segment: {}", s)));
        }

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
        let prefix = Prefix::from_segments(&encoded_prefix)?;

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
