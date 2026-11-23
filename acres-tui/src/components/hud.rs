use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Clear, Paragraph},
};

use super::Component;
use crate::action::{Action, Timing};

/// Width of the HUD drawer, in columns.
pub const WIDTH: u16 = 34;

/// Events the feed keeps.
const CAPACITY: usize = 200;

/// The timing HUD, toggled with `t`: a drawer over the right edge of the art.
///
/// It draws after the art, over it, so opening it never refits or re-renders the art. Every event
/// is kept while it is closed, so opening it shows what happened before.
#[derive(Default)]
pub struct Hud {
    shown: bool,
    feed: VecDeque<Timing>,
    /// Whether the next frame time belongs in the feed: true after an event that did real work.
    awaiting_frame: bool,
    /// How long the HUD's own last draw took.
    last_draw: Duration,
}

impl Hud {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, timing: Timing) {
        if self.feed.len() == CAPACITY {
            self.feed.pop_front();
        }
        self.feed.push_back(timing);
    }

    fn lines(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::new();
        for timing in &self.feed {
            lines.push(Line::styled(timing.label.as_str(), Style::new().bold()));
            for (stage, duration) in &timing.stages {
                lines.push(Line::raw(format!(
                    "  {stage:<8}{:>9.2} ms",
                    duration.as_secs_f64() * 1e3
                )));
            }
        }
        lines
    }
}

impl Component for Hud {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::ToggleHud => {
                self.shown = !self.shown;
                self.awaiting_frame = true;
            }
            Action::Timing(timing) => {
                self.push(timing);
                self.awaiting_frame = true;
            }
            Action::FrameTime(total) if self.awaiting_frame => {
                let mut timing = Timing::new("frame").stage("total", total);
                if self.shown {
                    timing = timing.stage("hud", self.last_draw);
                }
                self.push(timing);
                self.awaiting_frame = false;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if !self.shown {
            return Ok(());
        }
        let start = Instant::now();
        let width = WIDTH.min(area.width);
        let drawer = Rect::new(area.right() - width, area.top(), width, area.height);
        let lines = self.lines();
        let visible = usize::from(drawer.height.saturating_sub(2));
        let lines = lines[lines.len().saturating_sub(visible)..].to_vec();
        frame.render_widget(Clear, drawer);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .title(" timings ")
                    .border_style(Style::new().fg(Color::DarkGray)),
            ),
            drawer,
        );
        self.last_draw = start.elapsed();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    fn ms(ms: f64) -> Duration {
        Duration::from_secs_f64(ms / 1e3)
    }

    #[test]
    fn draws_nothing_until_toggled() {
        let mut hud = Hud::new();
        hud.update(Action::Timing(Timing::new("image 1"))).unwrap();
        assert!(render(&mut hud, 50, 10).trim().is_empty());
    }

    #[test]
    fn renders_feed_newest_at_the_bottom() {
        let mut hud = Hud::new();
        hud.update(Action::Timing(
            Timing::new("image 77333")
                .stage("fetch", ms(212.0))
                .stage("decode", ms(1.31))
                .stage("load", ms(0.04)),
        ))
        .unwrap();
        hud.update(Action::FrameTime(ms(0.2))).unwrap();
        hud.update(Action::Timing(
            Timing::new("resize 160x48")
                .stage("render", ms(0.52))
                .stage("draw", ms(0.41)),
        ))
        .unwrap();
        hud.update(Action::FrameTime(ms(0.98))).unwrap();
        // Frames that follow no event stay out of the feed.
        hud.update(Action::FrameTime(ms(0.5))).unwrap();
        hud.update(Action::ToggleHud).unwrap();
        insta::assert_snapshot!(render(&mut hud, 50, 16));
    }

    #[test]
    fn feed_keeps_the_latest_events() {
        let mut hud = Hud::new();
        for i in 0..CAPACITY + 5 {
            hud.update(Action::Timing(Timing::new(format!("event {i}"))))
                .unwrap();
        }
        assert_eq!(hud.feed.len(), CAPACITY);
        assert_eq!(hud.feed[0].label, "event 5");
    }

    #[test]
    fn frame_after_toggle_includes_the_hud() {
        let mut hud = Hud::new();
        hud.update(Action::ToggleHud).unwrap();
        hud.update(Action::FrameTime(ms(1.0))).unwrap();
        let stages: Vec<&str> = hud.feed[0].stages.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(stages, ["total", "hud"]);
    }
}
