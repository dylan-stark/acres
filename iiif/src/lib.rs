#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple, ergonomic access for working with [IIIF API 2.0](https://iiif.io/api/image/2.0/) services.
//!
//! You bring the URI and we make building the requests easy and error-proof.
//!
//! ```rust
//! # use anyhow::Result;
//! # use std::str::FromStr;
//! use iiif::{Degrees, Format, ImageRequest, InformationRequest, Quality, Region, Rotation, Size, BaseUri};
//!
//! # fn main() -> Result<()> {
//! let uri: BaseUri = "https://example.org/image-service/abcd1234/1E34750D-38DB-4825-A38A-B60A345E591C".parse()?;
//!
//! let info_request = InformationRequest::new(uri.clone());
//! println!("Check out {info_request} for information about the image.");
//!
//! let image_request = ImageRequest::builder()
//!     .base_uri(uri)
//!     .region(Region::Full)
//!     .size(Size::Width(843))
//!     .rotation(Rotation::Degrees(Degrees::default()))
//!     .quality(Quality::Default)
//!     .format(Format::Jpg)
//!     .build();
//! # Ok(())
//! # }
//! ```
//!
//! If you already have the full URL for an image request, you can use that, too.
//!
//! ```rust
//! # use anyhow::Result;
//! # use std::str::FromStr;
//! # use iiif::{ImageRequest, InformationRequest};
//! # fn main() -> Result<()> {
//! let info_request: InformationRequest = "https://example.org/image-service/abcd1234/1E34750D-38DB-4825-A38A-B60A345E591C/info.json".parse()?;
//!
//! let image_request: ImageRequest = "https://example.org/image-service/abcd1234\
//!     /1E34750D-38DB-4825-A38A-B60A345E591C\
//!     /full/843,/0/default.jpg".parse()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - `serde`: implements `Serialize` and `Deserialize` for the public types using their IIIF
//!   string form (e.g. a [`BaseUri`] serializes as `"https://example.org/images/12345"`).
//!   Deserializing validates exactly as [`str::parse`] does.

mod errors;
mod image_request;
mod information_request;
#[cfg(feature = "serde")]
mod serde_impls;
mod uri;

pub use self::errors::{BaseUriError, Error, ParameterError, RequestError};
pub use self::image_request::{
    Degrees, Format, ImageRequest, Percentage, Quality, Region, Rotation, Size,
};
pub use self::information_request::InformationRequest;
pub use self::uri::BaseUri;

#[doc(hidden)]
pub mod builder {

    /// Used to indicate that a component not set for a partially-constructed value
    #[derive(Debug, Clone, Copy)]
    pub struct Unset;

    /// Used to indicate that a component is set for a partially-constructed value
    #[derive(Debug, Clone, Copy)]
    pub struct Set;
}
