//! IIIF.

mod iiif_builder;

use std::fmt::Display;

pub use self::iiif_builder::IiifBuilder;

/// Region of an image.
#[derive(Clone, Debug, Default)]
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

impl TryFrom<String> for Region {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        try_from_str_region(&value)
    }
}

/// Region parser.
pub fn try_from_str_region(value: &str) -> Result<Region, String> {
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

/// An IIIF instance.
#[derive(Clone, Debug, PartialEq)]
pub struct Iiif(pub String);

impl Iiif {
    /// Returns a new builder.
    pub fn builder() -> IiifBuilder {
        IiifBuilder::default()
    }
}
