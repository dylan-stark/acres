use std::fmt::Display;

use bytes::Bytes;
use color_eyre::eyre::Result;
use image_to_ascii_builder::{ALPHABETS, Alphabet, FONTS, Font};
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

use crate::{action::Action, components::Component};

const HORIZONTAL_PADDING: u16 = 3;
const VERTICAL_PADDING: u16 = 2;

enum Focus {
    Alphabets,
    Fonts,
}

////////////////////////////////////////////////////////////////////////////////
// Image-to-ASCII Builder component

pub struct ImageToAsciiBuilder {
    image: Option<Bytes>,
    alphabet: Option<Alphabet>,
    alphabet_list: BrowserList<Alphabet>,
    font: Option<Font>,
    font_list: BrowserList<Font>,
    focus: Option<Focus>,
    action_tx: UnboundedSender<Action>,
}

impl ImageToAsciiBuilder {
    /// Sets up new alphabets component.
    pub fn new(action_tx: UnboundedSender<Action>) -> Self {
        Self {
            image: None,
            alphabet: None,
            alphabet_list: BrowserList::new(ALPHABETS),
            font: None,
            font_list: BrowserList::new(FONTS),
            focus: None,
            action_tx,
        }
    }

    /// Enter browse alphabets mode
    fn enter_browse_alphabets_mode(&mut self) -> Option<Action> {
        self.focus = Some(Focus::Alphabets);
        None
    }

    /// Enter browse fonts mode
    fn enter_browse_fonts_mode(&mut self) -> Option<Action> {
        self.focus = Some(Focus::Fonts);
        None
    }

    /// Enter View mode
    fn enter_view_mode(&mut self) -> Option<Action> {
        self.focus = None;
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
        let continuation = match self.focus {
            Some(Focus::Alphabets) => match action {
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
            Some(Focus::Fonts) => match action {
                Action::MoveDown => self.font_list.move_down(),
                Action::MoveUp => self.font_list.move_up(),
                Action::Select => self.font_list.choose(),
                Action::EnterViewMode => self.enter_view_mode(),
                Action::ImageToAsciiBuilderUpdateFont(font) => self.update_font(font),
                Action::ImageToAsciiBuilderBuildAscii => self.build_ascii(),
                _ => None,
            },
            None => match action {
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
        match self.focus {
            Some(Focus::Alphabets) => self.alphabet_list.draw(frame, area),
            Some(Focus::Fonts) => self.font_list.draw(frame, area),
            None => Ok(()),
        }
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

impl BrowserList<Font> {
    /// Chooses current selection to show.
    fn choose(&mut self) -> Option<Action> {
        if let Some(i) = self.state.selected() {
            let item = self.find_item(i);
            Some(Action::ImageToAsciiBuilderUpdateFont(item.value.clone()))
        } else {
            None
        }
    }
}

impl BrowserList<Alphabet> {
    /// Chooses current selection to show.
    fn choose(&mut self) -> Option<Action> {
        if let Some(i) = self.state.selected() {
            let item = self.find_item(i);
            Some(Action::ImageToAsciiBuilderUpdateAlphabet(item.value.clone()))
        } else {
            None
        }
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
