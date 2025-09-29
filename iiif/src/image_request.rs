//! IIIF.

use std::fmt::Display;

use bytes::Bytes;

use crate::{base_uri::BaseUri, image_request_builder::ImageRequestBuilder};

/// An IIIF instance.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageRequest {
    base_uri: BaseUri,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl ImageRequest {
    /// Create a new image request.
    pub fn new(
        base_uri: BaseUri,
        region: Region,
        size: Size,
        rotation: Rotation,
        quality: Quality,
        format: Format,
    ) -> Self {
        ImageRequest {
            base_uri,
            region,
            size,
            rotation,
            quality,
            format,
        }
    }
}

impl Display for ImageRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{}.{}",
            self.base_uri, self.region, self.size, self.rotation, self.quality, self.format
        )
    }
}

impl ImageRequest {
    /// Returns a new builder.
    pub fn builder() -> ImageRequestBuilder {
        ImageRequestBuilder::default()
    }
}

/// An IIIF image request response.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageResponse(Bytes);

impl From<Bytes> for ImageResponse {
    fn from(value: Bytes) -> Self {
        ImageResponse(value)
    }
}

impl From<ImageResponse> for Bytes {
    fn from(value: ImageResponse) -> Self {
        value.0
    }
}

/// Region of an image.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Region {
    /// The complete image.
    #[default]
    Full,
    /// Region expressed in absolute pixel values.
    Absolute(u32, u32, u32, u32),
    /// Region expressed in percent of image's dimensions.
    Percentage(f32, f32, f32, f32),
}

impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Full => write!(f, "full"),
            Region::Absolute(x, y, w, h) => write!(f, "{},{},{},{}", x, y, w, h),
            Region::Percentage(x, y, w, h) => write!(f, "pct:{},{},{},{}", x, y, w, h),
        }
    }
}

impl Region {
    /// Region parser.
    pub fn parse(value: &str) -> Result<Region, String> {
        if value == "full" {
            return Ok(Region::Full);
        }

        let is_pct = value.starts_with("pct:");
        let xywh = if is_pct {
            value.replacen("pct:", "", 1)
        } else {
            value.to_string()
        };

        let parts = xywh.split(",");
        if is_pct {
            let parts: Vec<f32> = parts
                .filter_map(|part| part.trim().parse::<f32>().ok())
                .collect();
            if parts.len() == 4 {
                return Ok(Region::Percentage(parts[0], parts[1], parts[2], parts[3]));
            }
        } else {
            let parts: Vec<u32> = parts
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .collect();
            if parts.len() == 4 {
                return Ok(Region::Absolute(parts[0], parts[1], parts[2], parts[3]));
            }
        }

        Err(format!(
            "could not understand region specification: {}",
            value
        ))
    }
}

/// Size to scale to.
#[derive(Clone, Debug, PartialEq)]
pub enum Size {
    /// Don't scale.
    Full,
    /// Scale width to this many pixels.
    Width(u32),
    /// Scale height to this many pixels.
    Height(u32),
    /// Scale height and width by this percent.
    Percentage(f32),
    /// Scale width and height to exactly this.
    Exactly(u32, u32),
    /// Scale width and height to best fit.
    BestFit(u32, u32),
}

impl Default for Size {
    fn default() -> Self {
        Size::Width(843)
    }
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Size::Full => write!(f, "full"),
            Size::Width(width) => write!(f, "{},", width),
            Size::Height(height) => write!(f, ",{}", height),
            Size::Percentage(percentage) => write!(f, "pct:{}", percentage),
            Size::Exactly(width, height) => write!(f, "{},{}", width, height),
            Size::BestFit(width, height) => write!(f, "!{},{}", width, height),
        }
    }
}

impl Size {
    /// Size parser.
    pub fn parse(value: &str) -> Result<Size, String> {
        if value == "full" {
            return Ok(Size::Full);
        } else if value.starts_with("pct:") {
            let n = value.replacen("pct:", "", 1).parse::<f32>().ok();
            if let Some(n) = n {
                return Ok(Size::Percentage(n));
            }
        } else if value.starts_with("!") {
            let parts = value
                .replacen("!", "", 1)
                .split(",")
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .collect::<Vec<u32>>();
            if parts.len() == 2 {
                return Ok(Size::BestFit(parts[0], parts[1]));
            }
        } else if value.starts_with(",") {
            let height = value.replacen(",", "", 1).parse::<u32>().ok();
            if let Some(height) = height {
                return Ok(Size::Height(height));
            }
        } else if value.ends_with(",") {
            let width = value.replacen(",", "", 1).parse::<u32>().ok();
            if let Some(width) = width {
                return Ok(Size::Width(width));
            }
        } else {
            let parts = value
                .split(",")
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .collect::<Vec<u32>>();
            if parts.len() == 2 {
                return Ok(Size::Exactly(parts[0], parts[1]));
            }
        }

        Err(format!(
            "could not understand size specification: {}",
            value
        ))
    }
}

/// Amount to rotate by.
#[derive(Clone, Debug, PartialEq)]
pub enum Rotation {
    /// Clockwise rotation in degrees.
    Degrees(f32),
    /// Mirrored rotation in degrees.
    Mirrored(f32),
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Degrees(0.0)
    }
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rotation::Degrees(degrees) => write!(f, "{}", degrees),
            Rotation::Mirrored(degrees) => write!(f, "!{}", degrees),
        }
    }
}

impl Rotation {
    /// Rotation parser.
    pub fn parse(value: &str) -> Result<Rotation, String> {
        if let Ok(degrees) = value.parse::<f32>() {
            return Ok(Rotation::Degrees(degrees));
        }
        if let Ok(degrees) = value.replacen("!", "", 1).parse::<f32>() {
            return Ok(Rotation::Mirrored(degrees));
        }

        Err(format!(
            "could not understand rotation specification: {}",
            value
        ))
    }
}

/// Image quality.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Quality {
    /// Full color.
    Color,
    /// Grayscale.
    Gray,
    /// Black-and-white.
    Bitonal,
    /// Server's choice.
    #[default]
    Default,
}

impl Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quality::Color => write!(f, "color"),
            Quality::Gray => write!(f, "gray"),
            Quality::Bitonal => write!(f, "bitonal"),
            Quality::Default => write!(f, "default"),
        }
    }
}

impl Quality {
    /// Quality parser.
    pub fn parse(value: &str) -> Result<Quality, String> {
        match value {
            _ if value == "color" => Ok(Quality::Color),
            _ if value == "gray" => Ok(Quality::Gray),
            _ if value == "bitonal" => Ok(Quality::Bitonal),
            _ if value == "default" => Ok(Quality::Default),
            _ => Err(format!(
                "could not understand quality specification: {}",
                value
            )),
        }
    }
}

/// Image format.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Format {
    /// JPEG format.
    #[default]
    Jpg,
    /// TIF format.
    Tif,
    /// PNG format.
    Png,
    /// GIF format.
    Gif,
    /// JP2 format.
    Jp2,
    /// PDF format.
    Pdf,
    /// WebP format.
    WebP,
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Jpg => write!(f, "jpg"),
            Format::Tif => write!(f, "tif"),
            Format::Png => write!(f, "png"),
            Format::Gif => write!(f, "gif"),
            Format::Jp2 => write!(f, "jp2"),
            Format::Pdf => write!(f, "pdf"),
            Format::WebP => write!(f, "webp"),
        }
    }
}

impl Format {
    /// Parse format from string.
    pub fn parse(value: &str) -> Result<Format, String> {
        match value {
            _ if value == "jpg" => Ok(Format::Jpg),
            _ if value == "tif" => Ok(Format::Tif),
            _ if value == "png" => Ok(Format::Png),
            _ if value == "gif" => Ok(Format::Gif),
            _ if value == "jp2" => Ok(Format::Jp2),
            _ if value == "pdf" => Ok(Format::Pdf),
            _ if value == "webp" => Ok(Format::WebP),
            _ => Err(format!(
                "could not understand format specification: {}",
                value
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert_eq!(Region::default().to_string(), "full");
        assert_eq!(Size::default().to_string(), "843,");
        assert_eq!(Rotation::default().to_string(), "0");
        assert_eq!(Quality::default().to_string(), "default");
        assert_eq!(Format::default().to_string(), "jpg");
    }

    #[test]
    fn region_parsing() {
        assert_eq!(Region::parse("full").unwrap(), Region::Full);
        assert_eq!(
            Region::parse("ful").unwrap_err(),
            "could not understand region specification: ful"
        );
        assert_eq!(
            Region::parse("Full").unwrap_err(),
            "could not understand region specification: Full"
        );

        assert_eq!(
            Region::parse("1,2,3,4").unwrap(),
            Region::Absolute(1, 2, 3, 4)
        );
        assert_eq!(
            Region::parse("1,2,4").unwrap_err(),
            "could not understand region specification: 1,2,4"
        );
        assert_eq!(
            Region::parse("1,2,,4").unwrap_err(),
            "could not understand region specification: 1,2,,4"
        );
        assert_eq!(
            Region::parse("1,2,a,4").unwrap_err(),
            "could not understand region specification: 1,2,a,4"
        );
        assert_eq!(
            Region::parse("1,2,3,4,5").unwrap_err(),
            "could not understand region specification: 1,2,3,4,5"
        );

        assert_eq!(
            Region::parse("pct:1,2,3,4").unwrap(),
            Region::Percentage(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            Region::parse("pct:1.2,2.37,3.4,4.513").unwrap(),
            Region::Percentage(1.2, 2.37, 3.4, 4.513)
        );
        assert_eq!(
            Region::parse("pct:1.2,3.4,4.513").unwrap_err(),
            "could not understand region specification: pct:1.2,3.4,4.513"
        );
    }

    #[test]
    fn size_parsing() {
        assert_eq!(Size::parse("full").unwrap(), Size::Full);
        assert_eq!(
            Size::parse("FULL").unwrap_err(),
            "could not understand size specification: FULL"
        );

        assert_eq!(Size::parse("42,").unwrap(), Size::Width(42));
        assert_eq!(
            Size::parse("42.0,").unwrap_err(),
            "could not understand size specification: 42.0,"
        );

        assert_eq!(Size::parse(",42").unwrap(), Size::Height(42));
        assert_eq!(
            Size::parse(",42.0").unwrap_err(),
            "could not understand size specification: ,42.0"
        );

        assert_eq!(Size::parse("pct:42").unwrap(), Size::Percentage(42.));
        assert_eq!(Size::parse("pct:42.3").unwrap(), Size::Percentage(42.3));
        assert_eq!(
            Size::parse("pct:").unwrap_err(),
            "could not understand size specification: pct:"
        );

        assert_eq!(Size::parse("42,24").unwrap(), Size::Exactly(42, 24));
        assert_eq!(
            Size::parse("42.0,24.0").unwrap_err(),
            "could not understand size specification: 42.0,24.0"
        );

        assert_eq!(Size::parse("!42,24").unwrap(), Size::BestFit(42, 24));
        assert_eq!(
            Size::parse("!42.0,24.0").unwrap_err(),
            "could not understand size specification: !42.0,24.0"
        );
    }

    #[test]
    fn rotation_parsing() {
        assert_eq!(Rotation::parse("42").unwrap(), Rotation::Degrees(42.0));
        assert_eq!(Rotation::parse("42").unwrap().to_string(), "42");
        assert_eq!(Rotation::parse("42.24").unwrap(), Rotation::Degrees(42.24));
        assert_eq!(Rotation::parse("42.24").unwrap().to_string(), "42.24");
        assert_eq!(
            Rotation::parse("forty-two").unwrap_err(),
            "could not understand rotation specification: forty-two"
        );

        assert_eq!(Rotation::parse("!42").unwrap(), Rotation::Mirrored(42.0));
        assert_eq!(Rotation::parse("!42").unwrap().to_string(), "!42");
        assert_eq!(
            Rotation::parse("!42.24").unwrap(),
            Rotation::Mirrored(42.24)
        );
        assert_eq!(Rotation::parse("!42.24").unwrap().to_string(), "!42.24");
        assert_eq!(
            Rotation::parse("").unwrap_err(),
            "could not understand rotation specification: "
        );
    }

    #[test]
    fn quality_parsing() {
        assert_eq!(Quality::parse("color").unwrap(), Quality::Color);
        assert_eq!(Quality::parse("gray").unwrap(), Quality::Gray);
        assert_eq!(Quality::parse("bitonal").unwrap(), Quality::Bitonal);
        assert_eq!(Quality::parse("default").unwrap(), Quality::Default);
        assert_eq!(
            Quality::parse("11").unwrap_err(),
            "could not understand quality specification: 11"
        );
    }

    #[test]
    fn format_parsing() {
        assert_eq!(Format::parse("jpg").unwrap(), Format::Jpg);
        assert_eq!(Format::parse("tif").unwrap(), Format::Tif);
        assert_eq!(Format::parse("png").unwrap(), Format::Png);
        assert_eq!(Format::parse("gif").unwrap(), Format::Gif);
        assert_eq!(Format::parse("jp2").unwrap(), Format::Jp2);
        assert_eq!(Format::parse("pdf").unwrap(), Format::Pdf);
        assert_eq!(Format::parse("webp").unwrap(), Format::WebP);
        assert_eq!(
            Format::parse("11").unwrap_err(),
            "could not understand format specification: 11"
        );
    }
}
