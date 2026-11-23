//! Inputs shared by the `charaoke` benches. Each bench uses only part of it.
#![allow(dead_code)]

use std::{io::Cursor, num::NonZeroU32, path::PathBuf};

use image::{ImageFormat, RgbImage, codecs::jpeg::JpegEncoder, imageops::FilterType};

/// A 320x240 artwork image from the TUI test fixtures.
const FIXTURE: &str =
    "../acres-tui/tests/fixtures/iiif/e966799b-97ee-1cc6-bd2f-a94b4b8bb8f9/full/843,/0/default.jpg";

/// Width the TUI requests from the AIC IIIF server (`full/843,/0/default.jpg`).
const AIC_WIDTH: u32 = 843;

/// Output widths in characters, matching the `image-to-ascii` baseline.
pub const COLS: &[u32] = &[80, 160, 320];

/// Named JPEG inputs: the fixture as-is, and upscaled to the width the TUI fetches.
pub fn inputs() -> Vec<(&'static str, Vec<u8>)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let small = std::fs::read(&path).expect("fixture image is readable");
    let image = decode(&small);
    let height = image.height() * AIC_WIDTH / image.width();
    let large =
        image::DynamicImage::ImageRgb8(image).resize_exact(AIC_WIDTH, height, FilterType::Lanczos3);
    let mut encoded = Cursor::new(Vec::new());
    large
        .write_with_encoder(JpegEncoder::new_with_quality(&mut encoded, 90))
        .expect("encodes");
    vec![("320x240", small), ("843x632", encoded.into_inner())]
}

/// Decodes a JPEG to RGB8, as the TUI will before handing pixels to `charaoke`.
pub fn decode(bytes: &[u8]) -> RgbImage {
    image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
        .expect("decodes")
        .into_rgb8()
}

/// Width and height as the non-zero values `charaoke` takes.
pub fn dimensions(image: &RgbImage) -> (NonZeroU32, NonZeroU32) {
    let nz = |n| NonZeroU32::new(n).expect("image is not empty");
    (nz(image.width()), nz(image.height()))
}

/// `n` as a non-zero column count.
pub fn cols(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("column count is not zero")
}
