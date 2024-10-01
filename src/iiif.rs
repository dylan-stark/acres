#![allow(dead_code, clippy::upper_case_acronyms)]
use std::fmt;

enum Format {
    JPG,
    PNG,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Format::JPG => write!(f, "jpg"),
            Format::PNG => write!(f, "png"),
        }
    }
}

enum Region {
    Full,
    Pixels(usize, usize, usize, usize),
    Percent(f32, f32, f32, f32),
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Region::Full => write!(f, "full"),
            Region::Pixels(x, y, w, h) => write!(f, "{x},{y},{w},{h}"),
            Region::Percent(x, y, w, h) => write!(f, "pct:{x},{y},{w},{h}"),
        }
    }
}

enum Quality {
    Bitonal,
    Color,
    Default,
    Gray,
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Quality::Bitonal => write!(f, "bitonal"),
            Quality::Color => write!(f, "color"),
            Quality::Default => write!(f, "default"),
            Quality::Gray => write!(f, "gray"),
        }
    }
}

enum Rotation {
    Degrees(f32),
}

impl fmt::Display for Rotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rotation::Degrees(d) => write!(f, "{d}"),
        }
    }
}
enum Size {
    Width(usize),
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Size::Width(w) => write!(f, "{w},"),
        }
    }
}

pub struct Iiif2Url {
    pub identifier: Option<String>,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl Iiif2Url {
    pub fn new() -> Self {
        Iiif2Url {
            identifier: None,
            region: Region::Full,
            size: Size::Width(843),
            rotation: Rotation::Degrees(0.0),
            quality: Quality::Default,
            format: Format::JPG,
        }
    }

    pub fn identifier(mut self, identifier: &str) -> Iiif2Url {
        self.identifier = Some(identifier.to_string());
        self
    }

    fn region(mut self, region: Region) -> Iiif2Url {
        self.region = region;
        self
    }

    fn size(mut self, size: Size) -> Iiif2Url {
        self.size = size;
        self
    }

    fn rotation(mut self, rotation: Rotation) -> Iiif2Url {
        self.rotation = rotation;
        self
    }

    fn quality(mut self, quality: Quality) -> Iiif2Url {
        self.quality = quality;
        self
    }

    fn format(mut self, format: Format) -> Iiif2Url {
        self.format = format;
        self
    }
}

impl fmt::Display for Iiif2Url {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "https://www.artic.edu/iiif/2/{}/{}/{}/{}/{}.{}",
            self.identifier.as_ref().unwrap(),
            self.region,
            self.size,
            self.rotation,
            self.quality,
            self.format
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_iiif_url_with_defaults() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_full_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Full)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_pixel_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Pixels(0, 0, 64, 64))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/0,0,64,64/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_percent_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Percent(50.0, 50.5, 25.0, 25.5))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/pct:50,50.5,25,25.5/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_size() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .size(Size::Width(640))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/640,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_rotation() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .rotation(Rotation::Degrees(42.7))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/42.7/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_quality() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .quality(Quality::Gray)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/gray.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_format() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .format(Format::PNG)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.png"
        );
    }
}
