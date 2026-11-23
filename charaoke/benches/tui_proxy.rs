//! A headless stand-in for the TUI's resize path.
//!
//! Each iteration does what `acres-tui` does on a terminal resize: render the loaded image to fit
//! the new pane, then draw it with Ratatui into an in-memory `TestBackend`. The pane width cycles
//! through a drag from 120 to 200 columns, so every frame is a new size. This measures conversion
//! plus Ratatui's layout and buffer work, though not the terminal's own output.
//! Run with `cargo bench -p charaoke --bench tui_proxy`.
//!
//! - `charaoke/tui_proxy/drag`: render into [`Cells`], then write each cell's character and
//!   truecolor straight into Ratatui's `Buffer` (the TUI's path since ADR 0011).
//! - `charaoke/tui_proxy/drag_fit`: the same, but the drag changes both width and height and the
//!   grid comes from `Grid::fit_within`, as the TUI fits art to its pane.
//! - `charaoke/tui_proxy/drag_ansi`: render, write truecolor ANSI, parse it back with
//!   `ansi-to-tui` and draw it as a `Paragraph` (the TUI's path before ADR 0011), to show what
//!   the direct write saves.

#[path = "common/cells_widget.rs"]
mod cells_widget;
mod common;

use std::hint::black_box;

use ansi_to_tui::IntoText;
use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, widgets::Paragraph};

use cells_widget::CellsWidget;
use common::{cols, decode, dimensions, inputs};

/// Pane widths visited by one simulated drag, in columns.
const DRAG: std::ops::RangeInclusive<u16> = 120..=200;

/// Terminal height in rows, enough for the tallest pane in the drag.
const TERMINAL_ROWS: u16 = 80;

/// Pane heights visited by the `drag_fit` drag, in rows.
const DRAG_ROWS: std::ops::RangeInclusive<u16> = 30..=60;

fn resize_drag(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/tui_proxy");
    let widths: Vec<u16> = DRAG.collect();
    let heights: Vec<u16> = DRAG_ROWS.collect();
    let mut terminal =
        Terminal::new(TestBackend::new(*DRAG.end(), TERMINAL_ROWS)).expect("test terminal");
    let mut converter = Converter::default();
    let mut cells = Cells::default();
    let mut ansi = String::new();

    for (name, bytes) in inputs() {
        let image = decode(&bytes);
        let (w, h) = dimensions(&image);
        converter
            .set_image(&image, w, h, PixelFormat::Rgb8)
            .expect("loads");
        let pane = |frame: usize| {
            let width = widths[frame % widths.len()];
            let grid = Grid::fit(w, h, cols(u32::from(width)));
            let rows = u16::try_from(grid.rows())
                .unwrap_or(u16::MAX)
                .min(TERMINAL_ROWS);
            (grid, Rect::new(0, 0, width, rows))
        };

        let mut frame = 0;
        group.bench_function(BenchmarkId::new("drag", name), |b| {
            b.iter(|| {
                let (grid, area) = pane(frame);
                frame += 1;
                converter.render(black_box(grid), &mut cells);
                terminal
                    .draw(|f| f.render_widget(CellsWidget(&cells), area))
                    .expect("draws");
            })
        });

        // Width and height step at different rates, so the drag is diagonal and varied.
        let fit_pane = |frame: usize| {
            let width = widths[frame % widths.len()];
            let height = heights[frame * 7 % heights.len()];
            let grid = Grid::fit_within(w, h, cols(u32::from(width)), cols(u32::from(height)));
            let area = Rect::new(0, 0, grid.cols() as u16, grid.rows() as u16);
            (grid, area)
        };

        let mut frame = 0;
        group.bench_function(BenchmarkId::new("drag_fit", name), |b| {
            b.iter(|| {
                let (grid, area) = fit_pane(frame);
                frame += 1;
                converter.render(black_box(grid), &mut cells);
                terminal
                    .draw(|f| f.render_widget(CellsWidget(&cells), area))
                    .expect("draws");
            })
        });

        let mut frame = 0;
        group.bench_function(BenchmarkId::new("drag_ansi", name), |b| {
            b.iter(|| {
                let (grid, area) = pane(frame);
                frame += 1;
                converter.render(black_box(grid), &mut cells);
                cells.write_ansi(ColorDepth::TrueColor, &mut ansi);
                let text = ansi.as_bytes().into_text().expect("parses");
                terminal
                    .draw(|f| f.render_widget(Paragraph::new(text), area))
                    .expect("draws");
            })
        });
    }
    group.finish();
}

criterion_group!(benches, resize_drag);
criterion_main!(benches);
