//! Errors, split by layer as in ADR 0006: [`RequestError`] for building a request,
//! [`ResponseError`] for talking to the API, [`CacheError`] for the on-disk cache and
//! [`ParseError`] for reading a response. [`Error`] wraps any of them.
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
}

/// A request that cannot be built. Nothing was sent.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum RequestError {
    /// The query parameters don't fit together, such as a `sort` without a `query`.
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    /// A value the API would reject, such as a `limit` over 100 or a page past 10,000 records.
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
    fn out_of_range_names_the_parameter() {
        let error = RequestError::OutOfRange {
            name: "limit",
            value: 101,
        };
        assert_eq!(error.to_string(), "limit out of range: 101");
    }
}
