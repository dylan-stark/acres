use std::marker::PhantomData;
use std::str::FromStr;
use std::{cmp::Ordering, fmt::Display};

use serde::{Deserialize, Serialize};

use crate::{
    IiifError, Uri,
    builder::{Set, Unset},
};

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
/// [floating point]: https://iiif.io/api/image/2.0/#image-request-parameters
/// [`Size`]: enum.Size.html
/// [`Region`]: enum.Region.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    use rstest::rstest;

    use crate::image_request::Percentage;

    #[rstest]
    #[case(0.0, "0")]
    #[case(4.2, "4.2")]
    #[case(1_f32/3_f32, "0.33")]
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
/// [floating point]: https://iiif.io/api/image/2.0/#image-request-parameters
/// [`Rotatin`]: enum.Rotatin.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    use rstest::rstest;

    use crate::image_request::Degree;

    #[rstest]
    #[case(0.0, "0")]
    #[case(4.2, "4.2")]
    #[case(33.0 + 1_f32/3_f32, "33.33")]
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

/// Defines a [region] of the underlying image content to retrieve.
///
/// You can create one from a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Region;
///
/// # fn main() -> Result<()> {
/// let region: Region = "pct:41.6,7.5,40,70".parse()?;
/// assert_eq!(
///     region,
///     Region::Percentage(
///         41.6_f32.try_into()?,
///         7.5_f32.try_into()?,
///         40_f32.try_into()?,
///         70_f32.try_into()?,
///     ),
/// );
/// # Ok(())
/// # }
/// ```
///
/// Or you can create one programmatically.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Region;
///
/// # fn main() -> Result<()> {
/// let region = Region::Percentage(
///     41.6_f32.try_into()?,
///     7.5_f32.try_into()?,
///     40_f32.try_into()?,
///     70_f32.try_into()?,
/// );
/// assert_eq!(region.to_string(), "pct:41.6,7.5,40,70");
/// # Ok(())
/// # }
/// ```
///
///
/// [region]: https://iiif.io/api/image/2.0/#region
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl FromStr for Region {
    type Err = IiifError;

    /// Creates a Region from a character string.
    ///
    /// Valid strings are
    ///
    /// - 'full'
    /// - 'x,y,w,h' (x, y, w, and h are non-negative integers)
    /// - 'pct:x,y,w,h' (x, y, w, and h are non-negative floats)
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_xywh<T: FromStr>(input: &str) -> Option<[T; 4]> {
            let values: Vec<T> = input
                .split(',')
                .map(|part| part.trim().parse().ok())
                .collect::<Option<_>>()?;

            values.try_into().ok()
        }

        let invalid = || IiifError::InvalidRegion(s.into());

        match s {
            "full" => Ok(Region::Full),
            _ if let Some(xywh) = s.strip_prefix("pct:") => {
                let [x, y, w, h] = parse_xywh(xywh).ok_or_else(invalid)?;
                Ok(Region::Percentage(x, y, w, h))
            }
            _ => {
                let [x, y, w, h] = parse_xywh(s).ok_or_else(invalid)?;
                Ok(Region::Absolute(x, y, w, h))
            }
        }
    }
}

/// Defines [size] (scaling) of the underlying image content to retrieve.
///
/// You can create one from a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Size;
///
/// # fn main() -> Result<()> {
/// let size: Size = "!640,480".parse()?;
/// assert_eq!(size, Size::BestFit(640, 480));
/// # Ok(())
/// # }
/// ```
///
/// Or you can create one programmatically.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Size;
///
/// # fn main() -> Result<()> {
/// let size = Size::BestFit(640, 480);
/// assert_eq!(size.to_string(), "!640,480");
/// # Ok(())
/// # }
/// ```
///
/// [size]: https://iiif.io/api/image/2.0/#size
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Size {
    /// Don't scale.
    #[default]
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

impl FromStr for Size {
    type Err = IiifError;

    /// Creates a Size from a string.
    ///
    /// Valid strings are
    ///
    /// - full
    /// - pct:n
    /// - !w,h
    /// - w,
    /// - ,h
    /// - w,h
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn parse_wh(input: &str) -> Option<[Option<u32>; 2]> {
            let values: Vec<_> = input
                .split(',')
                .map(|part| part.trim().parse().ok())
                .collect();
            values.try_into().ok()
        }

        let invalid = || IiifError::InvalidSize(s.into());

        match s {
            "full" => Ok(Size::Full),
            _ if let Some(n) = s.strip_prefix("pct:") => {
                Ok(Size::Percentage(n.trim().parse().map_err(|_| invalid())?))
            }
            _ if let Some(wh) = s.strip_prefix("!") => match parse_wh(wh).ok_or_else(invalid)? {
                [Some(w), Some(h)] => Ok(Size::BestFit(w, h)),
                _ => Err(IiifError::InvalidSize(s.into())),
            },
            _ => match parse_wh(s).ok_or_else(invalid)? {
                [Some(w), None] => Ok(Size::Width(w)),
                [None, Some(h)] => Ok(Size::Height(h)),
                [Some(w), Some(h)] => Ok(Size::Exactly(w, h)),
                [None, None] => Err(IiifError::InvalidSize(s.into())),
            },
        }
    }
}

/// Defines [rotation] of the underlying image content.
///
/// You can create one from a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Rotation;
///
/// # fn main() -> Result<()> {
/// let rotation: Rotation = "180".parse()?;
/// assert_eq!(rotation, Rotation::Degrees(180_f32.try_into()?));
/// # Ok(())
/// # }
/// ```
///
/// Or you can create one programmatically.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Rotation;
///
/// # fn main() -> Result<()> {
/// let rotation = Rotation::Degrees(180_f32.try_into()?);
/// assert_eq!(rotation.to_string(), "180");
/// # Ok(())
/// # }
/// ```
/// [rotation]: https://iiif.io/api/image/2.0/#rotation
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rotation {
    /// Clockwise rotation in degrees.
    Degrees(Degree),
    /// Mirrored rotation in degrees.
    Mirrored(Degree),
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Degrees(Degree::default())
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

impl FromStr for Rotation {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || IiifError::InvalidRotation(s.into());

        if let Some(n) = s.strip_prefix('!') {
            Ok(Rotation::Mirrored(n.trim().parse().map_err(|_| invalid())?))
        } else {
            Ok(Rotation::Degrees(s.trim().parse().map_err(|_| invalid())?))
        }
    }
}

/// Defines a [quality] of the underlying image content.
///
/// You can create one from a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Quality;
///
/// # fn main() -> Result<()> {
/// let quality: Quality = "color".parse()?;
/// assert_eq!(quality, Quality::Color);
/// # Ok(())
/// # }
/// ```
///
/// Or you can create one programmatically.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Quality;
///
/// # fn main() -> Result<()> {
/// let quality = Quality::Color;
/// assert_eq!(quality.to_string(), "color");
/// # Ok(())
/// # }
/// ```
///
/// [quality]: https://iiif.io/api/image/2.0/#quality
#[derive(Clone, Debug, PartialEq, Default, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl FromStr for Quality {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == "color" => Ok(Quality::Color),
            _ if s == "gray" => Ok(Quality::Gray),
            _ if s == "bitonal" => Ok(Quality::Bitonal),
            _ if s == "default" => Ok(Quality::Default),
            _ => Err(IiifError::InvalidQuality(s.into())),
        }
    }
}

/// Defines a [format] for the underlying image content.
///
/// You can create one from a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Format;
///
/// # fn main() -> Result<()> {
/// let format: Format = "png".parse()?;
/// assert_eq!(format, Format::Png);
/// # Ok(())
/// # }
/// ```
///
/// Or you can create one programmatically.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Format;
///
/// # fn main() -> Result<()> {
/// let format = Format::Png;
/// assert_eq!(format.to_string(), "png");
/// # Ok(())
/// # }
/// ```
///
/// [format]: https://iiif.io/api/image/2.0/#format
#[derive(Clone, Debug, PartialEq, Default, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl FromStr for Format {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == "jpg" => Ok(Format::Jpg),
            _ if s == "tif" => Ok(Format::Tif),
            _ if s == "png" => Ok(Format::Png),
            _ if s == "gif" => Ok(Format::Gif),
            _ if s == "jp2" => Ok(Format::Jp2),
            _ if s == "pdf" => Ok(Format::Pdf),
            _ if s == "webp" => Ok(Format::WebP),
            _ => Err(IiifError::InvalidFormat(s.into())),
        }
    }
}

/// Defines an [image request] for the IIIF Image API 2.0.
///
/// You can create a new image request if you know all of the parameters upfront.
///
/// ```rust
/// # use anyhow::Result;
/// use std::str::FromStr;
/// use iiif::{Degree, Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
///
/// # fn main() -> Result<()> {
/// let image_request = ImageRequest::new(
///     Uri::from_str("https://example.org/images/12345")?,
///     Region::Full,
///     Size::Width(1024),
///     Rotation::Degrees(Degree::default()),
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
/// # use iiif::{Degree, Format, ImageRequest, Quality, Region, Rotation, Size, Uri};
/// #
/// # fn main() -> Result<()> {
/// let mut image_request = ImageRequest::builder();
/// let mut image_request = image_request
///     .region(Region::Full)
///     .size(Size::Width(1024))
///     .rotation(Rotation::Degrees(Degree::default()))
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
/// let image_request: ImageRequest = "https://example.org/images/12345/full/1024,/0/default.png".parse()?;
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// [image request]: https://iiif.io/api/image/2.0/#4-image-requests
/// [typestate pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Image request syntax:
///
/// > {scheme}://{server}{/prefix}/{identifier}/{region}/{size}/{rotation}/{quality}.{format}
impl FromStr for ImageRequest {
    type Err = IiifError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.rsplitn(5, '/');

        let (quality, format) = parts
            .next()
            .and_then(|file| file.split_once('.'))
            .ok_or_else(|| IiifError::MissingFormat(s.into()))?;
        let quality = quality.parse()?;
        let format: Format = format.parse()?;

        let rotation = parts
            .next()
            .ok_or_else(|| IiifError::MissingRotation(s.into()))?
            .parse()?;

        let size = parts
            .next()
            .ok_or_else(|| IiifError::MissingSize(s.into()))?
            .parse()?;

        let region = parts
            .next()
            .ok_or_else(|| IiifError::MissingRegion(s.into()))?
            .parse()?;

        let uri = parts
            .next()
            .ok_or_else(|| IiifError::MissingIdentifier(s.into()))?
            .parse()?;

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
        assert_eq!(Size::default().to_string(), "full");
        assert_eq!(Rotation::default().to_string(), "0");
        assert_eq!(Quality::default().to_string(), "default");
        assert_eq!(Format::default().to_string(), "jpg");
    }

    #[rstest]
    #[case("full", Region::Full)]
    #[case("1,2,3,4", Region::Absolute(1, 2, 3, 4))]
    #[case("pct:1,2,3,4", Region::Percentage( 1.0.try_into().unwrap(), 2.0.try_into().unwrap(), 3.0.try_into().unwrap(), 4.0.try_into().unwrap()))]
    #[case("pct:1.2,2.37,3.4,4.513", Region::Percentage( 1.2.try_into().unwrap(), 2.37.try_into().unwrap(), 3.4.try_into().unwrap(), 4.513.try_into().unwrap()))]
    fn parsing_region(#[case] value: &str, #[case] expected: Region) {
        assert_eq!(Region::from_str(value).unwrap(), expected);
    }

    #[rstest]
    #[case("ful")]
    #[case("fully")]
    #[case("Full")]
    #[case("1,2,4")]
    #[case("1,2,,4")]
    #[case("1,2,-3,4")]
    #[case("1,2,a,4")]
    #[case("1,2,3,4,5")]
    #[case("1,2,3,4,abc")]
    #[case("pct:1.2,3.4,4.513")]
    #[case("pct:-1.2,2.37,3.4,4.513")]
    fn parsing_region_fails(#[case] value: &str) {
        assert_eq!(
            Region::from_str(value).unwrap_err().to_string(),
            format!("invalid region: {value}")
        )
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
    #[case("42", Rotation::Degrees(42.0_f32.try_into().unwrap()))]
    #[case("42.24", Rotation::Degrees(42.24_f32.try_into().unwrap()))]
    #[case("!42", Rotation::Mirrored(42.0_f32.try_into().unwrap()))]
    #[case("!42.24", Rotation::Mirrored(42.24_f32.try_into().unwrap()))]
    fn parse_rotation_degrees(#[case] value: &str, #[case] expected: Rotation) {
        assert_eq!(Rotation::from_str(value).unwrap(), expected);
    }

    #[rstest]
    #[case(Rotation::Degrees(42.0_f32.try_into().unwrap()), "42")]
    #[case(Rotation::Degrees(42.2_f32.try_into().unwrap()), "42.2")]
    #[case(Rotation::Degrees(42.22222_f32.try_into().unwrap()), "42.22")]
    #[case(Rotation::Mirrored(42.0_f32.try_into().unwrap()), "!42")]
    #[case(Rotation::Mirrored(42.2_f32.try_into().unwrap()), "!42.2")]
    #[case(Rotation::Mirrored(42.22222_f32.try_into().unwrap()), "!42.22")]
    fn display_rotation_degrees(#[case] value: Rotation, #[case] expected: &str) {
        assert_eq!(value.to_string(), expected);
    }

    #[rstest]
    #[case("")]
    #[case("forty-two")]
    #[case("!forty-two")]
    fn parsing_rotation_fails(#[case] value: &str) {
        assert_eq!(
            Rotation::from_str(value).unwrap_err().to_string(),
            format!("invalid rotation: {value}")
        );
    }

    #[rstest]
    #[case("color", Quality::Color)]
    #[case("gray", Quality::Gray)]
    #[case("bitonal", Quality::Bitonal)]
    #[case("default", Quality::Default)]
    fn parsing_quality(#[case] value: &str, #[case] expected: Quality) {
        assert_eq!(Quality::from_str(value).unwrap(), expected);
    }

    #[rstest]
    #[case("11")]
    fn parsing_quality_fails(#[case] value: &str) {
        assert_eq!(
            Quality::from_str(value).unwrap_err().to_string(),
            format!("invalid quality: {value}")
        )
    }

    #[rstest]
    #[case("jpg", Format::Jpg)]
    #[case("tif", Format::Tif)]
    #[case("png", Format::Png)]
    #[case("gif", Format::Gif)]
    #[case("jp2", Format::Jp2)]
    #[case("pdf", Format::Pdf)]
    #[case("webp", Format::WebP)]
    fn parsing_format(#[case] value: &str, #[case] expected: Format) {
        assert_eq!(Format::from_str(value).unwrap(), expected);
    }

    #[rstest]
    #[case("bmp")]
    fn parsing_format_fails(#[case] value: &str) {
        assert_eq!(
            Format::from_str(value).unwrap_err().to_string(),
            format!("invalid format: {value}")
        )
    }

    fn uri() -> Uri {
        Uri::from_str("https://www.artic.edu/iiif/2/abc").unwrap()
    }

    #[test]
    fn builder_defaults_to_full_default_jpg() {
        let request = ImageRequest::builder()
            .uri(uri())
            .region(Region::default())
            .size(Size::default())
            .rotation(Rotation::default())
            .quality(Quality::default())
            .format(Format::default())
            .build();
        assert_eq!(
            request.to_string(),
            "https://www.artic.edu/iiif/2/abc/full/full/0/default.jpg"
        );
    }

    #[test]
    fn builder_and_new_build_the_same_url() {
        let region = Region::Absolute(10, 20, 300, 400);
        let size = Size::from_str("843,").unwrap();
        let built = ImageRequest::builder()
            .uri(uri())
            .region(region.clone())
            .size(size.clone())
            .rotation(Rotation::default())
            .quality(Quality::default())
            .format(Format::Png)
            .build();
        let new = ImageRequest::new(
            uri(),
            region,
            size,
            Rotation::default(),
            Quality::default(),
            Format::Png,
        );
        assert_eq!(built.to_string(), new.to_string());
        assert_eq!(
            built.to_string(),
            "https://www.artic.edu/iiif/2/abc/10,20,300,400/843,/0/default.png"
        );
    }

    #[rstest]
    #[case("https://www.artic.edu/iiif/2/abc/full/843,/0/default.jpg")]
    #[case("https://www.artic.edu:5555/iiif/2/abc/full/843,/0/default.jpg")]
    #[case("https://www.artic.edu/iiif/2/abc/pct:10,10,50,50/!100,100/90/gray.webp")]
    fn image_request_round_trips(#[case] url: &str) {
        let request = ImageRequest::from_str(url).unwrap();
        assert_eq!(request.to_string(), url);
    }
}
