//! Stage benchmarks for `charaoke`.
//!
//! Each pipeline stage is measured on its own, over the same inputs and output widths as the
//! `image-to-ascii` baseline in `image-to-ascii-builder/benches/baseline.rs`, so the two reports
//! line up. Run with `cargo bench -p charaoke`.
//!
//! Only decoding and luminance exist so far; conversion stages join as they are written.

use std::{hint::black_box, io::Cursor, path::PathBuf};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use image::{DynamicImage, ImageFormat, codecs::jpeg::JpegEncoder, imageops::FilterType};

/// A 320x240 artwork image from the TUI test fixtures.
const FIXTURE: &str =
    "../acres-tui/tests/fixtures/iiif/e966799b-97ee-1cc6-bd2f-a94b4b8bb8f9/full/843,/0/default.jpg";

/// Width the TUI requests from the AIC IIIF server (`full/843,/0/default.jpg`).
const AIC_WIDTH: u32 = 843;

/// Named JPEG inputs: the fixture as-is, and upscaled to the width the TUI fetches.
fn inputs() -> Vec<(&'static str, Vec<u8>)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let small = std::fs::read(&path).expect("fixture image is readable");
    let image = image::load_from_memory_with_format(&small, ImageFormat::Jpeg).expect("decodes");
    let height = image.height() * AIC_WIDTH / image.width();
    let large = image.resize_exact(AIC_WIDTH, height, FilterType::Lanczos3);
    let mut encoded = Cursor::new(Vec::new());
    large
        .write_with_encoder(JpegEncoder::new_with_quality(&mut encoded, 90))
        .expect("encodes");
    vec![("320x240", small), ("843x632", encoded.into_inner())]
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/decode");
    for (name, bytes) in inputs() {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &bytes, |b, bytes| {
            b.iter(|| image::load_from_memory_with_format(black_box(bytes), ImageFormat::Jpeg))
        });
    }
    group.finish();
}

fn luma(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/luma");
    for (name, bytes) in inputs() {
        let image: DynamicImage =
            image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg).expect("decodes");
        group.throughput(Throughput::Elements(u64::from(
            image.width() * image.height(),
        )));
        group.bench_with_input(BenchmarkId::from_parameter(name), &image, |b, image| {
            b.iter(|| black_box(image).to_luma8())
        });
    }
    group.finish();
}

criterion_group!(benches, decode, luma);
criterion_main!(benches);
