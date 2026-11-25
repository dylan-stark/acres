use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::Paragraph,
};

use super::{Component, pane_block};

/// The one-line search bar, with earlier queries recalled by `up` and `down`.
#[derive(Default)]
pub struct Search {
    query: String,
    /// Queries run so far, oldest first, without repeats in a row.
    history: Vec<String>,
    /// The history entry `up`/`down` last recalled; `None` while editing a new query.
    recalled: Option<usize>,
    pub focused: bool,
}

impl Search {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn push(&mut self, c: char) {
        self.query.push(c);
        self.recalled = None;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.recalled = None;
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.recalled = None;
    }

    /// Shows `query` in the bar, as when going back to an earlier list.
    pub fn set(&mut self, query: &str) {
        self.query = query.to_string();
        self.recalled = None;
    }

    /// Records the query in history and returns it.
    pub fn submit(&mut self) -> String {
        let query = self.query.trim().to_string();
        if !query.is_empty() && self.history.last() != Some(&query) {
            self.history.push(query.clone());
        }
        self.recalled = None;
        query
    }

    /// Replaces the query with the one before the last recalled, or the newest.
    pub fn earlier(&mut self) {
        let at = match self.recalled {
            Some(0) => return,
            Some(i) => i - 1,
            None => match self.history.len().checked_sub(1) {
                Some(i) => i,
                None => return,
            },
        };
        self.recall(Some(at));
    }

    /// Replaces the query with the one after the last recalled, or clears it past the newest.
    pub fn later(&mut self) {
        let Some(i) = self.recalled else { return };
        if i + 1 < self.history.len() {
            self.recall(Some(i + 1));
        } else {
            self.recall(None);
        }
    }

    fn recall(&mut self, at: Option<usize>) {
        self.recalled = at;
        self.query = at.map(|i| self.history[i].clone()).unwrap_or_default();
    }
}

impl Component for Search {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = pane_block(1, "Search", self.focused);
        let inner = block.inner(area);
        let line = if self.query.is_empty() && !self.focused {
            Line::from("/ to search titles").fg(Color::DarkGray)
        } else {
            Line::from(self.query.as_str())
        };
        frame.render_widget(Paragraph::new(line).block(block), area);
        if self.focused {
            let width = u16::try_from(self.query.chars().count()).unwrap_or(u16::MAX);
            let x = inner
                .x
                .saturating_add(width)
                .min(inner.right().saturating_sub(1));
            frame.set_cursor_position(Position::new(x, inner.y));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    fn typed(text: &str) -> Search {
        let mut search = Search::default();
        text.chars().for_each(|c| search.push(c));
        search
    }

    #[test]
    fn renders_a_hint_until_focused() {
        let mut search = Search::default();
        insta::assert_snapshot!(render(&mut search, 30, 3));
        search.focused = true;
        search.push('m');
        let text = render(&mut search, 30, 3);
        assert!(text.lines().nth(1).unwrap().starts_with("│m "), "{text}");
    }

    #[test]
    fn recalls_earlier_queries_newest_first() {
        let mut search = typed("monet");
        assert_eq!(search.submit(), "monet");
        search.clear();
        "wave".chars().for_each(|c| search.push(c));
        search.submit();
        search.submit();
        search.clear();

        search.earlier();
        assert_eq!(search.query(), "wave", "repeats are kept once");
        search.earlier();
        assert_eq!(search.query(), "monet");
        search.earlier();
        assert_eq!(search.query(), "monet", "stays at the oldest");
        search.later();
        assert_eq!(search.query(), "wave");
        search.later();
        assert_eq!(search.query(), "", "past the newest is a new query");
    }

    #[test]
    fn backspace_and_clear_edit_the_query() {
        let mut search = typed("lilies");
        search.backspace();
        assert_eq!(search.query(), "lilie");
        search.clear();
        assert_eq!(search.query(), "");
        assert_eq!(search.submit(), "", "an empty query is not kept");
        search.earlier();
        assert_eq!(search.query(), "");
    }
}
