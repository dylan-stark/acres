//! Stage benchmarks for `charaoke`.
//!
//! Each pipeline stage is measured on its own, over the same inputs and output widths as the
//! `image-to-ascii` baseline in `image-to-ascii-builder/benches/baseline.rs`, so the two reports
//! line up. Run with `cargo bench -p charaoke`.
//!
//! - `charaoke/decode` and `charaoke/luma`: JPEG decode and `to_luma8` in `image` 0.25, for reference.
//! - `charaoke/load`: [`Converter::set_image`] from decoded RGB8, paid once per image.
//! - `charaoke/render`: [`Converter::render`] into [`Cells`] at each width, paid on every resize.
//! - `charaoke/ansi`: [`Cells::write_ansi`] at each width and color depth, the CLI's output step,
//!   comparable to `baseline/color`.
//! - `charaoke/end_to_end`: decode, load, render and truecolor ANSI, comparable to
//!   `baseline/end_to_end`.

mod common;

use std::hint::black_box;

use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use image::{DynamicImage, ImageFormat};

use common::{COLS, cols, decode as decode_rgb, dimensions, inputs};

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

fn load(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/load");
    let mut converter = Converter::default();
    for (name, bytes) in inputs() {
        let image = decode_rgb(&bytes);
        let (w, h) = dimensions(&image);
        group.throughput(Throughput::Elements(u64::from(w.get() * h.get())));
        group.bench_with_input(BenchmarkId::from_parameter(name), &image, |b, image| {
            b.iter(|| converter.set_image(black_box(image), w, h, PixelFormat::Rgb8))
        });
    }
    group.finish();
}

fn render(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/render");
    let mut converter = Converter::default();
    let mut cells = Cells::default();
    for (name, bytes) in inputs() {
        let image = decode_rgb(&bytes);
        let (w, h) = dimensions(&image);
        converter
            .set_image(&image, w, h, PixelFormat::Rgb8)
            .expect("loads");
        for &n in COLS {
            let grid = Grid::fit(w, h, cols(n));
            group.throughput(Throughput::Elements(grid.cells()));
            group.bench_with_input(BenchmarkId::new(name, n), &grid, |b, &grid| {
                b.iter(|| converter.render(black_box(grid), &mut cells))
            });
        }
    }
    group.finish();
}

fn ansi(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/ansi");
    let mut converter = Converter::default();
    let mut cells = Cells::default();
    let mut out = String::new();
    for (name, bytes) in inputs() {
        let image = decode_rgb(&bytes);
        let (w, h) = dimensions(&image);
        converter
            .set_image(&image, w, h, PixelFormat::Rgb8)
            .expect("loads");
        for &n in COLS {
            let grid = Grid::fit(w, h, cols(n));
            converter.render(grid, &mut cells);
            group.throughput(Throughput::Elements(grid.cells()));
            for (depth_name, depth) in [
                ("truecolor", ColorDepth::TrueColor),
                ("256", ColorDepth::Ansi256),
            ] {
                let id = BenchmarkId::new(format!("{name}/{depth_name}"), n);
                group.bench_with_input(id, &cells, |b, cells| {
                    b.iter(|| black_box(cells).write_ansi(depth, &mut out))
                });
            }
        }
    }
    group.finish();
}

fn end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/end_to_end");
    let mut converter = Converter::default();
    let mut cells = Cells::default();
    let mut out = String::new();
    for (name, bytes) in inputs() {
        for &n in COLS {
            group.bench_with_input(BenchmarkId::new(name, n), &bytes, |b, bytes| {
                b.iter(|| {
                    let image = decode_rgb(black_box(bytes));
                    let (w, h) = dimensions(&image);
                    converter
                        .set_image(&image, w, h, PixelFormat::Rgb8)
                        .expect("loads");
                    converter.render(Grid::fit(w, h, cols(n)), &mut cells);
                    cells.write_ansi(ColorDepth::TrueColor, &mut out);
                })
            });
        }
    }
    group.finish();
}

criterion_group!(benches, decode, luma, load, render, ansi, end_to_end);
criterion_main!(benches);
