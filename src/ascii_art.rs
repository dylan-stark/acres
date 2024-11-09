use std::io::Cursor;

use bytes::Bytes;
use image::DynamicImage;

use img_to_ascii::{
    convert::{
        char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
        img_to_char_rows,
    },
    font::Font,
    image::LumaImage,
};

const ALPHABET: &[u8] = include_bytes!("../.data/alphabet.txt");
const BITOCRA_13: &[u8] = include_bytes!("../.data/bitocra-13.bdf");

#[derive(Clone, Default)]
pub struct ArtBuilder {
    pub width: u16,
    pub height: u16,
    bytes: Bytes,
    image: DynamicImage,
    ascii: String,
}

impl ArtBuilder {
    pub fn into_ascii(self) -> String {
        if self.bytes.is_empty() || self.width == 0 || self.height == 0 {
            return String::from("");
        }
        self.ascii.clone()
    }

    pub fn with_bytes(mut self, bytes: Bytes) -> ArtBuilder {
        self.bytes = bytes;
        self.image = bytes_as_dyn_image(self.bytes.clone()).unwrap();
        if self.width != 0 && self.height != 0 {
            self.ascii = image_as_ascii(self.image.clone(), self.width, self.height);
        }
        self
    }

    pub fn with_size(mut self, width: u16, height: u16) -> ArtBuilder {
        if self.width == width && self.height == height {
            return self;
        }
        if !self.bytes.is_empty() {
            self.ascii = image_as_ascii(self.image.clone(), width, height);
        }
        self.width = width;
        self.height = height;
        self
    }
}

fn bytes_as_dyn_image(bytes: Bytes) -> std::result::Result<image::DynamicImage, image::ImageError> {
    let reader = image::io::Reader::new(Cursor::new(bytes))
        .with_guessed_format()
        .expect("cursor io never fails");
    reader.decode()
}

fn image_as_ascii(image: DynamicImage, width: u16, height: u16) -> String {
    let font = font();
    let image_area = PixelArea {
        width: image.width() as usize,
        height: image.height() as usize,
    };
    let frame_area = CharArea {
        width: width as usize,
        height: height as usize,
    };
    let resized_image_area = image_area.fit_into(frame_area.into_pixels(font.clone()));
    let target_image_width_in_chars = CharArea::from_pixels(resized_image_area, font);
    dyn_image_as_ascii(image, Some(target_image_width_in_chars.width))
}

struct CharArea {
    width: usize,
    height: usize,
}

impl CharArea {
    fn from_pixels(pixels: PixelArea, font: Font) -> CharArea {
        CharArea {
            width: CharArea::round_up(pixels.width, font.width) / font.width,
            height: CharArea::round_up(pixels.height, font.height) / font.width,
        }
    }

    fn into_pixels(self, font: Font) -> PixelArea {
        PixelArea {
            width: self.width * font.width,
            height: self.height * font.height,
        }
    }

    /// Rounds length up to next multiple of segment.
    fn round_up(length: usize, segment: usize) -> usize {
        if length % segment == 0 {
            length
        } else {
            length + 1
        }
    }
}

struct PixelArea {
    width: usize,
    height: usize,
}

impl PixelArea {
    fn fit_into(self, frame: PixelArea) -> PixelArea {
        let scaling_ratio = if frame.width > frame.height {
            // Scale image to frame height
            self.height as f64 / frame.height as f64
        } else {
            // Scale self to frame width
            self.width as f64 / frame.width as f64
        };
        PixelArea {
            width: (self.width as f64 / scaling_ratio) as usize,
            height: (self.height as f64 / scaling_ratio) as usize,
        }
    }
}

fn font() -> Font {
    let alphabet = &ALPHABET.iter().map(|&c| c as char).collect::<Vec<char>>();
    Font::from_bdf_stream(BITOCRA_13, alphabet)
}

fn dyn_image_as_ascii(dyn_img: image::DynamicImage, out_width: Option<usize>) -> String {
    let font = font();

    let luma_img = LumaImage::from(&dyn_img);

    let metric = String::from("direction-and-intensity");
    let convert = get_converter(&metric);

    let brightness_offset = 0.0;

    let algorithm = String::from("edge-augmented");
    let algorithm = get_conversion_algorithm(&algorithm);

    let char_rows = img_to_char_rows(
        &font,
        &luma_img,
        convert,
        out_width,
        brightness_offset,
        &algorithm,
    );

    char_rows_to_terminal_color_string(&char_rows, &dyn_img)
}
#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_default() {
        ArtBuilder::default();
    }

    #[test]
    fn test_size_sets_width_and_height() {
        let (some_width, some_height) = (10, 10);

        let builder = ArtBuilder::default().with_size(some_width, some_height);

        assert_eq!(builder.width, some_width);
        assert_eq!(builder.height, some_height);
    }

    #[test]
    fn test_size_updates_width_and_height() {
        let (some_width, some_height) = (10, 10);
        let builder = ArtBuilder::default().with_size(42, 42);

        let builder = builder.with_size(some_width, some_height);

        assert_eq!(builder.width, some_width);
        assert_eq!(builder.height, some_height);
    }
}
