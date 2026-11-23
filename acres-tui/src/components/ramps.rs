use std::fmt::{self, Display};

use color_eyre::eyre::Result;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{
        Color, Modifier, Style,
        palette::tailwind::{GRAY, GREEN, SLATE},
    },
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::{action::Action, components::Component};

const HORIZONTAL_PADDING: u16 = 3;
const VERTICAL_PADDING: u16 = 2;

/// The ramp preset the TUI starts with.
pub const DEFAULT_RAMP: &str = "alphabet";

/// The name of one of `charaoke`'s ramp presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RampName(&'static str);

impl Default for RampName {
    fn default() -> Self {
        Self(DEFAULT_RAMP)
    }
}

impl Display for RampName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Ramps component

/// The list of ramp presets, opened with `a`.
pub struct Ramps {
    list: BrowserList<RampName>,
    browsing: bool,
}

impl Ramps {
    pub fn new() -> Self {
        let names: Vec<RampName> = charaoke::Ramp::PRESETS
            .iter()
            .map(|&(name, _)| RampName(name))
            .collect();
        Self {
            list: BrowserList::new(&names),
            browsing: false,
        }
    }

    /// Chooses the highlighted ramp.
    fn choose(&mut self) -> Option<Action> {
        let i = self.list.state.selected()?;
        let name = self.list.find_item(i).value;
        Some(Action::SetRamp(name.to_string()))
    }
}

impl Component for Ramps {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if !self.browsing {
            if action == Action::EnterBrowseRampsMode {
                self.browsing = true;
            }
            return Ok(None);
        }
        Ok(match action {
            Action::MoveDown => self.list.move_down(),
            Action::MoveUp => self.list.move_up(),
            Action::Select => self.choose(),
            Action::EnterViewMode => {
                self.browsing = false;
                None
            }
            _ => None,
        })
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.browsing {
            self.list.draw(frame, area)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Status {
    Selected,
    Unselected,
}

////////////////////////////////////////////////////////////////////////////////
// Browser list (widget?)

#[derive(Default)]
struct BrowserList<T>
where
    T: Clone + Default + Display + PartialEq + Eq,
{
    items: Vec<BrowserItem<T>>,
    state: ListState,
}

impl<T> BrowserList<T>
where
    T: Clone + Default + Display + PartialEq + Eq,
{
    fn new(collection: &[T]) -> Self
    where
        T: Clone + Default + Display + PartialEq + Eq,
    {
        let (i, _) = collection
            .iter()
            .enumerate()
            .find(|(_, a)| **a == T::default())
            .expect("default is in the list");

        // TODO: Construct list of variants
        let list_iter: Vec<(Status, T)> = collection
            .iter()
            .enumerate()
            .map(|(j, item)| {
                let status = if j == i {
                    Status::Selected
                } else {
                    Status::Unselected
                };
                (status, item.clone())
            })
            .collect();
        let mut list = Self::from_iter(list_iter);
        list.state.select(Some(i));
        list
    }

    /// Moves down to the next item or stays at the bottom.
    fn move_down(&mut self) -> Option<Action> {
        self.state.select_next();
        None
    }

    /// Moves up to the previous item or stays at the top.
    fn move_up(&mut self) -> Option<Action> {
        self.state.select_previous();
        None
    }

    fn find_item(&mut self, i: usize) -> &BrowserItem<T> {
        for item in self.items.iter_mut() {
            item.status = Status::Unselected;
        }
        let item = self
            .items
            .iter_mut()
            .enumerate()
            .find(|(j, _)| i == *j)
            .expect("item at index i")
            .1;
        item.status = Status::Selected;
        item
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let width: u16 = area.width.min(
            self.items
                .iter()
                .map(|item| item.value.to_string().len() as u16 + HORIZONTAL_PADDING)
                .max()
                .expect("all items have len"),
        );
        let height = area.height.min(self.items.len() as u16);

        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Max(width),
            Constraint::Fill(1),
        ])
        .areas(area);
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Max(height + VERTICAL_PADDING),
            Constraint::Fill(1),
        ])
        .areas(middle);

        let block = Block::bordered().border_style(Style::new().fg(Color::DarkGray));

        let selected_style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

        let items: Vec<ListItem> = self.items.iter().map(ListItem::from).collect();
        let list: List<'_> = List::new(items)
            .block(block)
            .highlight_style(selected_style)
            .highlight_symbol("-")
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

        frame.render_widget(Clear, middle);
        frame.render_stateful_widget(list, middle, &mut self.state);
        Ok(())
    }
}

impl<T> FromIterator<(Status, T)> for BrowserList<T>
where
    T: Clone + Default + Display + Eq + PartialEq,
{
    fn from_iter<I: IntoIterator<Item = (Status, T)>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, item)| BrowserItem::new(status, item))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

#[derive(Clone)]
struct BrowserItem<T: Display> {
    value: T,
    status: Status,
}

impl<T: Display> BrowserItem<T> {
    fn new(status: Status, value: T) -> Self {
        Self { value, status }
    }
}

impl<T: Display> From<&BrowserItem<T>> for ListItem<'_> {
    fn from(value: &BrowserItem<T>) -> Self {
        let line = match value.status {
            Status::Selected => Line::styled(value.value.to_string(), GREEN.c500),
            Status::Unselected => Line::styled(value.value.to_string(), GRAY.c500),
        };
        ListItem::new(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn draws_nothing_in_view_mode() {
        let mut ramps = Ramps::new();
        assert!(render(&mut ramps, 40, 14).trim().is_empty());
    }

    #[test]
    fn renders_ramps_browser() {
        let mut ramps = Ramps::new();
        ramps.update(Action::EnterBrowseRampsMode).unwrap();
        insta::assert_snapshot!(render(&mut ramps, 40, 14));
    }

    #[test]
    fn select_sets_the_highlighted_ramp() {
        let mut ramps = Ramps::new();
        ramps.update(Action::EnterBrowseRampsMode).unwrap();
        ramps.update(Action::MoveDown).unwrap();
        assert_eq!(
            ramps.update(Action::Select).unwrap(),
            Some(Action::SetRamp(charaoke::Ramp::PRESETS[1].0.into()))
        );
    }

    #[test]
    fn escape_returns_to_view_mode() {
        let mut ramps = Ramps::new();
        ramps.update(Action::EnterBrowseRampsMode).unwrap();
        ramps.update(Action::EnterViewMode).unwrap();
        assert!(render(&mut ramps, 40, 14).trim().is_empty());
        assert_eq!(ramps.update(Action::Select).unwrap(), None);
    }
}
