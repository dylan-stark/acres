#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple, ergonomic access for the [IIIF API 2.0](https://iiif.io/api/image/2.0/).
//!
//! You bring the URI and we make building the requests easy and error-proof.
//!
//! ```rust
//! # use anyhow::Result;
//! # use std::str::FromStr;
//! use iiif::{Degree, Format, Quality, Region, Rotation, Size, Uri};
//!
//! # fn main() -> Result<()> {
//! let uri: Uri = "https://example.org/image-service/abcd1234/1E34750D-38DB-4825-A38A-B60A345E591C".parse()?;
//!
//! let info_request = iiif::InformationRequest::new(uri.clone());
//! println!("Check out {info_request} for information about the image.");
//!
//! let image_request = iiif::ImageRequest::builder()
//!     .uri(uri)
//!     .region(Region::Full)
//!     .size(Size::Width(843))
//!     .rotation(Rotation::Degrees(Degree::default()))
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
//! use iiif::{ImageRequest};
//!
//! # fn main() -> Result<()> {
//! let image_request: ImageRequest = "https://example.org/image-service/abcd1234 \
//!     /1E34750D-38DB-4825-A38A-B60A345E591C \
//!     /full/843,/0/default.jpg".parse()?;
//! # Ok(())
//! # }
//! ```

mod errors;
mod image_request;
mod information_request;
mod uri;

pub use self::errors::IiifError;
pub use self::image_request::{
    Degree, Format, ImageRequest, Percentage, Quality, Region, Rotation, Size,
};
pub use self::information_request::InformationRequest;
pub use self::uri::Uri;

/// Used to indicate that a component not set for a partially-constructed value
#[derive(Debug, Clone, Copy)]
pub struct Unset;

/// Used to indicate that a component is set for a partially-constructed value
#[derive(Debug, Clone, Copy)]
pub struct Set;
