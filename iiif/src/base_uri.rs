use std::fmt::Display;

use strum::Display;

use crate::errors::IiifError;

/// Supported URI schemes.
///
/// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
/// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
#[derive(Clone, Display, Default, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
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

    /// Scheme parser.
    pub fn parse(value: &str) -> Result<Scheme, IiifError> {
        match value {
            _ if value == "https" => Ok(Scheme::Https),
            _ if value == "http" => Ok(Scheme::Http),
            _ => Err(IiifError::InvalidScheme(value.to_string())),
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

impl BaseUri {
    /// Creates a image base URI builder.
    pub fn builder() -> BaseUriBuilder {
        BaseUriBuilder::default()
    }
}

/// A base URI builder
#[derive(Debug, Default)]
pub struct BaseUriBuilder {
    scheme: Scheme,
    server: String,
    prefix: String,
    identifier: String,
}

impl BaseUriBuilder {
    /// Sets the scheme for this URI (defaults to HTTPS).
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
    ///
    /// ```
    /// iiif::BaseUri::builder().scheme(iiif::Scheme::Http);
    /// iiif::BaseUri::builder().scheme(iiif::Scheme::Https);
    /// ```
    pub fn scheme(mut self, scheme: Scheme) -> Self {
        self.scheme = scheme;
        self
    }

    /// Sets the server for this URI.
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "The host server on which the service resides. The parameter may also include a port number."
    ///
    /// ```
    /// iiif::BaseUri::builder().server("example.org");
    /// ```
    pub fn server(mut self, server: &str) -> Self {
        self.server = server.to_string();
        self
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
    pub fn prefix(mut self, prefix: &str) -> Self {
        if !prefix.is_empty() {
            self.prefix = match prefix {
                _ if prefix.starts_with("/") => prefix.to_string(),
                _ => format!("/{}", prefix),
            }
        };
        self
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
    pub fn identifier(mut self, identifier: &str) -> Self {
        self.identifier = identifier.to_string();
        self
    }

    /// Builds the actual base URI for an image.
    pub fn build(self) -> Result<BaseUri, IiifError> {
        if self.server.is_empty() {
            return Err(IiifError::MissingServer);
        }
        if self.identifier.is_empty() {
            return Err(IiifError::MissingIdentifier);
        }

        Ok(BaseUri {
            scheme: self.scheme,
            server: self.server,
            prefix: self.prefix,
            identifier: self.identifier,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_uri_with_defaults() {
        assert_eq!(
            BaseUri::builder()
                .server("example.org")
                .identifier("abcd1234")
                .build()
                .unwrap()
                .to_string(),
            "https://example.org/abcd1234"
        );
    }

    #[test]
    fn base_uri_for_http() {
        assert_eq!(
            BaseUri::builder()
                .scheme(Scheme::Http)
                .server("example.org")
                .identifier("abcd1234")
                .build()
                .unwrap()
                .to_string(),
            "http://example.org/abcd1234"
        );
    }

    #[test]
    fn base_uri_with_prefix() {
        assert_eq!(
            BaseUri::builder()
                .server("example.org")
                .prefix("image-service")
                .identifier("abcd1234")
                .build()
                .unwrap()
                .to_string(),
            "https://example.org/image-service/abcd1234"
        );
    }

    #[test]
    fn base_uri_missing_server() {
        let result = BaseUri::builder().identifier("abcd1234").build();

        assert_eq!(format!("{}", result.unwrap_err()), "missing server");
    }

    #[test]
    fn base_uri_missing_identifier() {
        let result = BaseUri::builder().server("example.org").build();

        assert_eq!(format!("{}", result.unwrap_err()), "missing identifier");
    }
}
