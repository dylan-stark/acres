use ratatui::{
    layout::{Constraint, Layout},
    style::{
        Color, Modifier, Style,
        palette::tailwind::{GRAY, GREEN, SLATE},
    },
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState},
};

use super::Component;

#[derive(Default)]
pub struct Artworks {
    list: ArtworkList,
}

/// Constructs Artworks list from JSON.
///
/// This is called when constructing the App.
impl Artworks {
    pub fn new(artworks: acres::artworks::Artworks) -> Self {
        let list_iter: Vec<(Status, u64, String)> = artworks
            .data
            .iter()
            .map(|data| (Status::Unselected, data.id, data.title.clone()))
            .collect();
        let list = ArtworkList::from_iter(list_iter);
        Self { list }
    }
}

#[derive(Default)]
struct ArtworkList {
    items: Vec<ArtworkItem>,
    state: ListState,
}

impl FromIterator<(Status, u64, String)> for ArtworkList {
    fn from_iter<T: IntoIterator<Item = (Status, u64, String)>>(iter: T) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, id, title)| ArtworkItem::new(status, id, title))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

#[derive(Clone)]
struct ArtworkItem {
    label: String,
    status: Status,
}

impl ArtworkItem {
    fn new(status: Status, id: u64, title: String) -> Self {
        let label = format!("{} ({})", title, id);
        Self { label, status }
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

    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        let width: u16 = frame.area().width.min(
            self.list
                .items
                .iter()
                .map(|item| item.label.len() as u16)
                .max()
                .expect("all items have len"),
        );
        let height = frame.area().height.min(self.list.items.len() as u16);

        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Min(width),
            Constraint::Fill(1),
        ])
        .areas(area);
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Min(height),
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
