use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{
        Modifier, Style,
        palette::tailwind::{GRAY, GREEN, SLATE},
    },
    text::Line,
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState},
};

use super::{ACCENT, Component, centered};
use crate::action::Action;

/// The ramp preset the TUI starts with.
pub const DEFAULT_RAMP: &str = "alphabet";

/// The overlay listing `charaoke`'s ramp presets, opened with `r`.
///
/// The ramp in use is green; `enter` switches to the highlighted one and leaves the list open,
/// so ramps can be compared one after another.
pub struct Ramps {
    names: Vec<&'static str>,
    /// The index of the ramp in use.
    current: usize,
    state: ListState,
    /// Where the last draw put the overlay, so a click outside it can close it.
    pub area: Rect,
}

impl Default for Ramps {
    fn default() -> Self {
        Self::new()
    }
}

impl Ramps {
    pub fn new() -> Self {
        let names: Vec<&'static str> = charaoke::Ramp::PRESETS
            .iter()
            .map(|&(name, _)| name)
            .collect();
        let current = names
            .iter()
            .position(|&name| name == DEFAULT_RAMP)
            .expect("the default is a preset");
        Self {
            names,
            current,
            state: ListState::default().with_selected(Some(current)),
            area: Rect::default(),
        }
    }

    /// Moves the highlight by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) {
        let at = self.state.selected().unwrap_or(self.current);
        let last = self.names.len() - 1;
        self.state
            .select(Some(at.saturating_add_signed(delta).min(last)));
    }

    /// Switches to the highlighted ramp.
    pub fn choose(&mut self) -> Option<Action> {
        let i = self.state.selected()?;
        self.current = i;
        Some(Action::SetRamp(self.names[i].to_string()))
    }
}

impl Component for Ramps {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let widest = self.names.iter().map(|name| name.len()).max().unwrap_or(0);
        let width = u16::try_from(widest).unwrap_or(u16::MAX) + 4;
        let height = u16::try_from(self.names.len()).unwrap_or(u16::MAX) + 2;
        let overlay = centered(area, width.max(9), height);
        self.area = overlay;
        let items: Vec<ListItem> = self
            .names
            .iter()
            .enumerate()
            .map(|(i, &name)| {
                let color = if i == self.current {
                    GREEN.c500
                } else {
                    GRAY.c500
                };
                ListItem::new(Line::styled(name, color))
            })
            .collect();
        let list = List::new(items)
            .block(
                Block::bordered()
                    .border_style(Style::new().fg(ACCENT))
                    .title(" Ramps "),
            )
            .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
            .highlight_symbol("-")
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_widget(Clear, overlay);
        frame.render_stateful_widget(list, overlay, &mut self.state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn renders_ramps_overlay() {
        let mut ramps = Ramps::new();
        insta::assert_snapshot!(render(&mut ramps, 40, 14));
    }

    #[test]
    fn choose_sets_the_highlighted_ramp() {
        let mut ramps = Ramps::new();
        ramps.step(1);
        assert_eq!(
            ramps.choose(),
            Some(Action::SetRamp(charaoke::Ramp::PRESETS[1].0.into()))
        );
    }

    #[test]
    fn step_stops_at_the_ends() {
        let mut ramps = Ramps::new();
        ramps.step(-100);
        assert_eq!(ramps.state.selected(), Some(0));
        ramps.step(100);
        assert_eq!(ramps.state.selected(), Some(ramps.names.len() - 1));
    }
}
