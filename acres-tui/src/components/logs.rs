use std::collections::VecDeque;

use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Clear, Paragraph},
};

use super::Component;
use crate::action::Action;

/// Width of the log drawer, in columns, when the pane is wide enough.
pub const WIDTH: u16 = 48;

/// Lines the log keeps.
const CAPACITY: usize = 500;

/// The log, toggled with `L`: a drawer over the left edge of the art, the way the timing HUD
/// slides over the right edge.
///
/// Like the HUD it draws over the art, so opening it never refits or re-renders the art. It
/// records every action except the per-frame ones, plus [`Action::Log`] and [`Action::Error`]
/// messages, and keeps them while it is closed.
#[derive(Default)]
pub struct Logs {
    shown: bool,
    lines: VecDeque<String>,
}

impl Logs {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, line: String) {
        if self.lines.len() == CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }
}

impl Component for Logs {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::ToggleLogs => self.shown = !self.shown,
            Action::Tick | Action::Render | Action::FrameTime(_) | Action::Timing(_) => {}
            Action::Log(message) => self.push(message),
            Action::Error(message) => self.push(format!("error: {message}")),
            action => self.push(format!("action: {action}")),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.shown {
            return Ok(());
        }
        let width = WIDTH.min(area.width);
        let drawer = Rect::new(area.left(), area.top(), width, area.height);
        let visible = usize::from(drawer.height.saturating_sub(2));
        let lines: Vec<Line> = self
            .lines
            .iter()
            .skip(self.lines.len().saturating_sub(visible))
            .map(|line| Line::raw(line.as_str()))
            .collect();
        frame.render_widget(Clear, drawer);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .title(" logs ")
                    .border_style(Style::new().fg(Color::DarkGray)),
            ),
            drawer,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn draws_nothing_until_toggled() {
        let mut logs = Logs::new();
        logs.update(Action::Log("first".into())).unwrap();
        assert!(render(&mut logs, 30, 6).trim().is_empty());
    }

    #[test]
    fn renders_the_latest_lines_and_skips_frame_actions() {
        let mut logs = Logs::new();
        logs.update(Action::Log("first".into())).unwrap();
        logs.update(Action::Tick).unwrap();
        logs.update(Action::Render).unwrap();
        logs.update(Action::CycleArtMode).unwrap();
        logs.update(Action::Error("no image".into())).unwrap();
        logs.update(Action::Log("last".into())).unwrap();
        logs.update(Action::ToggleLogs).unwrap();
        // Three rows inside the border: the oldest line is scrolled off.
        insta::assert_snapshot!(render(&mut logs, 40, 5));
    }

    #[test]
    fn keeps_the_latest_lines() {
        let mut logs = Logs::new();
        for i in 0..CAPACITY + 3 {
            logs.update(Action::Log(format!("line {i}"))).unwrap();
        }
        assert_eq!(logs.lines.len(), CAPACITY);
        assert_eq!(logs.lines[0], "line 3");
    }
}
