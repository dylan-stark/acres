pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error;

pub mod api;
pub mod iiif;
