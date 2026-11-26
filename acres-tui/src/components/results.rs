use std::collections::HashSet;

use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, palette::tailwind::SLATE},
    text::{Line, Span},
    widgets::{HighlightSpacing, List, ListItem, ListState},
};

use super::{Component, pane_block};
use crate::entry::Entry;

/// Marks a row whose image is in the disk cache.
pub const CACHED: &str = "•";

/// Marks a row whose image would need a fetch.
pub const NOT_CACHED: &str = "◦";

/// The results pane: the artworks loaded, narrowed by the last search, with one selected.
///
/// A dim [`CACHED`] or [`NOT_CACHED`] before each title says whether its image is on disk
/// (ADR 0018), and saved artworks are marked with `*`. The same pane lists the saved list, titled
/// Saved.
#[derive(Default, Clone)]
pub struct Results {
    artworks: Vec<Entry>,
    /// Indexes into `artworks` that match the last search, in order.
    shown: Vec<usize>,
    state: ListState,
    /// Rows the list showed in its last draw, for half-page moves.
    rows: u16,
    /// Where the last draw put the list, for mapping clicks to rows.
    area: Rect,
    /// Ids of saved artworks, marked in the list.
    marked: HashSet<u32>,
    /// Ids of artworks whose image is in the disk cache.
    cached: HashSet<u32>,
    /// Whether this lists the saved artworks, which titles the pane Saved.
    pub saved_view: bool,
    pub focused: bool,
}

impl Results {
    /// Lists `artworks`, the first selected.
    pub fn new(artworks: Vec<Entry>) -> Self {
        let mut results = Self::default();
        results.replace(artworks);
        results
    }

    /// Lists `artworks` instead, from the top, and returns the one now selected.
    pub fn replace(&mut self, artworks: Vec<Entry>) -> Option<Entry> {
        self.shown = (0..artworks.len()).collect();
        self.artworks = artworks;
        self.state = ListState::default();
        self.state.select((!self.shown.is_empty()).then_some(0));
        self.selected().cloned()
    }

    /// Adds the next page of `artworks` below the rest, keeping the selection.
    pub fn append(&mut self, artworks: Vec<Entry>) {
        let start = self.artworks.len();
        self.artworks.extend(artworks);
        self.shown.extend(start..self.artworks.len());
        if self.state.selected().is_none() && !self.shown.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// How many rows are below the selected one.
    pub fn rows_below(&self) -> usize {
        let at = self.state.selected().unwrap_or(0);
        self.shown.len().saturating_sub(at + 1)
    }

    /// Every artwork loaded, shown or not.
    pub fn artworks(&self) -> &[Entry] {
        &self.artworks
    }

    /// The selected artwork.
    pub fn selected(&self) -> Option<&Entry> {
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
    pub fn filter(&mut self, query: &str) -> Option<Entry> {
        let before = self.selected().cloned();
        let query = query.to_lowercase();
        self.shown = (0..self.artworks.len())
            .filter(|&i| self.artworks[i].title().to_lowercase().contains(&query))
            .collect();
        self.state = ListState::default();
        self.state.select((!self.shown.is_empty()).then_some(0));
        self.changed(before)
    }

    /// Moves the selection by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) -> Option<Entry> {
        let last = self.shown.len().checked_sub(1)?;
        let before = self.selected().cloned();
        let at = self.state.selected().unwrap_or(0);
        self.state
            .select(Some(at.saturating_add_signed(delta).min(last)));
        self.changed(before)
    }

    /// Selects the first row.
    pub fn top(&mut self) -> Option<Entry> {
        self.step(isize::MIN)
    }

    /// Selects the last row.
    pub fn bottom(&mut self) -> Option<Entry> {
        self.step(isize::MAX)
    }

    /// Rows in half the list's height, at least one.
    pub fn half_page(&self) -> isize {
        isize::try_from((self.rows / 2).max(1)).unwrap_or(1)
    }

    /// Selects the row at screen position `(column, row)`, if one is there.
    pub fn click(&mut self, column: u16, row: u16) -> Option<Entry> {
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

    /// Marks these artworks as saved.
    pub fn mark(&mut self, saved: impl IntoIterator<Item = u32>) {
        self.marked = saved.into_iter().collect();
    }

    /// Marks these artworks as having their image cached.
    pub fn mark_cached(&mut self, cached: &HashSet<u32>) {
        self.cached.clone_from(cached);
    }

    /// Selects the artwork `id` if it is listed, else the row where the selection was, so
    /// removing a row moves the selection to the next one. Returns the artwork now selected.
    pub fn reselect(&mut self, id: Option<u32>, row: usize) -> Option<Entry> {
        let found = id.and_then(|id| self.shown.iter().position(|&i| self.artworks[i].id() == id));
        let at = found.or_else(|| (!self.shown.is_empty()).then(|| row.min(self.shown.len() - 1)));
        self.state.select(at);
        self.selected().cloned()
    }

    /// The selected row, from 0.
    pub fn selected_row(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    fn changed(&self, before: Option<Entry>) -> Option<Entry> {
        let after = self.selected()?;
        (before.as_ref() != Some(after)).then(|| after.clone())
    }
}

impl Component for Results {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let name = if self.saved_view { "Saved" } else { "Results" };
        let block = pane_block(2, name, self.focused);
        self.area = block.inner(area);
        self.rows = self.area.height;
        let dim = Style::new().fg(SLATE.c500);
        let items: Vec<ListItem> = self
            .shown
            .iter()
            .map(|&i| {
                let artwork = &self.artworks[i];
                let glyph = if artwork.summary.image_id.is_none() {
                    " "
                } else if self.cached.contains(&artwork.id()) {
                    CACHED
                } else {
                    NOT_CACHED
                };
                let star = if !self.saved_view && self.marked.contains(&artwork.id()) {
                    "* "
                } else {
                    ""
                };
                ListItem::new(Line::from(vec![
                    Span::styled(glyph, dim),
                    Span::raw(" "),
                    Span::raw(star),
                    Span::raw(artwork.title()),
                ]))
            })
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
        Results::new(Entry::from_file(
            serde_json::from_str(ARTWORKS_FIXTURE).expect("fixture parses"),
        ))
    }

    fn title(artwork: Option<Entry>) -> Option<String> {
        artwork.map(|a| a.title().to_string())
    }

    #[test]
    fn renders_the_list_with_the_first_selected() {
        let mut results = results();
        results.focused = true;
        insta::assert_snapshot!(render(&mut results, 40, 8));
    }

    #[test]
    fn rows_say_whether_their_image_is_cached() {
        let mut results = results();
        let ids: Vec<u32> = results.artworks().iter().map(Entry::id).collect();
        results.mark([ids[1]]);
        results.mark_cached(&HashSet::from([ids[0], ids[1]]));
        insta::assert_snapshot!(render(&mut results, 40, 8));
    }

    #[test]
    fn steps_report_only_changes_and_stop_at_the_ends() {
        let mut results = results();
        assert_eq!(title(results.step(1)), Some("The Bedroom".into()));
        assert_eq!(results.top().map(|a| a.id()), Some(27992));
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
    fn append_adds_a_page_below_and_keeps_the_selection() {
        let mut results = results();
        results.step(1);
        let more = results.artworks.clone();
        results.append(more);
        assert_eq!(results.counts(), (10, 10));
        assert_eq!(
            title(results.selected().cloned()),
            Some("The Bedroom".into())
        );
        assert_eq!(results.rows_below(), 8);
        assert_eq!(
            title(results.replace(Vec::new())),
            None,
            "replacing with nothing selects nothing"
        );
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
