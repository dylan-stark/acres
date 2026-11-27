//! Errors, split by layer: [`BaseUriError`] for the base URI, [`ParameterError`] for image request
//! parameters, and [`RequestError`] for whole requests. [`Error`] wraps any of them.
//!
//! Variant names describe what is wrong with the input (see ADR 0006). A variant with a
//! `source()` leaves the source out of its own message, so error reporters that walk the chain
//! (`eyre`, `anyhow`) print each cause once.

use std::num::ParseFloatError;

/// Any IIIF error.
///
/// Each parser returns its own layer's error; this wraps them so callers that don't care which
/// layer failed can use one type with `?`. It displays as, and has the same `source()` as, the
/// error it wraps.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// The base URI is invalid.
    #[error(transparent)]
    InvalidBaseUri(#[from] BaseUriError),
    /// An image request parameter is invalid.
    #[error(transparent)]
    InvalidParameter(#[from] ParameterError),
    /// A request is invalid.
    #[error(transparent)]
    InvalidRequest(#[from] RequestError),
}

/// An invalid [base URI](crate::BaseUri). Each variant holds the offending input.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum BaseUriError {
    /// Not parseable as a URL at all.
    #[error("malformed URI: {input}")]
    Malformed {
        /// The offending input.
        input: String,
        /// Why it couldn't be parsed.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A character outside RFC 3986.
    #[error("invalid char in URI: {0}")]
    InvalidChar(String),
    /// A `%` not followed by two hex digits.
    #[error("invalid escape in URI: {0}")]
    InvalidEscape(String),
    /// A scheme other than `http` or `https`.
    #[error("invalid scheme: {0}")]
    InvalidScheme(String),
    /// Missing or extra slashes after the scheme.
    #[error("bad slashes in URI: {0}")]
    BadSlashes(String),
    /// User info (`user:pass@`) before the server.
    #[error("unexpected user info in URI: {0}")]
    UnexpectedUserInfo(String),
    /// No server.
    #[error("missing server in URI: {0}")]
    MissingServer(String),
    /// A prefix with an empty segment, an unencoded special character, or bad percent-encoding.
    #[error("invalid prefix in URI: {0}")]
    InvalidPrefix(String),
    /// An identifier with an unencoded special character, a dot segment, or bad
    /// percent-encoding.
    #[error("invalid identifier in URI: {0}")]
    InvalidIdentifier(String),
    /// No identifier.
    #[error("missing identifier in URI: {0}")]
    MissingIdentifier(String),
    /// A slash after the identifier.
    #[error("trailing slash in URI: {0}")]
    TrailingSlash(String),
    /// A `?query`.
    #[error("unexpected query in URI: {0}")]
    UnexpectedQuery(String),
    /// A `#fragment`.
    #[error("unexpected fragment in URI: {0}")]
    UnexpectedFragment(String),
}

/// An invalid image request parameter: [`Region`](crate::Region), [`Size`](crate::Size),
/// [`Rotation`](crate::Rotation), [`Quality`](crate::Quality), [`Format`](crate::Format),
/// [`Percentage`](crate::Percentage) or [`Degrees`](crate::Degrees).
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ParameterError {
    /// Not a region.
    #[error("invalid region: {0}")]
    InvalidRegion(String),
    /// Not a size.
    #[error("invalid size: {0}")]
    InvalidSize(String),
    /// Not a rotation.
    #[error("invalid rotation: {0}")]
    InvalidRotation(String),
    /// Not a quality.
    #[error("invalid quality: {0}")]
    InvalidQuality(String),
    /// Not a format.
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    /// Not a number.
    #[error("invalid percentage: {input}")]
    InvalidPercentage {
        /// The offending input.
        input: String,
        /// Why it isn't a number.
        #[source]
        source: ParseFloatError,
    },
    /// A number outside `0..=100`.
    #[error("percentage out of range: {0}")]
    PercentageOutOfRange(f32),
    /// Not a number.
    #[error("invalid degrees: {input}")]
    InvalidDegrees {
        /// The offending input.
        input: String,
        /// Why it isn't a number.
        #[source]
        source: ParseFloatError,
    },
    /// A number outside `0..=360`.
    #[error("degrees out of range: {0}")]
    DegreesOutOfRange(f32),
}

/// An invalid [`ImageRequest`](crate::ImageRequest) or
/// [`InformationRequest`](crate::InformationRequest).
///
/// The `Missing*` variants hold the offending input. The `Invalid*` variants say which part of
/// the request failed and hold that part's error as their `source()`.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum RequestError {
    /// The base URI is invalid.
    #[error("invalid base URI in request")]
    InvalidBaseUri(#[from] BaseUriError),
    /// An image request parameter is invalid.
    #[error("invalid parameter in image request")]
    InvalidParameter(#[from] ParameterError),
    /// No base URI before the parameters.
    #[error("missing base URI in request: {0}")]
    MissingBaseUri(String),
    /// No region.
    #[error("missing region in image request: {0}")]
    MissingRegion(String),
    /// No size.
    #[error("missing size in image request: {0}")]
    MissingSize(String),
    /// No rotation.
    #[error("missing rotation in image request: {0}")]
    MissingRotation(String),
    /// No `{quality}.{format}`.
    #[error("missing format in image request: {0}")]
    MissingFormat(String),
    /// No `/info.json` suffix.
    #[error("missing info.json in information request: {0}")]
    MissingInfoJson(String),
}

impl Error {
    /// A stable, dotted name for what is wrong, such as `iiif.base_uri.invalid_char`.
    ///
    /// Kinds are part of the API: one is never renamed or reused, so programs can match on it
    /// instead of on the message.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::InvalidBaseUri(e) => e.kind(),
            Error::InvalidParameter(e) => e.kind(),
            Error::InvalidRequest(e) => e.kind(),
        }
    }
}

impl BaseUriError {
    /// A stable, dotted name for what is wrong, such as `iiif.base_uri.unexpected_fragment`.
    ///
    /// Kinds are part of the API: one is never renamed or reused, so programs can match on it
    /// instead of on the message.
    pub fn kind(&self) -> &'static str {
        match self {
            BaseUriError::Malformed { .. } => "iiif.base_uri.malformed",
            BaseUriError::InvalidChar(_) => "iiif.base_uri.invalid_char",
            BaseUriError::InvalidEscape(_) => "iiif.base_uri.invalid_escape",
            BaseUriError::InvalidScheme(_) => "iiif.base_uri.invalid_scheme",
            BaseUriError::BadSlashes(_) => "iiif.base_uri.bad_slashes",
            BaseUriError::UnexpectedUserInfo(_) => "iiif.base_uri.unexpected_user_info",
            BaseUriError::MissingServer(_) => "iiif.base_uri.missing_server",
            BaseUriError::InvalidPrefix(_) => "iiif.base_uri.invalid_prefix",
            BaseUriError::InvalidIdentifier(_) => "iiif.base_uri.invalid_identifier",
            BaseUriError::MissingIdentifier(_) => "iiif.base_uri.missing_identifier",
            BaseUriError::TrailingSlash(_) => "iiif.base_uri.trailing_slash",
            BaseUriError::UnexpectedQuery(_) => "iiif.base_uri.unexpected_query",
            BaseUriError::UnexpectedFragment(_) => "iiif.base_uri.unexpected_fragment",
        }
    }
}

impl ParameterError {
    /// A stable, dotted name for what is wrong, such as `iiif.parameter.degrees_out_of_range`.
    ///
    /// Kinds are part of the API: one is never renamed or reused, so programs can match on it
    /// instead of on the message.
    pub fn kind(&self) -> &'static str {
        match self {
            ParameterError::InvalidRegion(_) => "iiif.parameter.invalid_region",
            ParameterError::InvalidSize(_) => "iiif.parameter.invalid_size",
            ParameterError::InvalidRotation(_) => "iiif.parameter.invalid_rotation",
            ParameterError::InvalidQuality(_) => "iiif.parameter.invalid_quality",
            ParameterError::InvalidFormat(_) => "iiif.parameter.invalid_format",
            ParameterError::InvalidPercentage { .. } => "iiif.parameter.invalid_percentage",
            ParameterError::PercentageOutOfRange(_) => "iiif.parameter.percentage_out_of_range",
            ParameterError::InvalidDegrees { .. } => "iiif.parameter.invalid_degrees",
            ParameterError::DegreesOutOfRange(_) => "iiif.parameter.degrees_out_of_range",
        }
    }
}

impl RequestError {
    /// A stable, dotted name for what is wrong, such as `iiif.request.missing_info_json`.
    ///
    /// Kinds are part of the API: one is never renamed or reused, so programs can match on it
    /// instead of on the message.
    pub fn kind(&self) -> &'static str {
        match self {
            RequestError::InvalidBaseUri(e) => e.kind(),
            RequestError::InvalidParameter(e) => e.kind(),
            RequestError::MissingBaseUri(_) => "iiif.request.missing_base_uri",
            RequestError::MissingRegion(_) => "iiif.request.missing_region",
            RequestError::MissingSize(_) => "iiif.request.missing_size",
            RequestError::MissingRotation(_) => "iiif.request.missing_rotation",
            RequestError::MissingFormat(_) => "iiif.request.missing_format",
            RequestError::MissingInfoJson(_) => "iiif.request.missing_info_json",
        }
    }
}

#[cfg(test)]
mod kind_tests {
    use crate::{BaseUri, ImageRequest, Region};

    #[test]
    fn kinds_name_the_layer_and_reach_through_wrappers() {
        let base = "https://example.org/a b".parse::<BaseUri>().unwrap_err();
        assert_eq!(base.kind(), "iiif.base_uri.invalid_char");
        assert_eq!(
            crate::Error::from(base).kind(),
            "iiif.base_uri.invalid_char"
        );

        let region = "nope".parse::<Region>().unwrap_err();
        assert_eq!(region.kind(), "iiif.parameter.invalid_region");

        let request = "https://example.org/images/1/full"
            .parse::<ImageRequest>()
            .unwrap_err();
        assert!(
            request.kind().starts_with("iiif.request."),
            "{}",
            request.kind()
        );
    }
}
