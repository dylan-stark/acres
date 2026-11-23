//! The bench copy of the `acres-tui` widget that draws [`Cells`] into a Ratatui buffer.
//!
//! `acres-tui` includes this file in a unit test that checks its own widget draws the same buffer,
//! so the `drag` benches keep measuring the TUI's real work.

use charaoke::Cells;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

/// Draws [`Cells`] straight into a Ratatui buffer, one character and foreground color per cell.
///
/// Truecolor only, as the benches draw.
pub struct CellsWidget<'a>(pub &'a Cells);

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
