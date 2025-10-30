use crate::{
    Uri,
    errors::IiifError,
    image_request::{Format, ImageRequest, Quality, Region, Rotation, Size},
};

/// An IIIF builder.
#[derive(Debug, Default)]
pub struct ImageRequestBuilder {
    uri: Option<Uri>,
    region: Option<Region>,
    size: Option<Size>,
    rotation: Option<Rotation>,
    quality: Option<Quality>,
    format: Option<Format>,
}

impl ImageRequestBuilder {
    /// Artwork details.
    pub fn uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    /// Region of image to return.
    pub fn region(mut self, region: Option<Region>) -> Self {
        self.region = region;
        self
    }

    /// Size of the image to return.
    pub fn size(mut self, size: Option<Size>) -> Self {
        self.size = size;
        self
    }

    /// Rotation of the image to return.
    pub fn rotation(mut self, rotation: Option<Rotation>) -> Self {
        self.rotation = rotation;
        self
    }

    /// Quality of the image to return.
    pub fn quality(mut self, quality: Option<Quality>) -> Self {
        self.quality = quality;
        self
    }

    /// Format of the image to return.
    pub fn format(mut self, format: Option<Format>) -> Self {
        self.format = format;
        self
    }

    /// Build the IIIF instance.
    pub fn build(&self) -> Result<ImageRequest, IiifError> {
        tracing::info!(msg = "Building IIIF instance", ?self);

        // TODO: Remove default values from this package
        let uri = self.uri.clone();
        let region = self.region.as_ref().unwrap_or(&Region::Full);
        let size = self.size.as_ref().unwrap_or(&Size::Width(843));
        let rotation = self.rotation.as_ref().unwrap_or(&Rotation::Degrees(0.0));
        let quality = self.quality.as_ref().unwrap_or(&Quality::Default);
        let format = self.format.as_ref().unwrap_or(&Format::Jpg);

        Ok(ImageRequest::new(
            uri.unwrap().clone(),
            region.clone(),
            size.clone(),
            rotation.clone(),
            quality.clone(),
            format.clone(),
        ))
    }
}
