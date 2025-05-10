#![deny(missing_docs)]

//! Simple and ergonomic access to the IIIF APIs.
//!
//! Create an API client and list artworks with

use std::fmt::Display;

use anyhow::anyhow;

/// A base URI builder
#[derive(Default)]
pub struct ImageBaseUriBuilder {
    scheme: Scheme,
    server: String,
    prefix: String,
    identifier: String,
}

/// Supported URI schemes.
///
/// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
/// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
#[derive(Debug)]
pub enum Scheme {
    /// HTTP protocol
    Http,
    /// HTTPS protocol
    Https,
}

impl Default for Scheme {
    fn default() -> Self {
        Self::Https
    }
}

impl Display for Scheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let scheme = match self {
            Scheme::Https => "https",
            Scheme::Http => "http",
        };
        write!(f, "{}", scheme)
    }
}

impl ImageBaseUriBuilder {
    /// Sets the scheme for this URI (defaults to HTTPS).
    ///
    /// According to <https://iiif.io/api/image/3.0/#2-uri-syntax>:
    /// > "Indicates the use of the HTTP or HTTPS protocol in calling the service."
    ///
    /// ```
    /// iiif::ImageBaseUri::builder().scheme(iiif::Scheme::Http);
    /// iiif::ImageBaseUri::builder().scheme(iiif::Scheme::Https);
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
    /// iiif::ImageBaseUri::builder().server("example.org");
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
    /// iiif::ImageBaseUri::builder().prefix("image-service");
    /// ```
    pub fn prefix(mut self, prefix: &str) -> Self {
        if !prefix.is_empty() {
            self.prefix = format!("/{}", prefix)
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
    /// iiif::ImageBaseUri::builder().identifier("abcd1234");
    /// ```
    pub fn identifier(mut self, identifier: &str) -> Self {
        self.identifier = identifier.to_string();
        self
    }

    /// Builds the actual base URI for an image.
    pub fn build(self) -> anyhow::Result<ImageBaseUri> {
        if self.server.is_empty() {
            return Err(anyhow!("Missing server"));
        }
        if self.identifier.is_empty() {
            return Err(anyhow!("Missing identifier"));
        }

        Ok(ImageBaseUri {
            scheme: self.scheme,
            server: self.server,
            prefix: self.prefix,
            identifier: self.identifier,
        })
    }
}

/// The base URI for an image.
#[derive(Debug)]
pub struct ImageBaseUri {
    scheme: Scheme,
    server: String,
    prefix: String,
    identifier: String,
}

impl ImageBaseUri {
    /// Creates a image base URI builder.
    pub fn builder() -> ImageBaseUriBuilder {
        ImageBaseUriBuilder::default()
    }
}

impl Display for ImageBaseUri {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_uri_with_defaults() {
        assert_eq!(
            ImageBaseUri::builder()
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
            ImageBaseUri::builder()
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
            ImageBaseUri::builder()
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
        let result = ImageBaseUri::builder().identifier("abcd1234").build();

        assert_eq!(format!("{}", result.unwrap_err()), "Missing server");
    }

    #[test]
    fn base_uri_missing_identifier() {
        let result = ImageBaseUri::builder().server("example.org").build();

        assert_eq!(format!("{}", result.unwrap_err()), "Missing identifier");
    }
}
