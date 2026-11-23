//! A headless stand-in for the TUI's resize path.
//!
//! Each iteration does what `acres-tui` will do on a terminal resize: render the loaded image to
//! fit the new pane, then draw it with Ratatui into an in-memory `TestBackend`. The pane width
//! cycles through a drag from 120 to 200 columns, so every frame is a new size. This measures
//! conversion plus Ratatui's layout and buffer work, though not the terminal's own output.
//! Run with `cargo bench -p charaoke --bench tui_proxy`.
//!
//! - `charaoke/tui_proxy/drag`: render into [`Cells`], then write each cell's character and
//!   truecolor straight into Ratatui's `Buffer` (the path ADR 0010 chooses).
//! - `charaoke/tui_proxy/drag_ansi`: render, write truecolor ANSI, parse it back with
//!   `ansi-to-tui` and draw it as a `Paragraph` (today's TUI path), to show what the direct
//!   write saves.

mod common;

use std::hint::black_box;

use ansi_to_tui::IntoText;
use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Paragraph, Widget},
};

use common::{cols, decode, dimensions, inputs};

/// Pane widths visited by one simulated drag, in columns.
const DRAG: std::ops::RangeInclusive<u16> = 120..=200;

/// Terminal height in rows, enough for the tallest pane in the drag.
const TERMINAL_ROWS: u16 = 80;

/// Draws [`Cells`] straight into a Ratatui buffer, one character and foreground color per cell.
///
/// A copy of the widget `acres-tui` gets in step 4, kept here until then so the bench measures the
/// same work.
struct CellsWidget<'a>(&'a Cells);

impl Widget for CellsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.0.cols();
        if cols == 0 {
            return;
        }
        let rows = self
            .0
            .chars()
            .chunks(cols)
            .zip(self.0.colors().chunks(cols));
        for ((chars, colors), y) in rows.zip(area.top()..area.bottom()) {
            for ((&ch, &[r, g, b]), x) in chars.iter().zip(colors).zip(area.left()..area.right()) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(char::from(ch)).set_fg(Color::Rgb(r, g, b));
                }
            }
        }
    }
}

fn resize_drag(c: &mut Criterion) {
    let mut group = c.benchmark_group("charaoke/tui_proxy");
    let widths: Vec<u16> = DRAG.collect();
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
