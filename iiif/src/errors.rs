/// An IIIF error.
#[derive(thiserror::Error, Debug)]
pub enum IiifError {
    /// Unable to parse scheme.
    #[error("invalid scheme: {0}")]
    InvalidScheme(String),
    /// Server is missing.
    #[error("missing server")]
    MissingServer,
    /// Identifier is missing.
    #[error("missing identifier")]
    MissingIdentifier,
    /// Unable to parse provided URI.
    #[error("unable to parse URI: {0}")]
    ParseUri(#[from] url::ParseError),
    /// Something unexpected went wrong.
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
