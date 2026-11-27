//! Errors, split by layer as in ADR 0006: [`ConfigError`] for the `ACRES_*` environment,
//! [`RequestError`] for building a request, [`ResponseError`] for talking to the API,
//! [`CacheError`] for the on-disk cache and [`ParseError`] for reading a response. [`Error`]
//! wraps any of them.
//!
//! Every error has a stable [`kind`](Error::kind), such as `cache.missing`, for programs and
//! agents to match on, and some have a [`hint`](Error::hint) saying what to do next (see ADR 0019
//! and `docs/ERROR-HANDLING.md`).
//!
//! Variant names say what is wrong. Messages are lowercase with no trailing punctuation, and a
//! variant with a `source()` leaves the source out of its own message, so reporters that walk the
//! chain (`eyre`, `anyhow`) print each cause once. Third-party causes are boxed, so no dependency
//! type is part of the public API.

use std::path::PathBuf;

/// A boxed cause from another crate.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Any `acres` error.
///
/// Each layer returns its own error type; this wraps them so callers that don't care which layer
/// failed can use one type with `?`. It displays as, and has the same `source()` as, the error it
/// wraps.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// A request could not be built.
    #[error(transparent)]
    Request(#[from] RequestError),
    /// The API could not be reached, or answered with an error.
    #[error(transparent)]
    Response(#[from] ResponseError),
    /// The cache could not be read or written.
    #[error(transparent)]
    Cache(#[from] CacheError),
    /// A response could not be read.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// The `ACRES_*` environment could not be read.
    #[error(transparent)]
    Config(#[from] ConfigError),
}

impl Error {
    /// A stable, dotted name for what went wrong, such as `response.status` or `cache.missing`.
    ///
    /// Kinds are part of the API: a kind is never renamed or reused for something else, so
    /// programs can match on it instead of on the message.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Request(e) => e.kind(),
            Error::Response(e) => e.kind(),
            Error::Cache(e) => e.kind(),
            Error::Parse(e) => e.kind(),
            Error::Config(e) => e.kind(),
        }
    }

    /// What the caller can do about it, when there is something to say beyond the message.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Error::Request(e) => e.hint(),
            Error::Response(e) => e.hint(),
            Error::Cache(e) => e.hint(),
            Error::Parse(e) => e.hint(),
            Error::Config(e) => e.hint(),
        }
    }
}

/// The `ACRES_*` environment variables could not be read.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ConfigError {
    /// A variable holds a value of the wrong type, such as `ACRES_OFFLINE=maybe`, or a default
    /// directory is not valid UTF-8. Holds the reason, naming the setting.
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

impl ConfigError {
    /// See [`Error::kind`].
    pub fn kind(&self) -> &'static str {
        match self {
            ConfigError::Invalid(_) => "config.invalid",
        }
    }

    /// See [`Error::hint`].
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            ConfigError::Invalid(_) => Some(
                "check the ACRES_* environment variables: booleans are true or false, and \
                 directories must be valid UTF-8 paths",
            ),
        }
    }
}

/// A request that cannot be built. Nothing was sent.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum RequestError {
    /// The query parameters don't fit together, such as a `sort` without a `query`.
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    /// A value the API would reject, such as a `limit` over 100 or a page past 1,000 records.
    #[error("{name} out of range: {value}")]
    OutOfRange {
        /// The parameter, such as `limit`.
        name: &'static str,
        /// The offending value.
        value: u64,
    },
    /// An artwork's image URL could not be built from its IIIF base URL and image ID.
    #[error("invalid image URL for image {image_id}")]
    InvalidImageUrl {
        /// The image ID that was joined to the IIIF base URL.
        image_id: String,
        /// Why the join failed.
        #[source]
        source: BoxError,
    },
    /// A contact that can't go in an HTTP header, such as one with a newline.
    #[error("invalid contact: {0:?}")]
    InvalidContact(String),
    /// A IIIF URL was rejected by the `iiif` crate.
    #[error("invalid IIIF URL")]
    Iiif(#[from] iiif::Error),
}

impl RequestError {
    /// See [`Error::kind`]. A rejected IIIF URL takes the `iiif` crate's kind, such as
    /// `iiif.base_uri.invalid_char`.
    pub fn kind(&self) -> &'static str {
        match self {
            RequestError::InvalidQuery(_) => "request.invalid_query",
            RequestError::OutOfRange { .. } => "request.out_of_range",
            RequestError::InvalidImageUrl { .. } => "request.invalid_image_url",
            RequestError::InvalidContact(_) => "request.invalid_contact",
            RequestError::Iiif(e) => e.kind(),
        }
    }

    /// See [`Error::hint`].
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            RequestError::OutOfRange {
                name: "page" | "from",
                ..
            } => Some(
                "the API serves only the first 1,000 results of a search; narrow it with filters",
            ),
            RequestError::InvalidContact(_) => {
                Some("ACRES_CONTACT must fit on one line, such as an email address")
            }
            _ => None,
        }
    }
}

/// The API could not be reached, or answered with something other than success.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ResponseError {
    /// The API answered with an error status.
    #[error("{status} from {endpoint}: {error}: {detail}")]
    Status {
        /// The URL requested.
        endpoint: String,
        /// The HTTP status code.
        status: u16,
        /// The API's `error` field, or the status reason when the body has none.
        error: String,
        /// The API's `detail` field, or empty.
        detail: String,
    },
    /// The request waited in the queue past its maximum wait and was never sent.
    #[error("expired before it was sent: {endpoint}")]
    Expired {
        /// The URL that was not requested.
        endpoint: String,
    },
    /// The API still answered 429 Too Many Requests after every retry.
    #[error("too many requests: {endpoint}")]
    TooManyRequests {
        /// The URL requested.
        endpoint: String,
    },
    /// The request could not be sent or its body could not be read.
    #[error("transport failed: {endpoint}")]
    Transport {
        /// The URL requested.
        endpoint: String,
        /// The underlying HTTP error.
        #[source]
        source: BoxError,
    },
}

impl ResponseError {
    /// See [`Error::kind`]. An error status is `response.not_found` for 404 and
    /// `response.status` for any other; the status itself is in the message.
    pub fn kind(&self) -> &'static str {
        match self {
            ResponseError::Status { status: 404, .. } => "response.not_found",
            ResponseError::Status { .. } => "response.status",
            ResponseError::Expired { .. } => "response.expired",
            ResponseError::TooManyRequests { .. } => "response.too_many_requests",
            ResponseError::Transport { .. } => "response.transport",
        }
    }

    /// See [`Error::hint`].
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            ResponseError::Status { status: 404, .. } => Some("check the id"),
            ResponseError::Status { status: 403, .. } => Some(
                "the API refuses pages past the first 1,000 results of a search; narrow it with \
                 filters",
            ),
            ResponseError::Status { status, .. } if *status >= 500 => {
                Some("the API is having trouble; try again later")
            }
            ResponseError::Status { .. } => None,
            ResponseError::Expired { .. } => Some(
                "the shared rate limit (60 requests a minute) held the request too long; try \
                 again in a minute",
            ),
            ResponseError::TooManyRequests { .. } => {
                Some("the API is limiting requests; wait a minute and try again")
            }
            ResponseError::Transport { .. } => Some(
                "check the network connection, or work offline from the cache (ACRES_OFFLINE=true)",
            ),
        }
    }

    /// Whether the same request may succeed if tried again later.
    pub fn is_transient(&self) -> bool {
        match self {
            ResponseError::Status { status, .. } => *status == 429 || *status >= 500,
            ResponseError::Expired { .. }
            | ResponseError::TooManyRequests { .. }
            | ResponseError::Transport { .. } => true,
        }
    }
}

/// The cache could not answer.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum CacheError {
    /// Offline, and the cache has no entry for this URL.
    #[error("not cached while offline: {endpoint}")]
    Missing {
        /// The URL that was looked up.
        endpoint: String,
    },
    /// A cache file could not be read or written.
    #[error("cache file failed: {}", path.display())]
    Io {
        /// The cache file.
        path: PathBuf,
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl CacheError {
    /// See [`Error::kind`].
    pub fn kind(&self) -> &'static str {
        match self {
            CacheError::Missing { .. } => "cache.missing",
            CacheError::Io { .. } => "cache.io",
        }
    }

    /// See [`Error::hint`].
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            CacheError::Missing { .. } => {
                Some("go online (unset ACRES_OFFLINE, or drop --offline) to fetch it once")
            }
            CacheError::Io { .. } => Some(
                "check that the cache directory is writable; ACRES_CACHE_DIR moves it, and \
                 everything in it can be fetched again",
            ),
        }
    }
}

/// A response body that cannot be read as the expected type.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ParseError {
    /// The body is not the JSON shape expected from this input.
    #[error("invalid body from {input}")]
    InvalidBody {
        /// The URL it came from, or a short description of the input.
        input: String,
        /// Why it could not be parsed.
        #[source]
        source: BoxError,
    },
}

impl ParseError {
    /// See [`Error::kind`].
    pub fn kind(&self) -> &'static str {
        match self {
            ParseError::InvalidBody { .. } => "parse.invalid_body",
        }
    }

    /// See [`Error::hint`].
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            ParseError::InvalidBody { input, .. } if input.starts_with("http") => Some(concat!(
                "the API answered in a shape acres does not expect; please report it at ",
                env!("CARGO_PKG_REPOSITORY"),
                "/issues"
            )),
            ParseError::InvalidBody { .. } => {
                Some("check that the input is JSON as the API or acres-cli writes it")
            }
        }
    }

    /// Wraps a parse failure for `input`.
    pub(crate) fn body(input: impl Into<String>, source: impl Into<BoxError>) -> Self {
        Self::InvalidBody {
            input: input.into(),
            source: source.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn wrapper_displays_as_the_layer_error() {
        let error: Error = ResponseError::Expired {
            endpoint: "https://example.com/a".into(),
        }
        .into();
        assert_eq!(
            error.to_string(),
            "expired before it was sent: https://example.com/a"
        );
    }

    #[test]
    fn source_is_not_repeated_in_the_message() {
        let source = serde_json::from_str::<u8>("x").unwrap_err();
        let source_text = source.to_string();
        let error = ParseError::body("https://example.com/a", source);
        assert_eq!(error.to_string(), "invalid body from https://example.com/a");
        assert!(!error.to_string().contains(&source_text));
        assert_eq!(error.source().unwrap().to_string(), source_text);
    }

    #[test]
    fn status_message_holds_the_api_fields() {
        let error = ResponseError::Status {
            endpoint: "https://example.com/a".into(),
            status: 404,
            error: "Not found".into(),
            detail: "No such artwork".into(),
        };
        assert_eq!(
            error.to_string(),
            "404 from https://example.com/a: Not found: No such artwork"
        );
    }

    #[test]
    fn kinds_are_dotted_snake_case_and_reach_through_the_wrapper() {
        let errors: Vec<Error> = vec![
            ConfigError::Invalid("x".into()).into(),
            RequestError::InvalidQuery("x".into()).into(),
            RequestError::OutOfRange {
                name: "limit",
                value: 101,
            }
            .into(),
            RequestError::InvalidContact("x".into()).into(),
            ResponseError::Status {
                endpoint: "e".into(),
                status: 404,
                error: String::new(),
                detail: String::new(),
            }
            .into(),
            ResponseError::Status {
                endpoint: "e".into(),
                status: 500,
                error: String::new(),
                detail: String::new(),
            }
            .into(),
            ResponseError::Expired {
                endpoint: "e".into(),
            }
            .into(),
            ResponseError::TooManyRequests {
                endpoint: "e".into(),
            }
            .into(),
            CacheError::Missing {
                endpoint: "e".into(),
            }
            .into(),
            ParseError::body("e", "bad").into(),
        ];
        let kinds: Vec<&str> = errors.iter().map(Error::kind).collect();
        assert_eq!(
            kinds,
            [
                "config.invalid",
                "request.invalid_query",
                "request.out_of_range",
                "request.invalid_contact",
                "response.not_found",
                "response.status",
                "response.expired",
                "response.too_many_requests",
                "cache.missing",
                "parse.invalid_body",
            ]
        );
        for kind in kinds {
            assert!(
                kind.split('.').count() >= 2
                    && kind
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.'),
                "{kind}"
            );
        }
    }

    #[test]
    fn iiif_errors_keep_their_own_kind() {
        let iiif = "https://example.org/a b"
            .parse::<iiif::BaseUri>()
            .unwrap_err();
        let kind = iiif.kind();
        let error: Error = RequestError::Iiif(iiif.into()).into();
        assert!(kind.starts_with("iiif."), "{kind}");
        assert_eq!(error.kind(), kind);
    }

    #[test]
    fn hints_say_what_to_do_next() {
        let missing: Error = CacheError::Missing {
            endpoint: "e".into(),
        }
        .into();
        assert!(missing.hint().unwrap().contains("offline"));
        let forbidden = ResponseError::Status {
            endpoint: "e".into(),
            status: 403,
            error: String::new(),
            detail: String::new(),
        };
        assert!(forbidden.hint().unwrap().contains("1,000"));
        assert!(!forbidden.is_transient());
        assert!(
            ResponseError::TooManyRequests {
                endpoint: "e".into()
            }
            .is_transient()
        );
    }

    #[test]
    fn out_of_range_names_the_parameter() {
        let error = RequestError::OutOfRange {
            name: "limit",
            value: 101,
        };
        assert_eq!(error.to_string(), "limit out of range: 101");
    }
}
