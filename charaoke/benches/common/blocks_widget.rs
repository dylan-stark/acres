//! The bench copy of the `acres-tui` widget that draws [`Blocks`] into a Ratatui buffer.
//!
//! `acres-tui` includes this file in a unit test that checks its own widget draws the same buffer,
//! so the `drag_half` and `drag_quadrant` benches keep measuring the TUI's real work.

use charaoke::Blocks;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

/// Draws [`Blocks`] straight into a Ratatui buffer, one glyph, foreground and background per cell.
///
/// Truecolor only, as the benches draw.
pub struct BlocksWidget<'a>(pub &'a Blocks);

impl Widget for BlocksWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.0.cols();
        if cols == 0 {
            return;
        }
        let rows = self
            .0
            .masks()
            .chunks(cols)
            .zip(self.0.foreground().chunks(cols))
            .zip(self.0.background().chunks(cols));
        for (((masks, fgs), bgs), y) in rows.zip(area.top()..area.bottom()) {
            let cells = masks.iter().zip(fgs).zip(bgs);
            for (((&mask, &[fr, fg, fb]), &[br, bg, bb]), x) in cells.zip(area.left()..area.right())
            {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(charaoke::QUADRANTS[usize::from(mask)])
                        .set_fg(Color::Rgb(fr, fg, fb))
                        .set_bg(Color::Rgb(br, bg, bb));
                }
            }
        }
    }
}
