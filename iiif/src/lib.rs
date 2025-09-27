#![deny(missing_docs)]

//! Simple and ergonomic access to the IIIF APIs.

mod base_uri;
mod errors;
mod image_request;
mod image_request_builder;

pub use self::base_uri::{BaseUri, Scheme};
pub use self::image_request::{Format, ImageRequest, Quality, Region, Rotation, Size};
