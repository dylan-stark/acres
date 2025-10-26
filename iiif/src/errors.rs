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
    /// Something unexpected went wrong.
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
