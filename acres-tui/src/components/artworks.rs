use acres::artworks::ArtworkInfo;
use ratatui::{
    layout::{Constraint, Layout},
    style::{
        Color, Modifier, Style,
        palette::tailwind::{GRAY, GREEN, SLATE},
    },
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState},
};

use crate::{action::Action, app::Mode};

use super::Component;

const HORIZONTAL_PADDING: u16 = 3;
const VERTICAL_PADDING: u16 = 2;

#[derive(Default)]
pub struct Artworks {
    list: ArtworkList,
    mode: Mode,
}

/// Constructs Artworks list from JSON.
///
/// This is called when constructing the App.
impl Artworks {
    /// Sets up new artworks component.
    ///
    /// The primary job of this method is to construct the component's
    /// ArtworksList from an Acres Artworks (list).
    pub fn new(artworks: acres::artworks::Artworks, mode: Mode) -> Self {
        let artwork_infos: Vec<ArtworkInfo> = artworks.clone().into();
        // TODO: Figure out how to merge these two together now.
        let list_iter: Vec<(Status, u64, String, ArtworkInfo)> = artwork_infos
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let status = if i == 0 {
                    Status::Selected
                } else {
                    Status::Unselected
                };
                (
                    status,
                    info.data.id as u64,
                    info.data.title.clone(),
                    info.clone(),
                )
            })
            .collect();
        let mut list = ArtworkList::from_iter(list_iter);
        list.state.select_first();
        Self { list, mode }
    }

    /// Moves down to the next item or stays at the bottom.
    fn move_down(&mut self) -> Option<Action> {
        self.list.state.select_next();
        None
    }

    /// Moves up to the previous item or stays at the top.
    fn move_up(&mut self) -> Option<Action> {
        self.list.state.select_previous();
        None
    }

    /// Chooses current selection to show.
    fn choose(&mut self) -> Option<Action> {
        if let Some(i) = self.list.state.selected() {
            for item in self.list.items.iter_mut() {
                item.status = Status::Unselected;
            }
            let item = self
                .list
                .items
                .iter_mut()
                .enumerate()
                .find(|(j, _)| i == *j)
                .expect("item at index i")
                .1;
            item.status = Status::Selected;
            Some(Action::View(item.info.clone()))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ArtworkList {
    items: Vec<ArtworkItem>,
    state: ListState,
}

impl FromIterator<(Status, u64, String, ArtworkInfo)> for ArtworkList {
    fn from_iter<T: IntoIterator<Item = (Status, u64, String, ArtworkInfo)>>(iter: T) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, id, title, info)| ArtworkItem::new(status, id, title, info))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

#[derive(Clone)]
struct ArtworkItem {
    info: ArtworkInfo,
    label: String,
    status: Status,
}

impl ArtworkItem {
    fn new(status: Status, id: u64, title: String, info: ArtworkInfo) -> Self {
        let label = format!("{} ({})", title, id);
        Self {
            label,
            status,
            info,
        }
    }
}

impl From<&ArtworkItem> for ListItem<'_> {
    fn from(value: &ArtworkItem) -> Self {
        let line = match value.status {
            Status::Selected => Line::styled(value.label.clone(), GREEN.c500),
            Status::Unselected => Line::styled(value.label.clone(), GRAY.c500),
        };
        ListItem::new(line)
    }
}

#[derive(Clone)]
enum Status {
    Selected,
    Unselected,
}

impl Component for Artworks {
    fn init(&mut self, _area: ratatui::prelude::Size) -> color_eyre::eyre::Result<()> {
        Ok(())
    }

    fn update(
        &mut self,
        action: crate::action::Action,
    ) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
        let action = match self.mode {
            Mode::Browse => match action {
                Action::MoveDown => self.move_down(),
                Action::MoveUp => self.move_up(),
                Action::Select => self.choose(),
                _ => None,
            },
            Mode::View => {
                if action == Action::EnterBrowseMode {
                    self.mode = Mode::Browse;
                }
                None
            }
        };
        Ok(action)
    }

    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        if self.mode == Mode::View {
            return Ok(());
        }

        let width: u16 = area.width.min(
            self.list
                .items
                .iter()
                .map(|item| item.label.len() as u16 + HORIZONTAL_PADDING)
                .max()
                .expect("all items have len"),
        );
        let height = area.height.min(self.list.items.len() as u16);

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

        let items: Vec<ListItem> = self.list.items.iter().map(ListItem::from).collect();
        let list: List<'_> = List::new(items)
            .block(block)
            .highlight_style(selected_style)
            .highlight_symbol("-")
            .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

        frame.render_widget(Clear, middle);
        frame.render_stateful_widget(list, middle, &mut self.list.state);
        Ok(())
    }
}
