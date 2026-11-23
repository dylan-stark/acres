use std::str::FromStr;
use std::{cmp::Ordering, fmt::Display};

use crate::{BaseUri, ParameterError, RequestError};

/// A non-negative value with two fractional digits, stored as a whole number of hundredths so that
/// ordering, equality and hashing are numeric.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Hundredths(u32);

impl Hundredths {
    /// Rounds a finite, non-negative `value` to two fractional digits. Rounding goes through
    /// `{:.2}` so it matches what `Display` used to print exactly (round half to even on the
    /// exact `f32` value).
    fn from_f32(value: f32) -> Self {
        let rounded = format!("{:.2}", value);
        let (whole, frac) = rounded
            .split_once('.')
            .expect("`{:.2}` has a fractional part");
        let whole: u32 = whole.parse().expect("`{:.2}` of a bounded value is digits");
        let frac: u32 = frac.parse().expect("`{:.2}` of a bounded value is digits");
        Self(whole * 100 + frac)
    }

    /// The nearest `f32` to the stored value.
    fn to_f32(self) -> f32 {
        // Divide in `f64` so the result is the correctly rounded `f32` of `n / 100`.
        (f64::from(self.0) / 100.0) as f32
    }
}

impl Display for Hundredths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (whole, frac) = (self.0 / 100, self.0 % 100);
        match frac {
            0 => write!(f, "{}", whole),
            _ if frac % 10 == 0 => write!(f, "{}.{}", whole, frac / 10),
            _ => write!(f, "{}.{:02}", whole, frac),
        }
    }
}

#[cfg(test)]
mod hundredths_tests {
    use proptest::prelude::*;
    use rstest::rstest;

    use super::{Degrees, Hundredths, Percentage};

    /// How `Display` was computed when the value was stored as a string.
    fn old_display(value: f32) -> String {
        format!("{:.2}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }

    #[rstest]
    #[case(0.0, 0, "0")]
    #[case(0.001, 0, "0")]
    #[case(0.125, 12, "0.12")]
    #[case(4.2, 420, "4.2")]
    #[case(4.05, 405, "4.05")]
    #[case(99.999, 10000, "100")]
    #[case(360.0, 36000, "360")]
    fn from_f32(#[case] value: f32, #[case] hundredths: u32, #[case] displayed: &str) {
        let h = Hundredths::from_f32(value);
        assert_eq!(h, Hundredths(hundredths));
        assert_eq!(h.to_string(), displayed);
    }

    #[test]
    fn percentages_order_numerically() {
        let nine: Percentage = "9".parse().unwrap();
        let ten: Percentage = "10".parse().unwrap();
        assert!(nine < ten);
    }

    #[test]
    fn degrees_order_numerically() {
        let nine: Degrees = "9".parse().unwrap();
        let ninety: Degrees = "90".parse().unwrap();
        let one_eighty: Degrees = "180".parse().unwrap();
        assert!(nine < ninety && ninety < one_eighty);
    }

    #[test]
    fn equal_after_rounding() {
        assert_eq!(
            "4.2".parse::<Percentage>().unwrap(),
            "4.200".parse().unwrap()
        );
    }

    proptest! {
        #[test]
        fn display_matches_old_format(value in 0.0_f32..=360.0) {
            prop_assert_eq!(Hundredths::from_f32(value).to_string(), old_display(value));
        }

        #[test]
        fn value_is_nearest_f32_to_display(n in 0_u32..=36000) {
            let h = Hundredths(n);
            prop_assert_eq!(h.to_f32(), h.to_string().parse::<f32>().unwrap());
        }

        #[test]
        fn order_matches_value(a in 0.0_f32..=100.0, b in 0.0_f32..=100.0) {
            let (pa, pb) = (Percentage::try_from(a).unwrap(), Percentage::try_from(b).unwrap());
            let (ra, rb): (f32, f32) = (pa.to_string().parse().unwrap(), pb.to_string().parse().unwrap());
            prop_assert_eq!(pa.cmp(&pb), ra.total_cmp(&rb));
        }
    }
}

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
/// assert_eq!(Percentage::try_from(-10.0_f32).unwrap_err().to_string(), "percentage out of range: -10");
/// assert_eq!(Percentage::try_from(110.0_f32).unwrap_err().to_string(), "percentage out of range: 110");
/// # Ok(())
/// # }
/// ```
///
/// [floating point]: https://iiif.io/api/image/2.0/#image-request-parameters
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percentage(Hundredths);

impl TryFrom<f32> for Percentage {
    type Error = ParameterError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || !value.is_sign_positive() {
            return Err(ParameterError::PercentageOutOfRange(value));
        }
        if let Some(Ordering::Greater) = value.partial_cmp(&100.0) {
            return Err(ParameterError::PercentageOutOfRange(value));
        };
        Ok(Percentage(Hundredths::from_f32(value)))
    }
}

impl FromStr for Percentage {
    type Err = ParameterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s
            .parse()
            .map_err(|source| ParameterError::InvalidPercentage {
                input: s.into(),
                source,
            })?;
        Self::try_from(value)
    }
}

impl Display for Percentage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Percentage {
    /// Returns the value as an `f32`, rounded to two fractional digits.
    ///
    /// ```rust
    /// # use anyhow::Result;
    /// use iiif::Percentage;
    ///
    /// # fn main() -> Result<()> {
    /// assert_eq!(Percentage::try_from(10.3333333_f32)?.value(), 10.33);
    /// # Ok(())
    /// # }
    /// ```
    pub fn value(self) -> f32 {
        self.0.to_f32()
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
    #[case(-4.2, "percentage out of range: -4.2")]
    #[case(-0.0, "percentage out of range: -0")]
    #[case(100.00001, "percentage out of range: 100.00001")]
    #[case(f32::NAN, "percentage out of range: NaN")]
    #[case(f32::INFINITY, "percentage out of range: inf")]
    #[case(f32::NEG_INFINITY, "percentage out of range: -inf")]
    fn try_from_f32_fails(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(
            format!("{}", Percentage::try_from(value).unwrap_err()),
            expected
        )
    }

    #[test]
    fn from_str_fails_with_percentage_error() {
        assert_eq!(
            "abc".parse::<Percentage>().unwrap_err().to_string(),
            "invalid percentage: abc"
        );
    }
}

/// Defines an angle in [floating point] degrees.
///
/// These are used when setting [`Rotation`] as degrees to rotate the underlying image content.
///
/// You can create one from an `f32` or a string.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::Degrees;
///
/// # fn main() -> Result<()> {
/// let from_f32: Degrees = 4.2.try_into()?;
/// let from_str: Degrees = "4.2".parse()?;
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
/// # use iiif::{Degrees};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Degrees::try_from(10.0_f32)?.to_string(), "10");
/// assert_eq!(Degrees::try_from(10.3_f32)?.to_string(), "10.3");
/// assert_eq!(Degrees::try_from(10.3333333_f32)?.to_string(), "10.33");
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
/// # use iiif::{Degrees};
/// #
/// # fn main() -> Result<()> {
/// assert_eq!(Degrees::try_from(-10.0_f32).unwrap_err().to_string(), "degrees out of range: -10");
/// assert_eq!(Degrees::try_from(370.0_f32).unwrap_err().to_string(), "degrees out of range: 370");
/// # Ok(())
/// # }
/// ```
///
/// [floating point]: https://iiif.io/api/image/2.0/#image-request-parameters
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Degrees(Hundredths);

impl TryFrom<f32> for Degrees {
    type Error = ParameterError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || !value.is_sign_positive() {
            return Err(ParameterError::DegreesOutOfRange(value));
        }
        if let Some(Ordering::Greater) = value.partial_cmp(&360.0) {
            return Err(ParameterError::DegreesOutOfRange(value));
        };
        Ok(Degrees(Hundredths::from_f32(value)))
    }
}

impl FromStr for Degrees {
    type Err = ParameterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: f32 = s.parse().map_err(|source| ParameterError::InvalidDegrees {
            input: s.into(),
            source,
        })?;
        Self::try_from(value)
    }
}

impl Display for Degrees {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Degrees {
    /// Returns the value as an `f32`, rounded to two fractional digits.
    ///
    /// ```rust
    /// # use anyhow::Result;
    /// use iiif::Degrees;
    ///
    /// # fn main() -> Result<()> {
    /// assert_eq!(Degrees::try_from(10.3333333_f32)?.value(), 10.33);
    /// # Ok(())
    /// # }
    /// ```
    pub fn value(self) -> f32 {
        self.0.to_f32()
    }
}

#[cfg(test)]
mod degrees_tests {

    use rstest::rstest;

    use crate::image_request::Degrees;

    #[rstest]
    #[case(0.0, "0")]
    #[case(4.2, "4.2")]
    #[case(33.0 + 1_f32/3_f32, "33.33")]
    #[case(360.0, "360")]
    fn try_from_f33(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(format!("{}", Degrees::try_from(value).unwrap()), expected);
    }

    #[rstest]
    #[case(-4.2, "degrees out of range: -4.2")]
    #[case(-0.0, "degrees out of range: -0")]
    #[case(360.001, "degrees out of range: 360.001")]
    #[case(f32::NAN, "degrees out of range: NaN")]
    #[case(f32::INFINITY, "degrees out of range: inf")]
    #[case(f32::NEG_INFINITY, "degrees out of range: -inf")]
    fn try_from_f32_fails(#[case] value: f32, #[case] expected: &str) {
        assert_eq!(
            format!("{}", Degrees::try_from(value).unwrap_err()),
            expected
        )
    }

    #[test]
    fn from_str_fails_with_degrees_error() {
        assert_eq!(
            "abc".parse::<Degrees>().unwrap_err().to_string(),
            "invalid degrees: abc"
        );
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
///     Region::Percentage {
///         x: 41.6_f32.try_into()?,
///         y: 7.5_f32.try_into()?,
///         w: 40_f32.try_into()?,
///         h: 70_f32.try_into()?,
///     },
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
/// let region = Region::Percentage {
///     x: 41.6_f32.try_into()?,
///     y: 7.5_f32.try_into()?,
///     w: 40_f32.try_into()?,
///     h: 70_f32.try_into()?,
/// };
/// assert_eq!(region.to_string(), "pct:41.6,7.5,40,70");
/// # Ok(())
/// # }
/// ```
///
///
/// [region]: https://iiif.io/api/image/2.0/#region
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Region {
    /// The complete image.
    #[default]
    Full,
    /// Region expressed in absolute pixel values.
    Pixels {
        /// Pixels from the left edge.
        x: u32,
        /// Pixels from the top edge.
        y: u32,
        /// Width in pixels.
        w: u32,
        /// Height in pixels.
        h: u32,
    },
    /// Region expressed in percent of image's dimensions.
    Percentage {
        /// Percent of the width from the left edge.
        x: Percentage,
        /// Percent of the height from the top edge.
        y: Percentage,
        /// Width in percent of the image's width.
        w: Percentage,
        /// Height in percent of the image's height.
        h: Percentage,
    },
}

impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::Full => write!(f, "full"),
            Region::Pixels { x, y, w, h } => write!(f, "{},{},{},{}", x, y, w, h),
            Region::Percentage { x, y, w, h } => write!(f, "pct:{},{},{},{}", x, y, w, h),
        }
    }
}

impl FromStr for Region {
    type Err = ParameterError;

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

        let invalid = || ParameterError::InvalidRegion(s.into());

        match s {
            "full" => Ok(Region::Full),
            _ if let Some(xywh) = s.strip_prefix("pct:") => {
                let [x, y, w, h] = parse_xywh(xywh).ok_or_else(invalid)?;
                Ok(Region::Percentage { x, y, w, h })
            }
            _ => {
                let [x, y, w, h] = parse_xywh(s).ok_or_else(invalid)?;
                Ok(Region::Pixels { x, y, w, h })
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
/// assert_eq!(size, Size::BestFit { w: 640, h: 480 });
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
/// let size = Size::BestFit { w: 640, h: 480 };
/// assert_eq!(size.to_string(), "!640,480");
/// # Ok(())
/// # }
/// ```
///
/// [size]: https://iiif.io/api/image/2.0/#size
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
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
    /// Scale width and height to exactly this, ignoring aspect ratio.
    Exact {
        /// Width in pixels.
        w: u32,
        /// Height in pixels.
        h: u32,
    },
    /// Scale width and height to best fit.
    BestFit {
        /// Maximum width in pixels.
        w: u32,
        /// Maximum height in pixels.
        h: u32,
    },
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Size::Full => write!(f, "full"),
            Size::Width(width) => write!(f, "{},", width),
            Size::Height(height) => write!(f, ",{}", height),
            Size::Percentage(percentage) => write!(f, "pct:{}", percentage),
            Size::Exact { w, h } => write!(f, "{},{}", w, h),
            Size::BestFit { w, h } => write!(f, "!{},{}", w, h),
        }
    }
}

impl FromStr for Size {
    type Err = ParameterError;

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

        let invalid = || ParameterError::InvalidSize(s.into());

        match s {
            "full" => Ok(Size::Full),
            _ if let Some(n) = s.strip_prefix("pct:") => {
                Ok(Size::Percentage(n.trim().parse().map_err(|_| invalid())?))
            }
            _ if let Some(wh) = s.strip_prefix("!") => match parse_wh(wh).ok_or_else(invalid)? {
                [Some(w), Some(h)] => Ok(Size::BestFit { w, h }),
                _ => Err(ParameterError::InvalidSize(s.into())),
            },
            _ => match parse_wh(s).ok_or_else(invalid)? {
                [Some(w), None] => Ok(Size::Width(w)),
                [None, Some(h)] => Ok(Size::Height(h)),
                [Some(w), Some(h)] => Ok(Size::Exact { w, h }),
                [None, None] => Err(ParameterError::InvalidSize(s.into())),
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
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Rotation {
    /// Clockwise rotation in degrees.
    Degrees(Degrees),
    /// Mirrored rotation in degrees.
    Mirrored(Degrees),
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Degrees(Degrees::default())
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
    type Err = ParameterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ParameterError::InvalidRotation(s.into());

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
#[derive(Clone, Copy, Debug, PartialEq, Default, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    /// Full color.
    Color,
    /// Grayscale.
    Gray,
    /// Black-and-white.
    Bitonal,
    /// Server's choice, named `default` in the IIIF spec.
    ///
    /// This is also what the [`Default`] trait gives: `Quality::default()` returns
    /// `Quality::Default`.
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
    type Err = ParameterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == "color" => Ok(Quality::Color),
            _ if s == "gray" => Ok(Quality::Gray),
            _ if s == "bitonal" => Ok(Quality::Bitonal),
            _ if s == "default" => Ok(Quality::Default),
            _ => Err(ParameterError::InvalidQuality(s.into())),
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
#[derive(Clone, Copy, Debug, PartialEq, Default, Eq, PartialOrd, Ord, Hash)]
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
    Webp,
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
            Format::Webp => write!(f, "webp"),
        }
    }
}

impl FromStr for Format {
    type Err = ParameterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            _ if s == "jpg" => Ok(Format::Jpg),
            _ if s == "tif" => Ok(Format::Tif),
            _ if s == "png" => Ok(Format::Png),
            _ if s == "gif" => Ok(Format::Gif),
            _ if s == "jp2" => Ok(Format::Jp2),
            _ if s == "pdf" => Ok(Format::Pdf),
            _ if s == "webp" => Ok(Format::Webp),
            _ => Err(ParameterError::InvalidFormat(s.into())),
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
/// use iiif::{Degrees, Format, ImageRequest, Quality, Region, Rotation, Size, BaseUri};
///
/// # fn main() -> Result<()> {
/// let image_request = ImageRequest::new(
///     BaseUri::from_str("https://example.org/images/12345")?,
///     Region::Full,
///     Size::Width(1024),
///     Rotation::Degrees(Degrees::default()),
///     Quality::Default,
///     Format::Png,
/// );
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// Or you can construct one piece-wise with an [`ImageRequestBuilder`]. Only the base URI is
/// required; every other parameter has a default, giving `full/full/0/default.jpg`.
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// # use iiif::{Format, ImageRequest, Size, BaseUri};
/// #
/// # fn main() -> Result<()> {
/// let uri = BaseUri::from_str("https://example.org/images/12345")?;
/// let image_request = ImageRequest::builder().base_uri(uri.clone()).build();
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/full/0/default.jpg");
///
/// let image_request = ImageRequest::builder()
///     .size(Size::Width(1024))
///     .format(Format::Png)
///     .base_uri(uri)
///     .build();
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
/// # use iiif::{Format, ImageRequest, Quality, Region, Rotation, Size, BaseUri};
/// #
/// # fn main() -> Result<()> {
/// let image_request: ImageRequest = "https://example.org/images/12345/full/1024,/0/default.png".parse()?;
/// assert_eq!(image_request.to_string(), "https://example.org/images/12345/full/1024,/0/default.png");
/// # Ok(())
/// # }
/// ```
///
/// [image request]: https://iiif.io/api/image/2.0/#4-image-requests
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImageRequest {
    uri: BaseUri,
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
    type Err = RequestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.rsplitn(5, '/');

        let (quality, format) = parts
            .next()
            .and_then(|file| file.split_once('.'))
            .ok_or_else(|| RequestError::MissingFormat(s.into()))?;
        let quality = quality.parse()?;
        let format: Format = format.parse()?;

        let rotation = parts
            .next()
            .ok_or_else(|| RequestError::MissingRotation(s.into()))?
            .parse()?;

        let size = parts
            .next()
            .ok_or_else(|| RequestError::MissingSize(s.into()))?
            .parse()?;

        let region = parts
            .next()
            .ok_or_else(|| RequestError::MissingRegion(s.into()))?
            .parse()?;

        let uri = parts
            .next()
            .ok_or_else(|| RequestError::MissingBaseUri(s.into()))?
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
        uri: BaseUri,
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

    /// Returns a new builder with no base URI and every other parameter at its default.
    pub fn builder() -> ImageRequestBuilder {
        ImageRequestBuilder::default()
    }

    /// The base URI of the image service.
    pub fn base_uri(&self) -> &BaseUri {
        &self.uri
    }

    /// The region of the image to retrieve.
    pub fn region(&self) -> &Region {
        &self.region
    }

    /// How to scale the region.
    pub fn size(&self) -> &Size {
        &self.size
    }

    /// How to rotate and mirror the scaled region.
    pub fn rotation(&self) -> &Rotation {
        &self.rotation
    }

    /// The quality of the image.
    pub fn quality(&self) -> Quality {
        self.quality
    }

    /// The format of the image.
    pub fn format(&self) -> Format {
        self.format
    }
}

/// Marks an [`ImageRequestBuilder`] whose base URI hasn't been set yet.
///
/// Once [`ImageRequestBuilder::base_uri`] is called, the builder's type parameter becomes
/// [`BaseUri`] and [`ImageRequestBuilder::build`] becomes available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoBaseUri;

/// Builds an [`ImageRequest`] piece-wise. Create one with [`ImageRequest::builder`].
///
/// The type parameter `U` is the base URI: [`NoBaseUri`] until [`base_uri`](Self::base_uri) is
/// called, then [`BaseUri`]. The [typestate pattern] means [`build`](Self::build) only exists once
/// the base URI is set, so a request can't be built without one.
///
/// ```rust,compile_fail
/// let _ = iiif::ImageRequest::builder().build(); // no base URI
/// ```
///
/// Every other parameter starts at its default (`full`, `full`, `0`, `default`, `jpg`), and the
/// setters can be called in any order. You can name the builder's type, for example to share
/// common settings:
///
/// ```rust
/// # use anyhow::Result;
/// # use std::str::FromStr;
/// use iiif::{BaseUri, Format, ImageRequest, ImageRequestBuilder, NoBaseUri, Quality};
///
/// fn thumbnail() -> ImageRequestBuilder<NoBaseUri> {
///     ImageRequest::builder().size("!200,200".parse().unwrap()).quality(Quality::Gray)
/// }
///
/// # fn main() -> Result<()> {
/// let uri = BaseUri::from_str("https://example.org/images/12345")?;
/// let request = thumbnail().format(Format::Png).base_uri(uri).build();
/// assert_eq!(request.to_string(), "https://example.org/images/12345/full/!200,200/0/gray.png");
/// # Ok(())
/// # }
/// ```
///
/// [typestate pattern]: https://stanford-cs242.github.io/f19/lectures/08-2-typestate.html
#[derive(Clone, Debug)]
pub struct ImageRequestBuilder<U = NoBaseUri> {
    uri: U,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl Default for ImageRequestBuilder<NoBaseUri> {
    fn default() -> Self {
        Self {
            uri: NoBaseUri,
            region: Region::default(),
            size: Size::default(),
            rotation: Rotation::default(),
            quality: Quality::default(),
            format: Format::default(),
        }
    }
}

impl<U> ImageRequestBuilder<U> {
    /// Sets the base URI of the image service, replacing any previous one.
    pub fn base_uri(self, uri: BaseUri) -> ImageRequestBuilder<BaseUri> {
        ImageRequestBuilder {
            uri,
            region: self.region,
            size: self.size,
            rotation: self.rotation,
            quality: self.quality,
            format: self.format,
        }
    }

    /// Sets the region of the image to retrieve. Defaults to [`Region::Full`].
    pub fn region(mut self, region: Region) -> Self {
        self.region = region;
        self
    }

    /// Sets how to scale the region. Defaults to [`Size::Full`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets how to rotate and mirror the scaled region. Defaults to no rotation.
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets the quality of the image. Defaults to [`Quality::Default`].
    pub fn quality(mut self, quality: Quality) -> Self {
        self.quality = quality;
        self
    }

    /// Sets the format of the image. Defaults to [`Format::Jpg`].
    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }
}

impl ImageRequestBuilder<BaseUri> {
    /// Builds the request.
    pub fn build(self) -> ImageRequest {
        ImageRequest {
            uri: self.uri,
            region: self.region,
            size: self.size,
            rotation: self.rotation,
            quality: self.quality,
            format: self.format,
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
    #[case("1,2,3,4", Region::Pixels { x: 1, y: 2, w: 3, h: 4 })]
    #[case("pct:1,2,3,4", Region::Percentage { x: 1.0.try_into().unwrap(), y: 2.0.try_into().unwrap(), w: 3.0.try_into().unwrap(), h: 4.0.try_into().unwrap() })]
    #[case("pct:1.2,2.37,3.4,4.513", Region::Percentage { x: 1.2.try_into().unwrap(), y: 2.37.try_into().unwrap(), w: 3.4.try_into().unwrap(), h: 4.513.try_into().unwrap() })]
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
    #[case("640,480", Size::Exact { w: 640, h: 480 })]
    #[case("!640,480", Size::BestFit { w: 640, h: 480 })]
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
    #[case("webp", Format::Webp)]
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

    fn uri() -> BaseUri {
        BaseUri::from_str("https://www.artic.edu/iiif/2/abc").unwrap()
    }

    #[test]
    fn builder_defaults_to_full_default_jpg() {
        let request = ImageRequest::builder().base_uri(uri()).build();
        assert_eq!(
            request.to_string(),
            "https://www.artic.edu/iiif/2/abc/full/full/0/default.jpg"
        );
    }

    #[test]
    fn builder_and_new_build_the_same_url() {
        let region = Region::Pixels {
            x: 10,
            y: 20,
            w: 300,
            h: 400,
        };
        let size = Size::from_str("843,").unwrap();
        let built = ImageRequest::builder()
            .base_uri(uri())
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

    #[test]
    fn accessors_return_parts() {
        let request = ImageRequest::from_str(
            "https://www.artic.edu/iiif/2/abc/pct:10,10,50,50/!100,100/!90/gray.webp",
        )
        .unwrap();
        assert_eq!(request.base_uri(), &uri());
        assert_eq!(
            request.region(),
            &Region::from_str("pct:10,10,50,50").unwrap()
        );
        assert_eq!(request.size(), &Size::BestFit { w: 100, h: 100 });
        assert_eq!(request.rotation(), &Rotation::from_str("!90").unwrap());
        assert_eq!(request.quality(), Quality::Gray);
        assert_eq!(request.format(), Format::Webp);
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
