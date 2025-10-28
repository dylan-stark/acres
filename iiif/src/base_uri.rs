use std::{fmt::Display, marker::PhantomData, str::FromStr};

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::errors::IiifError;

/// Supported URI schemes.
///
/// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
/// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
#[derive(
    Clone, Display, Default, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Scheme {
    /// HTTP protocol
    #[strum(to_string = "http")]
    Http,
    /// HTTPS protocol
    #[default]
    #[strum(to_string = "https")]
    Https,
}

impl Scheme {
    /// Creates a new default scheme.
    pub fn new() -> Self {
        Self::default()
    }
}

impl FromStr for Scheme {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == "https" => Ok(Scheme::Https),
            _ if s == "http" => Ok(Scheme::Http),
            _ => Err(IiifError::InvalidScheme(s.to_string())),
        }
    }
}

/// The base URI for an image.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaseUri {
    scheme: Scheme,
    server: String,
    prefix: String,
    identifier: String,
}

impl Display for BaseUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}://{}{}/{}",
            self.scheme,
            self.server.clone(),
            self.prefix.clone(),
            self.identifier.clone()
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Set;

#[derive(Debug, Clone, Copy)]
pub struct Unset;

impl BaseUri {
    /// Creates a image base URI builder.
    pub fn builder() -> Builder<Unset, Unset, Unset, Unset> {
        Builder::default()
    }
}

/// A base URI builder
#[derive(Debug)]
pub struct Builder<A, B, C, D> {
    base_uri: BaseUri,
    set: (
        PhantomData<A>,
        PhantomData<B>,
        PhantomData<C>,
        PhantomData<D>,
    ),
}

impl<A, B, C, D> Builder<A, B, C, D> {
    /// Sets the scheme for this URI.
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
    ///
    /// ```rust
    /// # use std::error::Error;
    /// #
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// iiif::BaseUri::builder().scheme(iiif::Scheme::Http);
    /// iiif::BaseUri::builder().scheme("http".parse()?);
    /// #   Ok(())
    /// # }
    /// ```
    pub fn scheme(mut self, scheme: Scheme) -> Builder<Set, B, C, D> {
        self.base_uri.scheme = scheme;
        Builder {
            base_uri: self.base_uri,
            set: (PhantomData::<Set>, self.set.1, self.set.2, self.set.3),
        }
    }

    /// Sets the server for this URI.
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "The host server on which the service resides. The parameter may also include a port number."
    ///
    /// ```
    /// iiif::BaseUri::builder().server("example.org");
    /// ```
    pub fn server(mut self, server: &str) -> Builder<A, Set, C, D> {
        self.base_uri.server = server.to_string();
        Builder {
            base_uri: self.base_uri,
            set: (self.set.0, PhantomData::<Set>, self.set.2, self.set.3),
        }
    }

    /// Sets the optional prefix for this URI.
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "The path on the host server to the service. This prefix is optional,
    /// > but may be useful when the host server supports multiple services.
    /// > The prefix may contain multiple path segments, delimited by slashes,
    /// > but all other special characters must be encoded. See URI Encoding
    /// > and Decoding for more information."
    ///
    /// ```
    /// iiif::BaseUri::builder().prefix("image-service");
    /// ```
    pub fn prefix(mut self, prefix: &str) -> Builder<A, B, Set, D> {
        if !prefix.is_empty() {
            self.base_uri.prefix = match prefix {
                _ if prefix.starts_with("/") => prefix.to_string(),
                _ => format!("/{}", prefix),
            }
        };
        Builder {
            base_uri: self.base_uri,
            set: (self.set.0, self.set.1, PhantomData::<Set>, self.set.3),
        }
    }

    /// Sets the identifier for this URI.
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "The identifier of the requested image. This may be an ARK, URN,
    /// > filename, or other identifier. Special characters must be URI encoded."
    ///
    /// # Examples
    ///
    /// ```
    /// iiif::BaseUri::builder().identifier("abcd1234");
    /// ```
    pub fn identifier(mut self, identifier: &str) -> Builder<A, B, C, Set> {
        self.base_uri.identifier = identifier.to_string();
        Builder {
            base_uri: self.base_uri,
            set: (self.set.0, self.set.1, self.set.2, PhantomData::<Set>),
        }
    }
}

impl Builder<Set, Set, Set, Set> {
    /// Builds the actual base URI for an image.
    pub fn build(self) -> BaseUri {
        self.base_uri
    }
}

impl Default for Builder<Unset, Unset, Unset, Unset> {
    fn default() -> Self {
        Self {
            base_uri: BaseUri::default(),
            set: (PhantomData, PhantomData, PhantomData, PhantomData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_uri_for_http() {
        assert_eq!(
            BaseUri::builder()
                .scheme(Scheme::Http)
                .server("example.org")
                .prefix("")
                .identifier("abcd1234")
                .build()
                .to_string(),
            "http://example.org/abcd1234"
        );
    }

    #[test]
    fn base_uri_with_prefix() {
        assert_eq!(
            BaseUri::builder()
                .scheme(Scheme::Https)
                .server("example.org")
                .prefix("image-service")
                .identifier("abcd1234")
                .build()
                .to_string(),
            "https://example.org/image-service/abcd1234"
        );
    }
}
