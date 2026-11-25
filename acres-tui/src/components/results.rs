use acres::artworks::ArtworkInfo;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, palette::tailwind::SLATE},
    widgets::{HighlightSpacing, List, ListItem, ListState},
};

use super::{Component, pane_block};

/// The results pane: the artworks loaded, narrowed by the last search, with one selected.
#[derive(Default)]
pub struct Results {
    artworks: Vec<ArtworkInfo>,
    /// Indexes into `artworks` that match the last search, in order.
    shown: Vec<usize>,
    state: ListState,
    /// Rows the list showed in its last draw, for half-page moves.
    rows: u16,
    /// Where the last draw put the list, for mapping clicks to rows.
    area: Rect,
    pub focused: bool,
}

impl Results {
    /// Lists every artwork with an image, the first selected.
    pub fn new(artworks: acres::artworks::Artworks) -> Self {
        let artworks: Vec<ArtworkInfo> = artworks.into();
        let mut results = Self {
            shown: (0..artworks.len()).collect(),
            artworks,
            ..Self::default()
        };
        results
            .state
            .select((!results.shown.is_empty()).then_some(0));
        results
    }

    /// The selected artwork.
    pub fn selected(&self) -> Option<&ArtworkInfo> {
        let i = *self.shown.get(self.state.selected()?)?;
        self.artworks.get(i)
    }

    /// How many artworks match the last search, and how many are loaded.
    pub fn counts(&self) -> (usize, usize) {
        (self.shown.len(), self.artworks.len())
    }

    /// Keeps only artworks whose title contains `query`, ignoring case. An empty query shows all.
    ///
    /// Returns the newly selected artwork when the selection changed.
    pub fn filter(&mut self, query: &str) -> Option<ArtworkInfo> {
        let before = self.selected().cloned();
        let query = query.to_lowercase();
        self.shown = (0..self.artworks.len())
            .filter(|&i| self.artworks[i].data.title.to_lowercase().contains(&query))
            .collect();
        self.state = ListState::default();
        self.state.select((!self.shown.is_empty()).then_some(0));
        self.changed(before)
    }

    /// Moves the selection by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) -> Option<ArtworkInfo> {
        let last = self.shown.len().checked_sub(1)?;
        let before = self.selected().cloned();
        let at = self.state.selected().unwrap_or(0);
        self.state
            .select(Some(at.saturating_add_signed(delta).min(last)));
        self.changed(before)
    }

    /// Selects the first row.
    pub fn top(&mut self) -> Option<ArtworkInfo> {
        self.step(isize::MIN)
    }

    /// Selects the last row.
    pub fn bottom(&mut self) -> Option<ArtworkInfo> {
        self.step(isize::MAX)
    }

    /// Rows in half the list's height, at least one.
    pub fn half_page(&self) -> isize {
        isize::try_from((self.rows / 2).max(1)).unwrap_or(1)
    }

    /// Selects the row at screen position `(column, row)`, if one is there.
    pub fn click(&mut self, column: u16, row: u16) -> Option<ArtworkInfo> {
        let inner = self.area;
        if !inner.contains((column, row).into()) {
            return None;
        }
        let index = self.state.offset() + usize::from(row - inner.y);
        if index >= self.shown.len() {
            return None;
        }
        let before = self.selected().cloned();
        self.state.select(Some(index));
        self.changed(before)
    }

    fn changed(&self, before: Option<ArtworkInfo>) -> Option<ArtworkInfo> {
        let after = self.selected()?;
        (before.as_ref() != Some(after)).then(|| after.clone())
    }
}

impl Component for Results {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = pane_block(2, "Results", self.focused);
        self.area = block.inner(area);
        self.rows = self.area.height;
        let items: Vec<ListItem> = self
            .shown
            .iter()
            .map(|&i| ListItem::new(self.artworks[i].data.title.as_str()))
            .collect();
        let highlight = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
        let list = List::new(items)
            .block(block)
            .highlight_style(highlight)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, area, &mut self.state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::{ARTWORKS_FIXTURE, render};

    fn results() -> Results {
        Results::new(serde_json::from_str(ARTWORKS_FIXTURE).expect("fixture parses"))
    }

    fn title(artwork: Option<ArtworkInfo>) -> Option<String> {
        artwork.map(|a| a.data.title)
    }

    #[test]
    fn renders_the_list_with_the_first_selected() {
        let mut results = results();
        results.focused = true;
        insta::assert_snapshot!(render(&mut results, 40, 8));
    }

    #[test]
    fn steps_report_only_changes_and_stop_at_the_ends() {
        let mut results = results();
        assert_eq!(title(results.step(1)), Some("The Bedroom".into()));
        assert_eq!(results.top().map(|a| a.data.id), Some(27992));
        assert_eq!(results.step(-1), None, "already at the top");
        let last = results.bottom().expect("moved");
        assert_eq!(results.step(1), None, "already at the bottom");
        assert_eq!(results.selected(), Some(&last));
    }

    #[test]
    fn filter_matches_titles_ignoring_case() {
        let mut results = results();
        assert_eq!(title(results.filter("WAVE")), Some("The Great Wave".into()));
        assert_eq!(results.counts(), (1, 5));
        assert_eq!(results.filter("nothing like it"), None);
        assert_eq!(results.selected(), None);
        assert_eq!(results.step(1), None);
        results.filter("");
        assert_eq!(results.counts(), (5, 5));
    }

    #[test]
    fn click_selects_the_row_under_the_pointer() {
        let mut results = results();
        render(&mut results, 40, 8);
        // Row 0 is the border, so screen row 3 is the third artwork.
        assert_eq!(title(results.click(5, 3)), Some("Water Lilies".into()));
        assert_eq!(results.click(5, 3), None, "already selected");
        assert_eq!(results.click(5, 0), None, "the border");
    }
}
