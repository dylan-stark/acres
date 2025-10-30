#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

//! Simple, ergonomic access to the [IIIF APIs](https://iiif.io/api/image/3.0/).
//!
//! You bring the URI and we make building the requests easy and error-proof.
//!
//! ```rust,ignore
//! let uri = iiif::Uri::parse("https://example.org/image-service/abcd1234/1E34750D-38DB-4825-A38A-B60A345E591C")?;
//!
//! let info_request = iiif::InformationRequest::new(uri);
//! println!("Check out {info_request} for information about the image.")
//!
//! let image_request = iiif::ImageRequest::builder()
//!     .region(Some(Region::Full))
//!     .size(Some(Size::Width(843)))
//!     .rotation(Some(Rotation::Degrees(0.0)))
//!     .quality(Some(Quality::Default))
//!     .format(Some(Format::Jpg))
//!     .build();
//! ```

mod errors;
mod image_request;
mod image_request_builder;
mod information_request;
mod uri;

pub use self::errors::IiifError;
pub use self::image_request::{
    Format, ImageRequest, ImageResponse, Quality, Region, Rotation, Size,
};
pub use self::image_request_builder::ImageRequestBuilder;
pub use self::information_request::{InformationRequest, InformationResponse};
pub use self::uri::Uri;
