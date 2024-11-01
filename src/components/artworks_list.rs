use std::fs;

use color_eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Rect, Size},
    style::{
        palette::tailwind::{GRAY, GREEN, SLATE},
        Color, Modifier, Style,
    },
    text::Line,
    widgets::{Block, Clear, List, ListItem, ListState},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info};

use super::Component;

use crate::{
    action::Action,
    aic::{self, api::artworks::Client},
    config::get_data_dir,
    tui::Event,
};

#[derive(Debug, Clone)]
pub struct ArtworksList {
    artworks: Vec<ArtworkItem>,
    state: ListState,
    q: String,
    is_up: bool,
    action_tx: Option<UnboundedSender<Action>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArtworkItem {
    id: usize,
    title: String,
    image_id: String,
    status: ArtworkListStatus,
}

#[derive(Debug, Clone, PartialEq)]
enum ArtworkListStatus {
    Selected,
    Unselected,
}

impl ArtworksList {
    pub fn new(q: String) -> Self {
        info!("Creating new artworks list");
        Self {
            artworks: vec![],
            state: ListState::default(),
            q,
            is_up: true,
            action_tx: None,
        }
    }

    fn move_down(&mut self) -> Result<()> {
        self.state.select_next();
        Ok(())
    }

    fn move_up(&mut self) -> Result<()> {
        self.state.select_previous();
        Ok(())
    }

    fn select(&mut self) -> Result<()> {
        let tx = self.action_tx.clone().unwrap();
        if let Some(i) = self.state.selected() {
            for (j, item) in self.artworks.iter_mut().enumerate() {
                if i == j {
                    item.status = match item.status {
                        ArtworkListStatus::Selected => ArtworkListStatus::Unselected,
                        ArtworkListStatus::Unselected => ArtworkListStatus::Selected,
                    };
                    tx.send(Action::Display(item.image_id.clone()))?;
                } else {
                    item.status = match item.status {
                        ArtworkListStatus::Selected => ArtworkListStatus::Unselected,
                        ArtworkListStatus::Unselected => ArtworkListStatus::Unselected,
                    }
                }
            }
        }
        Ok(())
    }

    fn toggle(&mut self) -> Result<()> {
        self.is_up = !self.is_up;
        Ok(())
    }

    /// Starts the artworks search.
    ///
    /// We spawn off a task to handle the actual search so that it can block
    /// on the (possibly) long-latency op.
    ///
    /// When the task finishes it sends a continuation action to process
    /// the search results.
    fn search(&mut self) -> Result<()> {
        //fn start_search(&mut self) -> Result<()> {
        info!("Starting search");
        let q = self.q.clone();
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            info!("Search started");
            let mut results_path = get_data_dir().clone();
            results_path.push("results.json");

            if results_path.exists() & false {
                info!("Already have results on disk");
            } else {
                let pieces = Client::builder()
                    .build()
                    .search()
                    .with_text(q)
                    .start()
                    .await
                    .unwrap()
                    .result()
                    .await
                    .unwrap();

                info!("Writing search results to {:?}", results_path);
                fs::write(results_path, serde_json::to_vec(&pieces).unwrap())
                    .expect("failed to write");
            };
            info!("Search completed");
            tx.send(Action::ExitSearch).unwrap();
        });
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        let mut results_path = get_data_dir().clone();
        results_path.push("results.json");

        info!("Reading results from disk");
        let json = fs::read_to_string(results_path).unwrap();

        let json_data: Vec<aic::api::artworks::Artwork> =
            serde_json::from_str(json.as_str()).unwrap();

        self.artworks = json_data
            .into_iter()
            .map(|artwork| {
                ArtworkItem::new(
                    artwork.id,
                    artwork.title,
                    artwork.image_id,
                    ArtworkListStatus::Unselected,
                )
            })
            .collect();

        Ok(())
    }
}

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

impl Component for ArtworksList {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let action = match event {
            Some(Event::Init) => Some(Action::EnterSearch),
            Some(Event::Key(key_event)) => self.handle_key_event(key_event)?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_event(mouse_event)?,
            _ => None,
        };
        Ok(action)
    }

    fn init(&mut self, area: Size) -> Result<()> {
        info!("Initializing artworks list component");
        debug!("area: {:?}", area);
        let _ = area;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if !(action == Action::Render || action == Action::Tick) {
            info!("Updating artworks list: {:?}", action);
        }

        if action == Action::EnterBrowseMode || action == Action::EnterViewMode {
            self.toggle()?
        }

        // TODO: Consider if you can handle this better with modes
        if self.is_up {
            match action {
                Action::MoveDown => self.move_down()?,
                Action::MoveUp => self.move_up()?,
                Action::Select => self.select()?,
                Action::EnterSearch => self.search()?,
                Action::ExitSearch => self.load()?,
                _ => {}
            };
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.is_up {
            let [_, middle, _] =
                Layout::horizontal([Constraint::Min(8), Constraint::Max(64), Constraint::Min(8)])
                    .areas(area);
            let [_, middle, _] =
                Layout::vertical([Constraint::Min(8), Constraint::Max(8), Constraint::Min(8)])
                    .areas(middle);

            let block = Block::bordered().border_style(Style::new().fg(Color::DarkGray));

            let items: Vec<ListItem> = self.artworks.iter().map(ListItem::from).collect();

            let list: List<'_> = List::new(items)
                .block(block)
                .highlight_style(SELECTED_STYLE)
                .highlight_symbol("-")
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);

            frame.render_widget(Clear, middle);
            frame.render_stateful_widget(list, middle, &mut self.state);
        }
        Ok(())
    }
}

impl ArtworkItem {
    fn new(id: usize, title: String, image_id: String, status: ArtworkListStatus) -> Self {
        ArtworkItem {
            id,
            title,
            image_id,
            status,
        }
    }
}

const SELECTED_TEXT_FG_COLOR: Color = GREEN.c500;
const UNSELECTED_TEXT_FG_COLOR: Color = GRAY.c500;

impl From<&ArtworkItem> for ListItem<'_> {
    fn from(value: &ArtworkItem) -> Self {
        let line = match value.status {
            ArtworkListStatus::Selected => Line::styled(
                format!("{}: {}", value.id, value.title),
                SELECTED_TEXT_FG_COLOR,
            ),
            ArtworkListStatus::Unselected => Line::styled(
                format!("{}: {}", value.id, value.title),
                UNSELECTED_TEXT_FG_COLOR,
            ),
        };
        ListItem::new(line)
    }
}
