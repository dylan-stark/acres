use acres::artworks::request::search::Filters;
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize, palette::tailwind::SLATE},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::{ACCENT, Component, catalog::DATED_YEARS, centered};
use crate::failure::{self, Failure};

/// The departments and artwork types the API lists, to choose from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facets {
    /// `(id, title)`, such as `("PC-10", "Painting and Sculpture of Europe")`.
    pub departments: Vec<(String, String)>,
    /// `(id, title)`, such as `(1, "Painting")`.
    pub types: Vec<(u32, String)>,
}

/// One line of the filter overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Image,
    PublicDomain,
    OnView,
    Department,
    Type,
    From,
    To,
}

const ROWS: [Row; 7] = [
    Row::Image,
    Row::PublicDomain,
    Row::OnView,
    Row::Department,
    Row::Type,
    Row::From,
    Row::To,
];

impl Row {
    fn name(self) -> &'static str {
        match self {
            Row::Image => "Image",
            Row::PublicDomain => "Public domain",
            Row::OnView => "On view",
            Row::Department => "Department",
            Row::Type => "Type",
            Row::From => "From year",
            Row::To => "To year",
        }
    }
}

/// The filter overlay that `f` opens (ADR 0013).
///
/// It edits a copy of the search's filters: `h`/`l` (or the arrows) change the highlighted row,
/// the year rows take typed digits, `enter` applies the copy and `esc` drops it.
#[derive(Default)]
pub struct FilterPanel {
    draft: Filters,
    row: usize,
    from: String,
    to: String,
    facets: Option<Facets>,
    /// Why the departments and types could not be listed.
    facets_error: Option<String>,
    /// Why the years did not parse, until the next edit.
    error: Option<String>,
    /// Where the last draw put the overlay, so a click outside it can close it.
    pub area: Rect,
}

/// The next of `options` after `current` by `delta`, wrapping around.
fn cycle<T: PartialEq + Clone>(options: &[T], current: &T, delta: isize) -> T {
    let at = options.iter().position(|o| o == current).unwrap_or(0) as isize;
    let len = options.len() as isize;
    options[(at + delta).rem_euclid(len) as usize].clone()
}

fn year_text(year: Option<i32>) -> String {
    year.map(|y| y.to_string()).unwrap_or_default()
}

impl FilterPanel {
    /// Starts editing a copy of `filters`, on the first row.
    pub fn open(&mut self, filters: &Filters) {
        self.draft = filters.clone();
        self.row = 0;
        self.from = year_text(filters.years.map(|(from, _)| from));
        self.to = year_text(filters.years.map(|(_, to)| to));
        self.error = None;
    }

    /// Whether the departments and types are listed yet.
    pub fn has_facets(&self) -> bool {
        self.facets.is_some()
    }

    pub fn set_facets(&mut self, facets: Result<Facets, Failure>) {
        match facets {
            Ok(mut facets) => {
                facets.departments.sort_by(|a, b| a.1.cmp(&b.1));
                facets.types.sort_by(|a, b| a.1.cmp(&b.1));
                self.facets = Some(facets);
                self.facets_error = None;
            }
            Err(failure) => self.facets_error = Some(failure::line(&failure)),
        }
    }

    fn current(&self) -> Row {
        ROWS[self.row]
    }

    /// Moves the highlight by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) {
        self.row = self.row.saturating_add_signed(delta).min(ROWS.len() - 1);
    }

    /// Whether the highlighted row takes typed text.
    pub fn is_typing(&self) -> bool {
        matches!(self.current(), Row::From | Row::To)
    }

    fn year_mut(&mut self) -> Option<&mut String> {
        match self.current() {
            Row::From => Some(&mut self.from),
            Row::To => Some(&mut self.to),
            _ => None,
        }
    }

    /// Types `c` into a year row, if it can be part of a year.
    pub fn push(&mut self, c: char) {
        self.error = None;
        let Some(year) = self.year_mut() else { return };
        let fits = c.is_ascii_digit() || (c == '-' && year.is_empty());
        if fits && year.len() < 6 {
            year.push(c);
        }
    }

    pub fn backspace(&mut self) {
        self.error = None;
        if let Some(year) = self.year_mut() {
            year.pop();
        }
    }

    /// Changes the highlighted row's value by `delta` steps.
    pub fn change(&mut self, delta: isize) {
        self.error = None;
        let row = self.current();
        let draft = &mut self.draft;
        match row {
            Row::Image => draft.has_image = !draft.has_image,
            Row::PublicDomain => {
                draft.is_public_domain = cycle(
                    &[None, Some(true), Some(false)],
                    &draft.is_public_domain,
                    delta,
                )
            }
            Row::OnView => {
                draft.is_on_view = cycle(&[None, Some(true), Some(false)], &draft.is_on_view, delta)
            }
            Row::Department => {
                let Some(facets) = &self.facets else { return };
                let options: Vec<Option<String>> = std::iter::once(None)
                    .chain(facets.departments.iter().map(|(id, _)| Some(id.clone())))
                    .collect();
                draft.department_id = cycle(&options, &draft.department_id, delta);
            }
            Row::Type => {
                let Some(facets) = &self.facets else { return };
                let options: Vec<Option<u32>> = std::iter::once(None)
                    .chain(facets.types.iter().map(|(id, _)| Some(*id)))
                    .collect();
                draft.artwork_type_id = cycle(&options, &draft.artwork_type_id, delta);
            }
            Row::From | Row::To => {}
        }
    }

    /// Puts every filter back to the defaults, without applying them yet.
    pub fn reset(&mut self, defaults: &Filters) {
        let row = self.row;
        self.open(defaults);
        self.row = row;
    }

    /// The filters as edited, or why the years don't make a range.
    pub fn apply(&mut self) -> Result<Filters, String> {
        let parse = |text: &str, name: &str| -> Result<Option<i32>, String> {
            match text {
                "" => Ok(None),
                text => text
                    .parse()
                    .map(Some)
                    .map_err(|_| format!("{name} must be a year, such as 1890 or -500")),
            }
        };
        let years = match (parse(&self.from, "from")?, parse(&self.to, "to")?) {
            (None, None) => None,
            (from, to) => {
                let from = from.unwrap_or(DATED_YEARS.0);
                let to = to.unwrap_or(DATED_YEARS.1);
                if from > to {
                    self.error = Some("the from year is after the to year".into());
                    return Err(self.error.clone().unwrap_or_default());
                }
                Some((from, to))
            }
        };
        let mut filters = self.draft.clone();
        filters.years = years;
        Ok(filters)
    }

    /// Shows why the filters were not applied.
    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
    }

    fn value(&self, row: Row) -> String {
        let tri = |value: Option<bool>| match value {
            None => "any",
            Some(true) => "yes",
            Some(false) => "no",
        };
        let any = || "any".to_string();
        match row {
            Row::Image => if self.draft.has_image {
                "required"
            } else {
                "any"
            }
            .into(),
            Row::PublicDomain => tri(self.draft.is_public_domain).into(),
            Row::OnView => tri(self.draft.is_on_view).into(),
            Row::Department => match (&self.draft.department_id, &self.facets) {
                (None, _) => any(),
                (Some(id), Some(facets)) => facets
                    .departments
                    .iter()
                    .find(|(d, _)| d == id)
                    .map_or_else(|| id.clone(), |(_, title)| title.clone()),
                (Some(id), None) => id.clone(),
            },
            Row::Type => match (self.draft.artwork_type_id, &self.facets) {
                (None, _) => any(),
                (Some(id), Some(facets)) => facets
                    .types
                    .iter()
                    .find(|(t, _)| *t == id)
                    .map_or_else(|| id.to_string(), |(_, title)| title.clone()),
                (Some(id), None) => id.to_string(),
            },
            Row::From => self.from.clone(),
            Row::To => self.to.clone(),
        }
    }
}

impl Component for FilterPanel {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let quiet = Style::new().fg(Color::DarkGray);
        let mut lines = Vec::new();
        for (i, row) in ROWS.into_iter().enumerate() {
            let selected = i == self.row;
            let mut value = self.value(row);
            let listed = matches!(row, Row::Department | Row::Type);
            if listed && self.facets.is_none() {
                value = if self.facets_error.is_some() {
                    "unavailable".into()
                } else {
                    "loading…".into()
                };
            }
            let value = match (selected, row) {
                (true, Row::From | Row::To) => format!("{value}▏"),
                (true, _) => format!("‹ {value} ›"),
                (false, Row::From | Row::To) if value.is_empty() => "any".into(),
                (false, _) => value,
            };
            let mut line = Line::from(vec![
                Span::raw(if selected { "> " } else { "  " }),
                Span::raw(format!("{:<15}", row.name())),
                Span::raw(value),
            ]);
            if selected {
                line = line.style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD));
            }
            lines.push(line);
        }
        lines.push(Line::default());
        match (&self.error, &self.facets_error) {
            (Some(error), _) => lines.push(Line::from(error.clone()).fg(Color::Red)),
            (None, Some(error)) => {
                lines.push(Line::from(format!("Lists unavailable: {error}")).style(quiet))
            }
            (None, None) => lines.push(
                Line::from("h/l change · enter apply · esc cancel · ctrl-u reset").style(quiet),
            ),
        }
        let height = u16::try_from(lines.len()).unwrap_or(u16::MAX) + 2;
        let overlay = centered(area, 62, height);
        self.area = overlay;
        frame.render_widget(Clear, overlay);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .border_style(Style::new().fg(ACCENT))
                    .title(" Filters "),
            ),
            overlay,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{catalog::default_filters, test_support::render};

    fn facets() -> Facets {
        Facets {
            departments: vec![
                ("PC-7".into(), "Arts of Asia".into()),
                ("PC-10".into(), "Painting and Sculpture of Europe".into()),
            ],
            types: vec![(18, "Print".into()), (1, "Painting".into())],
        }
    }

    fn panel() -> FilterPanel {
        let mut panel = FilterPanel::default();
        panel.open(&default_filters());
        panel.set_facets(Ok(facets()));
        panel
    }

    #[test]
    fn renders_the_defaults_with_the_first_row_highlighted() {
        let mut panel = panel();
        insta::assert_snapshot!(render(&mut panel, 70, 12));
    }

    #[test]
    fn rows_cycle_through_their_values() {
        let mut panel = panel();
        panel.change(1);
        assert!(!panel.draft.has_image);
        panel.step(1);
        panel.change(1);
        assert_eq!(panel.draft.is_public_domain, Some(true));
        panel.change(1);
        assert_eq!(panel.draft.is_public_domain, Some(false));
        panel.change(1);
        assert_eq!(panel.draft.is_public_domain, None, "wraps to any");
        panel.step(2);
        panel.change(1);
        assert_eq!(
            panel.draft.department_id.as_deref(),
            Some("PC-7"),
            "sorted by title"
        );
        panel.change(-2);
        assert_eq!(
            panel.draft.department_id.as_deref(),
            Some("PC-10"),
            "wraps backward past any"
        );
        panel.step(1);
        panel.change(1);
        assert_eq!(
            panel.draft.artwork_type_id,
            Some(1),
            "Painting before Print"
        );
    }

    #[test]
    fn years_are_typed_and_checked() {
        let mut panel = panel();
        panel.step(5);
        assert!(panel.is_typing());
        for c in "18x90".chars() {
            panel.push(c);
        }
        assert_eq!(panel.from, "1890", "only digits");
        let filters = panel.apply().unwrap();
        assert_eq!(filters.years, Some((1890, DATED_YEARS.1)));
        panel.step(1);
        for c in "-500".chars() {
            panel.push(c);
        }
        assert!(panel.apply().is_err(), "from after to");
        insta::assert_snapshot!(render(&mut panel, 70, 12));
    }

    #[test]
    fn reset_goes_back_to_the_defaults_and_open_drops_edits() {
        let mut panel = panel();
        panel.change(1);
        panel.reset(&default_filters());
        assert_eq!(panel.apply().unwrap(), default_filters());
        panel.change(1);
        panel.open(&default_filters());
        assert_eq!(panel.apply().unwrap(), default_filters());
    }

    #[test]
    fn lists_say_when_they_are_loading_or_unavailable() {
        let mut panel = FilterPanel::default();
        panel.open(&default_filters());
        assert!(render(&mut panel, 70, 12).contains("loading…"));
        panel.set_facets(Err(crate::components::test_support::offline()));
        let drawn = render(&mut panel, 70, 12);
        assert!(drawn.contains("unavailable") && drawn.contains("offline"));
    }
}
