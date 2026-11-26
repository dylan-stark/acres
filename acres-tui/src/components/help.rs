use std::collections::BTreeMap;

use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::{
    ACCENT, Component, centered,
    results::{CACHED, NOT_CACHED},
};
use crate::{
    action::{Action, ActionDiscriminants},
    config::{Context, KeyBindings, key_event_to_string},
};

/// Widest the overlay gets, in columns.
const WIDTH: u16 = 56;

/// The `?` overlay: every binding for the focused context, then the global ones.
///
/// It is built from the loaded keymap, user overrides included, so it cannot drift from what
/// the keys really do.
#[derive(Default)]
pub struct Help {
    lines: Vec<Line<'static>>,
    scroll: u16,
    /// Rows the overlay showed in its last draw, to bound scrolling.
    rows: u16,
    /// Where the last draw put the overlay, so a click outside it can close it.
    pub area: Rect,
}

impl Help {
    /// Lists the bindings for `context`, then `Global`, from the top.
    pub fn open(&mut self, keybindings: &KeyBindings, context: Context) {
        let mut lines = Vec::new();
        let mut contexts = vec![context];
        if context != Context::Global {
            contexts.push(Context::Global);
        }
        for context in contexts {
            let entries = entries(keybindings, context);
            if entries.is_empty() {
                continue;
            }
            if !lines.is_empty() {
                lines.push(Line::default());
            }
            lines.push(Line::from(context.title()).bold().fg(ACCENT));
            let width = entries
                .iter()
                .map(|(keys, _)| keys.len())
                .max()
                .unwrap_or(0);
            for (keys, description) in entries {
                lines.push(Line::from(vec![
                    Span::raw(format!("  {keys:<width$}  ")).bold(),
                    Span::raw(description),
                ]));
            }
        }
        if context == Context::List {
            lines.push(Line::default());
            lines.push(Line::from("Marks").bold().fg(ACCENT));
            lines.push(Line::from(format!(
                "  {CACHED} image on disk  {NOT_CACHED} needs a fetch  * saved"
            )));
        }
        self.lines = lines;
        self.scroll = 0;
    }

    pub fn scroll(&mut self, delta: isize) {
        let max = self.lines.len().saturating_sub(usize::from(self.rows));
        let to = usize::from(self.scroll)
            .saturating_add_signed(delta)
            .min(max);
        self.scroll = u16::try_from(to).unwrap_or(u16::MAX);
    }
}

/// Each action bound in `context`, in help order, with every key that does it.
fn entries(keybindings: &KeyBindings, context: Context) -> Vec<(String, &'static str)> {
    let mut by_action: BTreeMap<ActionDiscriminants, (&Action, Vec<String>)> = BTreeMap::new();
    for (keys, action) in keybindings.get(&context).into_iter().flatten() {
        if action.description().is_none() {
            continue;
        }
        let keys: Vec<String> = keys.iter().map(key_event_to_string).collect();
        let entry = by_action
            .entry(ActionDiscriminants::from(action))
            .or_insert((action, Vec::new()));
        entry.1.push(keys.join(" "));
    }
    by_action
        .into_values()
        .map(|(action, mut keys)| {
            // Single characters first, then named keys, each alphabetically.
            keys.sort_by_key(|key| (key.chars().count() > 1, key.clone()));
            let description = action.description().expect("filtered above");
            (keys.join(", "), description)
        })
        .collect()
}

impl Component for Help {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let height = u16::try_from(self.lines.len()).unwrap_or(u16::MAX);
        let overlay = centered(area, WIDTH, height.saturating_add(2));
        self.area = overlay;
        self.rows = overlay.height.saturating_sub(2);
        self.scroll(0);
        let block = Block::bordered()
            .border_style(Style::new().fg(ACCENT))
            .title(" Keys ")
            .title_bottom(Line::from(" ? or esc to close ").fg(Color::DarkGray));
        frame.render_widget(Clear, overlay);
        frame.render_widget(
            Paragraph::new(self.lines.clone())
                .scroll((self.scroll, 0))
                .block(block),
            overlay,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{components::test_support::render, config::Config};

    fn help(context: Context) -> Help {
        let config = Config::new().expect("default config");
        let mut help = Help::default();
        help.open(&config.keybindings, context);
        help
    }

    #[test]
    fn renders_the_default_keymap_for_the_results() {
        let mut help = help(Context::List);
        insta::assert_snapshot!(render(&mut help, 60, 50));
    }

    #[test]
    fn lists_the_focused_context_before_global() {
        let help = help(Context::Search);
        let titles: Vec<String> = help
            .lines
            .iter()
            .filter(|line| line.spans.len() == 1)
            .map(|line| line.to_string())
            .collect();
        assert_eq!(titles, ["Search bar", "Anywhere"]);
    }

    #[test]
    fn merges_keys_that_do_the_same_thing() {
        let help = help(Context::List);
        let down = help
            .lines
            .iter()
            .map(|line| line.to_string())
            .find(|line| line.ends_with(" move down"))
            .expect("a line for moving down");
        assert!(down.trim_start().starts_with("j, down "), "{down}");
    }

    #[test]
    fn scrolls_when_it_does_not_fit() {
        let mut help = help(Context::List);
        render(&mut help, 60, 10);
        help.scroll(1000);
        let bottom = help.scroll;
        assert_eq!(usize::from(bottom), help.lines.len() - 8);
        help.scroll(-1000);
        assert_eq!(help.scroll, 0);
    }
}
