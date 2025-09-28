#![deny(missing_docs)]

//! Simple and ergonomic access to the IIIF APIs.

mod base_uri;
mod errors;
mod image_request;
mod image_request_builder;
mod information_request;

pub use self::base_uri::{BaseUri, Scheme};
pub use self::image_request::{Format, ImageRequest, Quality, Region, Rotation, Size};
pub use self::information_request::InformationRequest;
