//! Helpers for render tests.

use ratatui::{Terminal, backend::TestBackend};

use super::Component;

/// Offline artworks fixture shared by render tests and the demo.
pub const ARTWORKS_FIXTURE: &str = include_str!("../../tests/fixtures/artworks.json");

/// Renders `component` into a `width` x `height` buffer and returns it as text, one line per row.
pub fn render(component: &mut impl Component, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test terminal");
    terminal
        .draw(|frame| {
            component
                .draw(frame, frame.area())
                .expect("component draws")
        })
        .expect("frame draws");
    let buffer = terminal.backend().buffer();
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .map(|row| row.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}
