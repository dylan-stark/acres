use bytes::Bytes;
use std::marker::PhantomData;
use std::str::FromStr;
use std::{cmp::Ordering, fmt::Display};

use crate::{IiifError, Set, Unset, Uri};

/// Defines a [floating point] percentage.
///
/// These are used when setting [`Region`] or [`Size`] as a percentage of the underlying image content.
///
/// You can create one from an `f32` or a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Percentage;
///
/// # fn main() -> Result<()> {
/// let from_f32: Percentage = 4.2.try_into()?;
/// let from_str: Percentage = "4.2".parse()?;
/// assert_eq!(from_f32, from_str);
/// # Ok(())
/// # }
/// ```
///
/// We prefer whole numbers and keep only up to 2 fractional digits when including percentages in Regions or Sizes and URLs.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Percentage};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Percentage::try_from(10.0_f32)?.to_string(), "10");
/// assert_eq!(Percentage::try_from(10.3_f32)?.to_string(), "10.3");
/// assert_eq!(Percentage::try_from(10.3333333_f32)?.to_string(), "10.33");
/// # Ok(())
/// # }
/// ```
///
/// We also take care of ensuring percentages are from 0 to 100, inclusive.
/// So expect to get errors if you're outside of those bounds.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Percentage};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Percentage::try_from(-10.0_f32).unwrap_err().to_string(), "invalid percentage: -10");
/// assert_eq!(Percentage::try_from(110.0_f32).unwrap_err().to_string(), "invalid percentage: 110");
/// # Ok(())
/// # }
/// ```
///
/// [floating point]: https://iiif.io/api/image/3.0/#47-floating-point-values
/// [`Size`]: enum.Size.html
/// [`Region`]: enum.Region.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percentage(String);

impl Default for Percentage {
    fn default() -> Self {
        Self(String::from("0"))
    }
}

impl TryFrom<f32> for Percentage {
    type Error = IiifError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || !value.is_sign_positive() {
            return Err(IiifError::InvalidPercentage(value));
        }
        if let Some(Ordering::Greater) = value.partial_cmp(&100.0) {
            return Err(IiifError::InvalidPercentage(value));
        };
        Ok(Percentage(
            format!("{:.2}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string(),
        ))
    }
}

impl FromStr for Percentage {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s.parse().map_err(IiifError::from)?;
        Self::try_from(value)
    }
}

impl Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Percentage {
    /// Creates a Percentage with default of 0.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod percentage_tests {
    use core::f32;

    use rstest::rstest;

    use crate::image_request::Percentage;

    #[rstest]
    #[case(0.0, "0")]
    #[case(4.2, "4.2")]
    #[case(1 as f32/3 as f32, "0.33")]
    #[case(100.0, "100")]
    fn try_from_f32(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(
            format!("{}", Percentage::try_from(value).unwrap()),
            expected
        );
    }

    #[rstest]
    #[case(-4.2, "invalid percentage: -4.2")]
    #[case(-0.0, "invalid percentage: -0")]
    #[case(100.00001, "invalid percentage: 100.00001")]
    #[case(f32::NAN, "invalid percentage: NaN")]
    #[case(f32::INFINITY, "invalid percentage: inf")]
    #[case(f32::NEG_INFINITY, "invalid percentage: -inf")]
    fn try_from_f32_fails(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(
            format!("{}", Percentage::try_from(value).unwrap_err()),
            expected
        )
    }
}

/// Defines a [floating point] degrees.
///
/// These are used when setting [`Rotation`] as degrees to rotate the underlying image content.
///
/// You can create one from an `f32` or a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Degree;
///
/// # fn main() -> Result<()> {
/// let from_f32: Degree = 4.2.try_into()?;
/// let from_str: Degree = "4.2".parse()?;
/// assert_eq!(from_f32, from_str);
/// # Ok(())
/// # }
/// ```
///
/// We prefer whole numbers and keep only up to 2 fractional digits including in Rotations and URLs.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Degree};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Degree::try_from(10.0_f32)?.to_string(), "10");
/// assert_eq!(Degree::try_from(10.3_f32)?.to_string(), "10.3");
/// assert_eq!(Degree::try_from(10.3333333_f32)?.to_string(), "10.33");
/// # Ok(())
/// # }
/// ```
///
/// We also take care of ensuring rotations are from 0 to 360, inclusive.
/// So expect to get errors if you're outside of those bounds.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Degree};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Degree::try_from(-10.0_f32).unwrap_err().to_string(), "invalid degree: -10");
/// assert_eq!(Degree::try_from(370.0_f32).unwrap_err().to_string(), "invalid degree: 370");
/// # Ok(())
/// # }
/// ```
///
/// [floating point]: https://iiif.io/api/image/3.0/#47-floating-point-values
/// [`Rotatin`]: enum.Rotatin.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Degree(String);

impl Default for Degree {
    fn default() -> Self {
        Self(String::from("0"))
    }
}

impl TryFrom<f32> for Degree {
    type Error = IiifError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || !value.is_sign_positive() {
            return Err(IiifError::InvalidDegree(value));
        }
        if let Some(Ordering::Greater) = value.partial_cmp(&360.0) {
            return Err(IiifError::InvalidDegree(value));
        };
        Ok(Degree(
            format!("{:.2}", value)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string(),
        ))
    }
}

impl FromStr for Degree {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s.parse().map_err(IiifError::from)?;
        Self::try_from(value)
    }
}

impl Display for Degree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Degree {
    /// Creates a Degree with default of 0.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod degree_tests {
    use core::f32;

    use rstest::rstest;

    use crate::image_request::Degree;

    #[rstest]
    #[case(0.0, "0")]
    #[case(4.2, "4.2")]
    #[case(33.0 + 1 as f32/3 as f32, "33.33")]
    #[case(360.0, "360")]
    fn try_from_f33(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(format!("{}", Degree::try_from(value).unwrap()), expected);
    }

    #[rstest]
    #[case(-4.2, "invalid degree: -4.2")]
    #[case(-0.0, "invalid degree: -0")]
    #[case(360.001, "invalid degree: 360.001")]
    #[case(f32::NAN, "invalid degree: NaN")]
    #[case(f32::INFINITY, "invalid degree: inf")]
    #[case(f32::NEG_INFINITY, "invalid degree: -inf")]
    fn try_from_f32_fails(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(
            format!("{}", Degree::try_from(value).unwrap_err()),
            expected
        )
    }
}

/// Defines an [image request] for the IIIF Image API 3.0.
///
/// You can create a new image request if you know all of the parameters upfront.
///
/// ```rust
/// # use anyhow::Result;
/// use std::str::FromStr;
/// use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
///
/// # fn main() -> Result<()> {
/// let image_request = ImageRequest::new(
///     Uri::from_str("https://example.org/images/12345")?,
///     Region::Full,
///     Size::Width(1024),
///     Rotation::Degrees(0.0),
///     Quality::Default,
///     Format::Png,
/// );
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// Or you can construct one piece-wise with a builder. This gives you the flexibility to set
/// parameters when and in what order works best for you. And the [typestate pattern] ensures you
/// can't `.build()` until all parameters are set.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
/// #
/// # fn main() -> Result<()> {
/// let mut image_request = ImageRequest::builder();
/// let mut image_request = image_request
///     .region(Region::Full)
///     .size(Size::Width(1024))
///     .rotation(Rotation::Degrees(0.0))
///     .quality(Quality::Default)
///     .format(Format::Png);
/// // The following won't compile because the URI isn't set:
/// // let _ = image_request.build();
///     
/// let uri = Uri::from_str("https://example.org/images/12345")?;
/// let mut image_request = image_request.uri(uri);
/// // Now we can build it!
///
/// let image_request = image_request.build();
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// And on the off chance that you already have a string and want an [`ImageRequest`], you can do
/// that, too.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
/// #
/// # fn main() -> Result<()> {
/// let image_request: ImageRequest = "https://example.org/images/12345/full/1024,/0/default.png".try_into()?;
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// [image request]: https://iiif.io/api/image/3.0/#4-image-requests
/// [typestate pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Clone, Debug)]
pub struct ImageRequest {
    uri: Uri,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl Display for ImageRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}/{}/{}.{}",
            self.uri, self.region, self.size, self.rotation, self.quality, self.format
        )
    }
}

impl TryFrom<&str> for ImageRequest {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let url = url::Url::parse(value).map_err(IiifError::from)?;

        let region: Region;
        let size: Size;
        let rotation: Rotation;
        let quality: Quality;
        let format: Format;
        let mut params: Vec<&str> = url.path_segments().map_or_else(Vec::new, |v| v.collect());
        if let Some(file) = params.pop() {
            if let Some((q, f)) = file.split_once('.') {
                format = f.try_into()?;
                quality = q.try_into()?;
            } else {
                return Err(IiifError::MissingFormat(value.to_string()));
            }
        } else {
            return Err(IiifError::MissingFormat(value.to_string()));
        }
        if let Some(r) = params.pop() {
            rotation = r.try_into()?;
        } else {
            return Err(IiifError::MissingRotation(value.to_string()));
        }
        if let Some(s) = params.pop() {
            size = s.try_into()?;
        } else {
            return Err(IiifError::MissingSize(value.to_string()));
        }
        if let Some(r) = params.pop() {
            region = r.try_into()?;
        } else {
            return Err(IiifError::MissingRegion(value.to_string()));
        }

        let uri = Uri::from_str(
            format!(
                "{}://{}/{}",
                url.scheme(),
                url.host_str().map_or("", |v| v),
                params.join("/")
            )
            .as_str(),
        )?;

        Ok(Self {
            uri,
            region,
            size,
            rotation,
            quality,
            format,
        })
    }
}

impl ImageRequest {
    /// Constructs a new request.
    pub fn new(
        uri: Uri,
        region: Region,
        size: Size,
        rotation: Rotation,
        quality: Quality,
        format: Format,
    ) -> Self {
        Self {
            uri,
            region,
            size,
            rotation,
            quality,
            format,
        }
    }

    /// Returns a new builder.
    pub fn builder() -> Builder<Unset, Unset, Unset, Unset, Unset, Unset> {
        Builder::default()
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
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Region {
    /// The complete image.
    #[default]
    Full,
    /// Region expressed in absolute pixel values.
    Absolute(u32, u32, u32, u32),
    /// Region expressed in percent of image's dimensions.
    Percentage(Percentage, Percentage, Percentage, Percentage),
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

impl TryFrom<&str> for Region {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
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
            let parts: Vec<Percentage> = parts
                .filter_map(|part| part.trim().parse::<Percentage>().ok())
                .collect();
            if parts.len() == 4 {
                return Ok(Region::Percentage(
                    parts[0].clone(),
                    parts[1].clone(),
                    parts[2].clone(),
                    parts[3].clone(),
                ));
            }
        } else {
            let parts: Vec<u32> = parts
                .filter_map(|part| part.trim().parse::<u32>().ok())
                .collect();
            if parts.len() == 4 {
                return Ok(Region::Absolute(parts[0], parts[1], parts[2], parts[3]));
            }
        }

        Err(IiifError::InvalidRegion(value.into()))
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
            let parts: Vec<Percentage> = parts
                .filter_map(|part| part.trim().parse::<Percentage>().ok())
                .collect();
            if parts.len() == 4 {
                return Ok(Region::Percentage(
                    parts[0].clone(),
                    parts[1].clone(),
                    parts[2].clone(),
                    parts[3].clone(),
                ));
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Size {
    /// Don't scale.
    Full,
    /// Scale width to this many pixels.
    Width(u32),
    /// Scale height to this many pixels.
    Height(u32),
    /// Scale height and width by this percent.
    Percentage(Percentage),
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

impl TryFrom<&str> for Size {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == "full" {
            return Ok(Size::Full);
        } else if value.starts_with("pct:") {
            let n = value.replacen("pct:", "", 1).parse::<f32>().ok();
            if let Some(n) = n {
                return Ok(Size::Percentage(n.try_into()?));
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
        Err(IiifError::InvalidSize(value.into()))
    }
}

impl FromStr for Size {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
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

impl TryFrom<&str> for Rotation {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Ok(degrees) = value.parse::<f32>() {
            return Ok(Rotation::Degrees(degrees));
        }
        if let Ok(degrees) = value.replacen("!", "", 1).parse::<f32>() {
            return Ok(Rotation::Mirrored(degrees));
        }

        Err(IiifError::InvalidRotation(value.into()))
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

impl TryFrom<&str> for Quality {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            _ if value == "color" => Ok(Quality::Color),
            _ if value == "gray" => Ok(Quality::Gray),
            _ if value == "bitonal" => Ok(Quality::Bitonal),
            _ if value == "default" => Ok(Quality::Default),
            _ => Err(IiifError::InvalidQuality(value.into())),
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

impl TryFrom<&str> for Format {
    type Error = IiifError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            _ if value == "jpg" => Ok(Format::Jpg),
            _ if value == "tif" => Ok(Format::Tif),
            _ if value == "png" => Ok(Format::Png),
            _ if value == "gif" => Ok(Format::Gif),
            _ if value == "jp2" => Ok(Format::Jp2),
            _ if value == "pdf" => Ok(Format::Pdf),
            _ if value == "webp" => Ok(Format::WebP),
            _ => Err(IiifError::InvalidFormat(value.into())),
        }
    }
}

impl Format {
    // TODO: Remove this method in favor of TryFrom impl.
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

#[derive(Clone, Debug, Default)]
struct Partial {
    uri: Option<Uri>,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl From<Partial> for ImageRequest {
    fn from(value: Partial) -> Self {
        ImageRequest {
            uri: value.uri.expect("set"),
            region: value.region,
            size: value.size,
            rotation: value.rotation,
            quality: value.quality,
            format: value.format,
        }
    }
}

type SetType<A, B, C, D, E, F> = (
    PhantomData<A>,
    PhantomData<B>,
    PhantomData<C>,
    PhantomData<D>,
    PhantomData<E>,
    PhantomData<F>,
);

/// An image request builder.
#[derive(Debug)]
pub struct Builder<A, B, C, D, E, F> {
    under_construction: Partial,
    set: SetType<A, B, C, D, E, F>,
}

impl Default for Builder<Unset, Unset, Unset, Unset, Unset, Unset> {
    fn default() -> Self {
        Self {
            under_construction: Partial::default(),
            set: (
                PhantomData,
                PhantomData,
                PhantomData,
                PhantomData,
                PhantomData,
                PhantomData,
            ),
        }
    }
}

impl Builder<Set, Set, Set, Set, Set, Set> {
    /// Builds the full image request.
    pub fn build(&self) -> ImageRequest {
        self.under_construction.clone().into()
    }
}

impl<A, B, C, D, E, F> Builder<A, B, C, D, E, F> {
    pub fn uri(mut self, uri: Uri) -> Builder<Set, B, C, D, E, F> {
        self.under_construction.uri = Some(uri);
        Builder {
            under_construction: self.under_construction,
            set: (
                PhantomData::<Set>,
                self.set.1,
                self.set.2,
                self.set.3,
                self.set.4,
                self.set.5,
            ),
        }
    }

    pub fn region(mut self, region: Region) -> Builder<A, Set, C, D, E, F> {
        self.under_construction.region = region;
        Builder {
            under_construction: self.under_construction,
            set: (
                self.set.0,
                PhantomData::<Set>,
                self.set.2,
                self.set.3,
                self.set.4,
                self.set.5,
            ),
        }
    }

    pub fn size(mut self, size: Size) -> Builder<A, B, Set, D, E, F> {
        self.under_construction.size = size;
        Builder {
            under_construction: self.under_construction,
            set: (
                self.set.0,
                self.set.1,
                PhantomData::<Set>,
                self.set.3,
                self.set.4,
                self.set.5,
            ),
        }
    }

    pub fn rotation(mut self, rotation: Rotation) -> Builder<A, B, C, Set, E, F> {
        self.under_construction.rotation = rotation;
        Builder {
            under_construction: self.under_construction,
            set: (
                self.set.0,
                self.set.1,
                self.set.2,
                PhantomData::<Set>,
                self.set.4,
                self.set.5,
            ),
        }
    }

    pub fn quality(mut self, quality: Quality) -> Builder<A, B, C, D, Set, F> {
        self.under_construction.quality = quality;
        Builder {
            under_construction: self.under_construction,
            set: (
                self.set.0,
                self.set.1,
                self.set.2,
                self.set.3,
                PhantomData::<Set>,
                self.set.5,
            ),
        }
    }

    pub fn format(mut self, format: Format) -> Builder<A, B, C, D, E, Set> {
        self.under_construction.format = format;
        Builder {
            under_construction: self.under_construction,
            set: (
                self.set.0,
                self.set.1,
                self.set.2,
                self.set.3,
                self.set.4,
                PhantomData::<Set>,
            ),
        }
    }
}
#[cfg(test)]
mod tests {
    use rstest::rstest;

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
            Region::Percentage(
                1.0.try_into().unwrap(),
                2.0.try_into().unwrap(),
                3.0.try_into().unwrap(),
                4.0.try_into().unwrap()
            )
        );
        assert_eq!(
            Region::parse("pct:1.2,2.37,3.4,4.513").unwrap(),
            Region::Percentage(
                1.2.try_into().unwrap(),
                2.37.try_into().unwrap(),
                3.4.try_into().unwrap(),
                4.513.try_into().unwrap()
            )
        );
        assert_eq!(
            Region::parse("pct:1.2,3.4,4.513").unwrap_err(),
            "could not understand region specification: pct:1.2,3.4,4.513"
        );
    }

    #[rstest]
    #[case("full", Size::Full)]
    #[case("42,", Size::Width(42))]
    #[case(",42", Size::Height(42))]
    #[case("pct:42", Size::Percentage(Percentage::from_str("42").unwrap()))]
    #[case("pct:42.3", Size::Percentage(Percentage::from_str("42.3").unwrap()))]
    #[case("640,480", Size::Exactly(640, 480))]
    #[case("!640,480", Size::BestFit(640, 480))]
    fn parsing(#[case] value: &str, #[case] expected: Size) {
        assert_eq!(Size::from_str(value).unwrap(), expected);
    }

    #[rstest]
    #[case("FULL")]
    #[case("42.0,")]
    #[case(",42.0")]
    #[case("42.0,42.0")]
    #[case("!42.0,42.0")]
    #[case("pct:")]
    fn parsing_fails(#[case] value: &str) {
        assert_eq!(
            Size::from_str(value).unwrap_err().to_string(),
            format!("invalid size: {value}")
        );
    }

    #[rstest]
    #[case("42", Rotation::Degrees(42.0))]
    #[case("42.24", Rotation::Degrees(42.24))]
    fn parse_rotation_degrees(#[case] value: &str, #[case] expected: Rotation) {
        assert_eq!(Rotation::parse(value).unwrap(), expected);
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
