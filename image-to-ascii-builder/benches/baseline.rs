//! Baseline benchmarks for the `image-to-ascii` path that `charaoke` replaces.
//!
//! Stages are measured separately (decode, luminance, font loading, conversion, coloring) and
//! end to end, plus `baseline/ansi_parse`, the `ansi-to-tui` parse the TUI runs on the colored
//! string, over the same inputs and output widths as `charaoke/benches/stages.rs`. Run with
//! `cargo bench -p image-to-ascii-builder`.

use std::{hint::black_box, io::Cursor, path::PathBuf};

use ansi_to_tui::IntoText;
use bytes::{Buf, Bytes};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use image::{DynamicImage, ImageFormat, ImageOutputFormat, imageops::FilterType};
use image_to_ascii_builder::{Alphabet, Ascii, CharWidth, ConversionAlgorithm, Font, Metric};
use img_to_ascii::{
    convert::{
        char_rows_to_terminal_color_string, get_conversion_algorithm, get_converter,
        img_to_char_rows,
    },
    font,
    image::LumaImage,
};

/// A 320x240 artwork image from the TUI test fixtures.
const FIXTURE: &str =
    "../acres-tui/tests/fixtures/iiif/e966799b-97ee-1cc6-bd2f-a94b4b8bb8f9/full/843,/0/default.jpg";

/// Width the TUI requests from the AIC IIIF server (`full/843,/0/default.jpg`).
const AIC_WIDTH: u32 = 843;

/// Output widths in characters: a small pane, a typical pane and a wide full screen.
const COLS: &[usize] = &[80, 160, 320];

/// Algorithms to compare: the cheapest and the default.
const ALGORITHMS: &[ConversionAlgorithm] = &[
    ConversionAlgorithm::Base,
    ConversionAlgorithm::EdgeAugmented,
];

/// Named JPEG inputs: the fixture as-is, and upscaled to the width the TUI fetches.
fn inputs() -> Vec<(&'static str, Bytes)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let small = std::fs::read(&path).expect("fixture image is readable");
    let image = decoded(&small);
    let height = image.height() * AIC_WIDTH / image.width();
    let large = image.resize_exact(AIC_WIDTH, height, FilterType::Lanczos3);
    let mut encoded = Cursor::new(Vec::new());
    large
        .write_to(&mut encoded, ImageOutputFormat::Jpeg(90))
        .expect("encodes");
    vec![
        ("320x240", Bytes::from(small)),
        ("843x632", Bytes::from(encoded.into_inner())),
    ]
}

fn decoded(bytes: &[u8]) -> DynamicImage {
    image::load_from_memory_with_format(bytes, ImageFormat::Jpeg).expect("decodes")
}

fn load_font() -> font::Font {
    font::Font::from_bdf_stream(
        Bytes::from(Font::default()).reader(),
        &Vec::from(Alphabet::default()),
        false,
    )
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline/decode");
    for (name, bytes) in inputs() {
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &bytes, |b, bytes| {
            b.iter(|| image::load_from_memory_with_format(black_box(bytes), ImageFormat::Jpeg))
        });
    }
    group.finish();
}

fn luma(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline/luma");
    for (name, bytes) in inputs() {
        let image = decoded(&bytes);
        group.throughput(Throughput::Elements(u64::from(
            image.width() * image.height(),
        )));
        group.bench_with_input(BenchmarkId::from_parameter(name), &image, |b, image| {
            b.iter(|| LumaImage::<f32>::from(black_box(image)))
        });
    }
    group.finish();
}

fn font_load(c: &mut Criterion) {
    c.bench_function("baseline/font", |b| b.iter(load_font));
}

fn convert(c: &mut Criterion) {
    let font = load_font();
    let metric = Metric::default().to_string();
    for algorithm in ALGORITHMS {
        let algo = get_conversion_algorithm(&algorithm.to_string());
        let mut group = c.benchmark_group(format!("baseline/convert/{algorithm}"));
        for (name, bytes) in inputs() {
            let luma = LumaImage::<f32>::from(&decoded(&bytes));
            for &cols in COLS {
                group.bench_with_input(BenchmarkId::new(name, cols), &cols, |b, &cols| {
                    b.iter(|| {
                        img_to_char_rows(
                            &font,
                            black_box(&luma),
                            get_converter(&metric),
                            Some(cols),
                            0.0,
                            &algo,
                        )
                    })
                });
            }
        }
        group.finish();
    }
}

fn color(c: &mut Criterion) {
    let font = load_font();
    let metric = Metric::default().to_string();
    let algo = get_conversion_algorithm(&ConversionAlgorithm::Base.to_string());
    let mut group = c.benchmark_group("baseline/color");
    for (name, bytes) in inputs() {
        let image = decoded(&bytes);
        let luma = LumaImage::<f32>::from(&image);
        for &cols in COLS {
            let rows =
                img_to_char_rows(&font, &luma, get_converter(&metric), Some(cols), 0.0, &algo);
            group.bench_with_input(BenchmarkId::new(name, cols), &rows, |b, rows| {
                b.iter(|| char_rows_to_terminal_color_string(black_box(rows), &image))
            });
        }
    }
    group.finish();
}

fn ansi_parse(c: &mut Criterion) {
    let font = load_font();
    let metric = Metric::default().to_string();
    let algo = get_conversion_algorithm(&ConversionAlgorithm::Base.to_string());
    let mut group = c.benchmark_group("baseline/ansi_parse");
    for (name, bytes) in inputs() {
        let image = decoded(&bytes);
        let luma = LumaImage::<f32>::from(&image);
        for &cols in COLS {
            let rows =
                img_to_char_rows(&font, &luma, get_converter(&metric), Some(cols), 0.0, &algo);
            let ansi = char_rows_to_terminal_color_string(&rows, &image);
            group.bench_with_input(BenchmarkId::new(name, cols), &ansi, |b, ansi| {
                b.iter(|| black_box(ansi.as_bytes()).into_text().expect("parses"))
            });
        }
    }
    group.finish();
}

fn end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline/end_to_end");
    for (name, bytes) in inputs() {
        for &cols in COLS {
            group.bench_with_input(BenchmarkId::new(name, cols), &bytes, |b, bytes| {
                b.iter(|| {
                    Ascii::builder()
                        .input(black_box(bytes.clone()))
                        .chars_wide(CharWidth::CharsWide(cols))
                        .build()
                        .expect("converts")
                })
            });
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = decode, luma, font_load, convert, color, ansi_parse, end_to_end
}
criterion_main!(benches);
