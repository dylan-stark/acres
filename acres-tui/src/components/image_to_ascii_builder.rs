use bytes::Bytes;
use color_eyre::eyre::Result;
use image_to_ascii_builder::{Alphabet, Font, ALPHABETS, FONTS};
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
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, app::Mode, components::Component};

const HORIZONTAL_PADDING: u16 = 3;
const VERTICAL_PADDING: u16 = 2;

////////////////////////////////////////////////////////////////////////////////
// Image-to-ASCII Builder component

pub struct ImageToAsciiBuilder {
    image: Option<Bytes>,
    alphabet: Option<Alphabet>,
    alphabet_list: AlphabetList,
    font: Option<Font>,
    font_list: FontList,
    mode: Mode,
    action_tx: UnboundedSender<Action>,
}

impl ImageToAsciiBuilder {
    /// Sets up new alphabets component.
    pub fn new(action_tx: UnboundedSender<Action>, mode: Mode) -> Self {
        Self {
            image: None,
            alphabet: None,
            alphabet_list: AlphabetList::new(),
            font: None,
            font_list: FontList::new(),
            mode,
            action_tx,
        }
    }

    /// Enter browse alphabets mode
    fn enter_browse_alphabets_mode(&mut self) -> Option<Action> {
        self.mode = Mode::BrowseAlphabets;
        None
    }

    /// Enter browse fonts mode
    fn enter_browse_fonts_mode(&mut self) -> Option<Action> {
        self.mode = Mode::BrowseFonts;
        None
    }

    /// Enter View mode
    fn enter_view_mode(&mut self) -> Option<Action> {
        self.mode = Mode::View;
        None
    }

    /// Update image.
    fn update_image(&mut self, image: Bytes) -> Option<Action> {
        self.image = Some(image);
        Some(Action::ImageToAsciiBuilderBuildAscii)
    }

    /// Update alphabet.
    fn update_alphabet(&mut self, alphabet: Alphabet) -> Option<Action> {
        self.alphabet = Some(alphabet);
        Some(Action::ImageToAsciiBuilderBuildAscii)
    }

    /// Update font.
    fn update_font(&mut self, font: Font) -> Option<Action> {
        self.font = Some(font);
        Some(Action::ImageToAsciiBuilderBuildAscii)
    }

    /// Build ASCII
    fn build_ascii(&self) -> Option<Action> {
        if let Some(image) = self.image.clone() {
            let alphabet = self.alphabet.clone();
            let font = self.font.clone();
            let action_tx = self.action_tx.clone();
            tokio::spawn(async move {
                let _ = action_tx.send(Action::StartingRenderAscii);
                let ascii = image_to_ascii_builder::Ascii::builder()
                    .input(image)
                    .alphabet(alphabet)
                    .font(font)
                    .build()
                    .ok();
                if let Some(ascii) = ascii {
                    let _ = action_tx.send(Action::UpdateAscii(ascii));
                }
            });
        }
        None
    }
}

impl Component for ImageToAsciiBuilder {
    fn update(
        &mut self,
        action: crate::action::Action,
    ) -> color_eyre::eyre::Result<Option<crate::action::Action>> {
        let continuation = match self.mode {
            Mode::BrowseAlphabets => match action {
                Action::MoveDown => self.alphabet_list.move_down(),
                Action::MoveUp => self.alphabet_list.move_up(),
                Action::Select => self.alphabet_list.choose(),
                Action::EnterViewMode => self.enter_view_mode(),
                Action::ImageToAsciiBuilderUpdateAlphabet(alphabet) => {
                    self.update_alphabet(alphabet)
                }
                Action::ImageToAsciiBuilderBuildAscii => self.build_ascii(),
                _ => None,
            },
            Mode::BrowseFonts => match action {
                Action::MoveDown => self.font_list.move_down(),
                Action::MoveUp => self.font_list.move_up(),
                Action::Select => self.font_list.choose(),
                Action::EnterViewMode => self.enter_view_mode(),
                Action::ImageToAsciiBuilderUpdateFont(font) => {
                    self.update_font(font)
                }
                Action::ImageToAsciiBuilderBuildAscii => self.build_ascii(),
                _ => None,
            },
            _ => match action {
                Action::ImageToAsciiBuilderUpdateImage(image) => self.update_image(image),
                Action::ImageToAsciiBuilderBuildAscii => self.build_ascii(),
                Action::EnterBrowseAlphabetsMode => self.enter_browse_alphabets_mode(),
                Action::EnterBrowseFontsMode => self.enter_browse_fonts_mode(),
                _ => None,
            },
        };
        Ok(continuation)
    }

    fn draw(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::prelude::Rect,
    ) -> color_eyre::eyre::Result<()> {
        match self.mode {
            Mode::BrowseAlphabets => self.alphabet_list.draw(frame, area),
            Mode::BrowseFonts => self.font_list.draw(frame, area),
            _ => Ok(()),
        }
    }
}

#[derive(Clone)]
enum Status {
    Selected,
    Unselected,
}

////////////////////////////////////////////////////////////////////////////////
// Alphabet list (widget?)

#[derive(Default)]
struct AlphabetList {
    items: Vec<AlphabetItem>,
    state: ListState,
}

impl AlphabetList {
    fn new() -> Self {
        // TODO: Find default alphabet in list so we can set the status and selected state
        // appropriately.
        let (i, _) = ALPHABETS
            .iter()
            .enumerate()
            .find(|(_, a)| **a == image_to_ascii_builder::Alphabet::default())
            .expect("default is in the list");

        // TODO: Construct list of variants
        let list_iter: Vec<(Status, Alphabet)> = ALPHABETS
            .iter()
            .enumerate()
            .map(|(j, alphabet)| {
                let status = if j == i {
                    Status::Selected
                } else {
                    Status::Unselected
                };
                (status, alphabet.clone())
            })
            .collect();
        let mut list = AlphabetList::from_iter(list_iter);
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

    /// Chooses current selection to show.
    fn choose(&mut self) -> Option<Action> {
        if let Some(i) = self.state.selected() {
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
            Some(Action::ImageToAsciiBuilderUpdateAlphabet(
                item.alphabet.clone(),
            ))
        } else {
            None
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let width: u16 = area.width.min(
            self.items
                .iter()
                .map(|item| item.alphabet.to_string().len() as u16 + HORIZONTAL_PADDING)
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

impl FromIterator<(Status, Alphabet)> for AlphabetList {
    fn from_iter<T: IntoIterator<Item = (Status, Alphabet)>>(iter: T) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, alphabet)| AlphabetItem::new(status, alphabet))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

#[derive(Clone)]
struct AlphabetItem {
    alphabet: Alphabet,
    status: Status,
}

impl AlphabetItem {
    fn new(status: Status, alphabet: Alphabet) -> Self {
        Self { alphabet, status }
    }
}

impl From<&AlphabetItem> for ListItem<'_> {
    fn from(value: &AlphabetItem) -> Self {
        let line = match value.status {
            Status::Selected => Line::styled(value.alphabet.to_string(), GREEN.c500),
            Status::Unselected => Line::styled(value.alphabet.to_string(), GRAY.c500),
        };
        ListItem::new(line)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Font list (widget?)

#[derive(Default)]
struct FontList {
    items: Vec<FontItem>,
    state: ListState,
}

impl FontList {
    fn new() -> Self {
        let (i, _) = FONTS
            .iter()
            .enumerate()
            .find(|(_, a)| **a == image_to_ascii_builder::Font::default())
            .expect("default is in the list");

        // TODO: Construct list of variants
        let list_iter: Vec<(Status, Font)> = FONTS
            .iter()
            .enumerate()
            .map(|(j, font)| {
                let status = if j == i {
                    Status::Selected
                } else {
                    Status::Unselected
                };
                (status, font.clone())
            })
            .collect();
        let mut list = FontList::from_iter(list_iter);
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

    /// Chooses current selection to show.
    fn choose(&mut self) -> Option<Action> {
        if let Some(i) = self.state.selected() {
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
            Some(Action::ImageToAsciiBuilderUpdateFont(
                item.font.clone(),
            ))
        } else {
            None
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let width: u16 = area.width.min(
            self.items
                .iter()
                .map(|item| item.font.to_string().len() as u16 + HORIZONTAL_PADDING)
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

impl FromIterator<(Status, Font)> for FontList {
    fn from_iter<T: IntoIterator<Item = (Status, Font)>>(iter: T) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, font)| FontItem::new(status, font))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

#[derive(Clone)]
struct FontItem {
    font: Font,
    status: Status,
}

impl FontItem {
    fn new(status: Status, font: Font) -> Self {
        Self { font, status }
    }
}

impl From<&FontItem> for ListItem<'_> {
    fn from(value: &FontItem) -> Self {
        let line = match value.status {
            Status::Selected => Line::styled(value.font.to_string(), GREEN.c500),
            Status::Unselected => Line::styled(value.font.to_string(), GRAY.c500),
        };
        ListItem::new(line)
    }
}
