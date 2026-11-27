use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use super::{ACCENT, Component};

/// Widest the zen popup gets, in columns.
const POPUP_WIDTH: u16 = 72;

/// Draws `alert` in a small box at the bottom of `area`, for zen, where there is no status bar.
pub fn draw_popup(frame: &mut Frame, area: Rect, alert: &str) {
    let text = Line::from(vec![
        Span::raw("error: ").fg(Color::Red).bold(),
        Span::raw(alert).fg(Color::Red),
    ]);
    let width = POPUP_WIDTH.min(area.width);
    let inner = usize::from(width.saturating_sub(2).max(1));
    let rows = text.width().div_ceil(inner).max(1);
    let height = (u16::try_from(rows).unwrap_or(u16::MAX) + 2).min(area.height);
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.bottom().saturating_sub(height + 1).max(area.y),
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .block(Block::bordered().border_style(Style::new().fg(Color::Red))),
        popup,
    );
}

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
    /// The last failure of something asked for, with its hint. Shown instead of the echo.
    pub alert: Option<String>,
}

impl Component for Status {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let quiet = Style::new().fg(Color::DarkGray);
        let mut hints = vec![Span::raw(" ")];
        for (keys, description) in &self.hints {
            hints.push(Span::raw(keys.as_str()).bold());
            hints.push(Span::styled(format!(" {description}  "), quiet));
        }
        let echo = match &self.alert {
            Some(alert) => Line::from(vec![
                Span::raw("error: ").fg(Color::Red).bold(),
                Span::raw(alert.as_str()).fg(Color::Red),
            ]),
            None => Line::from(self.echo.as_deref().unwrap_or_default()).fg(ACCENT),
        };
        let echo_width = u16::try_from(echo.width() + 1).unwrap_or(u16::MAX);
        let counts_width = u16::try_from(self.counts.chars().count() + 2).unwrap_or(u16::MAX);
        let [counts, hints_area, echo_area] = Layout::horizontal([
            Constraint::Length(counts_width),
            Constraint::Fill(1),
            Constraint::Length(echo_width),
        ])
        .areas(area);
        frame.render_widget(Paragraph::new(format!(" {}", self.counts)), counts);
        frame.render_widget(Paragraph::new(Line::from(hints)), hints_area);
        frame.render_widget(Paragraph::new(echo).right_aligned(), echo_area);
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
            alert: None,
        };
        insta::assert_snapshot!(render(&mut status, 70, 1));
    }

    #[test]
    fn a_failure_shows_instead_of_the_last_key() {
        let mut status = Status {
            counts: "5 artworks".into(),
            hints: vec![("?".into(), "help")],
            echo: Some("o · open on artic.edu".into()),
            alert: Some("not cached while offline: x · go online".into()),
        };
        insta::assert_snapshot!(render(&mut status, 70, 1));
    }
}
