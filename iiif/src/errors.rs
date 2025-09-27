use thiserror::Error;

/// An IIIF error.
#[derive(Error, Debug)]
pub enum IiifError {
    /// Unable to parse scheme.
    #[error("invalid scheme: {0}")]
    InvalidScheme(String),
    /// Something unexpected went wrong.
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
