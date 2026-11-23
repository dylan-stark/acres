//! A headless stand-in for the TUI's resize path.
//!
//! Each iteration does what `acres-tui` will do on a terminal resize: render the loaded image to
//! fit the new pane, then draw it with Ratatui into an in-memory `TestBackend`. The pane width
//! cycles through a drag from 120 to 200 columns, so every frame is a new size. This measures
//! conversion plus Ratatui's layout and buffer work, though not the terminal's own output.
//! Run with `cargo bench -p charaoke --bench tui_proxy`.

mod common;

use std::hint::black_box;

use charaoke::{Converter, Grid, PixelFormat};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, widgets::Paragraph};

use common::{cols, decode, dimensions, inputs};

/// Pane widths visited by one simulated drag, in columns.
const DRAG: std::ops::RangeInclusive<u16> = 120..=200;

/// Terminal height in rows, enough for the tallest pane in the drag.
const TERMINAL_ROWS: u16 = 80;

fn resize_drag(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/tui_proxy");
    let widths: Vec<u16> = DRAG.collect();
    let mut terminal =
        Terminal::new(TestBackend::new(*DRAG.end(), TERMINAL_ROWS)).expect("test terminal");
    let mut converter = Converter::default();
    let mut out = String::new();

    for (name, bytes) in inputs() {
        let image = decode(&bytes);
        let (w, h) = dimensions(&image);
        converter
            .set_image(&image, w, h, PixelFormat::Rgb8)
            .expect("loads");
        let mut frame = 0;
        group.bench_function(BenchmarkId::new("drag", name), |b| {
            b.iter(|| {
                let width = widths[frame % widths.len()];
                frame += 1;
                let grid = Grid::fit(w, h, cols(u32::from(width)));
                converter.render(black_box(grid), &mut out);
                let rows = u16::try_from(grid.rows())
                    .unwrap_or(u16::MAX)
                    .min(TERMINAL_ROWS);
                terminal
                    .draw(|f| {
                        f.render_widget(Paragraph::new(out.as_str()), Rect::new(0, 0, width, rows))
                    })
                    .expect("draws");
            })
        });
    }
    group.finish();
}

criterion_group!(benches, resize_drag);
criterion_main!(benches);
