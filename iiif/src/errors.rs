use std::num::ParseFloatError;

/// An IIIF error.
#[derive(thiserror::Error, Debug)]
pub enum IiifError {
    /// Unable to parse quality.
    #[error("invalid quality: {0}")]
    InvalidQuality(String),
    /// Unable to parse scheme.
    #[error("invalid scheme: {0}")]
    InvalidScheme(String),
    /// Unable to parse format.
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    /// Unable to parse rotation.
    #[error("invalid rotation: {0}")]
    InvalidRotation(String),
    /// Unable to parse size.
    #[error("invalid size: {0}")]
    InvalidSize(String),
    /// Unable to parse region.
    #[error("invalid region: {0}")]
    InvalidRegion(String),
    /// Unable to parse percenage.
    #[error("invalid percentage: {0}")]
    InvalidPercentage(f32),
    /// Unable to parse degrees.
    #[error("invalid degree: {0}")]
    InvalidDegree(f32),
    /// Unusable percentage string.
    #[error("unable to parse percentage: {0}")]
    InvalidPercentageString(#[from] ParseFloatError),
    /// Format is missing.
    #[error("missing format in URL: {0}")]
    MissingFormat(String),
    /// Parameters are missing.
    #[error("missing parameters on URL: {0}")]
    MissingParams(String),
    /// Rotation is missing.
    #[error("missing rotation in URL: {0}")]
    MissingRotation(String),
    /// Size is missing.
    #[error("missing size in URL: {0}")]
    MissingSize(String),
    /// Region is missing.
    #[error("missing region in URL: {0}")]
    MissingRegion(String),
    /// Server is missing.
    #[error("missing server")]
    MissingServer,
    /// Identifier is missing.
    #[error("missing identifier in URL: {0}")]
    MissingIdentifier(String),
    /// Info part is missing.
    #[error("missing info part in URL: {0}")]
    MissingInfoPart(String),
    /// Unable to parse provided URI.
    #[error("unable to parse URI: {0}")]
    ParseUri(#[from] url::ParseError),
    /// Something unexpected went wrong.
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
