use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{ACCENT, Component};

/// The one-line status bar: how many artworks are listed, a few keys for the focused pane, and
/// the last key pressed with what it did.
#[derive(Default)]
pub struct Status {
    /// What the list holds, such as `5 artworks` or `1 / 5 artworks`.
    pub counts: String,
    /// Keys worth knowing in the focused pane, as `(keys, description)`.
    pub hints: Vec<(String, &'static str)>,
    /// The last key and what it did, such as `n · next artwork`.
    pub echo: Option<String>,
}

impl Component for Status {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let quiet = Style::new().fg(Color::DarkGray);
        let mut hints = vec![Span::raw(" ")];
        for (keys, description) in &self.hints {
            hints.push(Span::raw(keys.as_str()).bold());
            hints.push(Span::styled(format!(" {description}  "), quiet));
        }
        let echo = self.echo.as_deref().unwrap_or_default();
        let echo_width = u16::try_from(echo.chars().count() + 1).unwrap_or(u16::MAX);
        let counts_width = u16::try_from(self.counts.chars().count() + 2).unwrap_or(u16::MAX);
        let [counts, hints_area, echo_area] = Layout::horizontal([
            Constraint::Length(counts_width),
            Constraint::Fill(1),
            Constraint::Length(echo_width),
        ])
        .areas(area);
        frame.render_widget(Paragraph::new(format!(" {}", self.counts)), counts);
        frame.render_widget(Paragraph::new(Line::from(hints)), hints_area);
        frame.render_widget(
            Paragraph::new(Line::from(echo).fg(ACCENT)).right_aligned(),
            echo_area,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn renders_counts_hints_and_the_last_key() {
        let mut status = Status {
            counts: "5 artworks".into(),
            hints: vec![("?".into(), "help"), ("j / k".into(), "move")],
            echo: Some("x · not mapped here".into()),
        };
        insta::assert_snapshot!(render(&mut status, 70, 1));
    }
}
