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
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

use super::Component;

use crate::{action::Action, config::get_data_dir};

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

    fn search(&mut self) -> Result<()> {
        info!("Running search");
        let q = self.q.clone();
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            info!("Search started");
            aic_artworks_search(q).await.unwrap();
            info!("Search completed");
            tx.send(Action::ExitSearch).unwrap();
        });
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        let mut results_path = get_data_dir().clone();
        results_path.push("results.json");

        info!("Reading results from disk");
        let body = fs::read_to_string(results_path).unwrap();

        let json: SearchResults = serde_json::from_str(body.as_str()).unwrap();

        let json_data = json
            .data
            .iter()
            .map(|datum| Artwork {
                id: datum.id,
                title: datum.title.clone(),
                image_id: datum.image_id.clone(),
            })
            .collect::<Vec<Artwork>>();

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
        info!("register_action_handler");
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, area: Size) -> Result<()> {
        info!("init");
        let _ = area;
        let tx = self.action_tx.clone().unwrap();
        info!("tx: {:?}", tx);
        tx.send(Action::EnterSearch)?;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if action == Action::ToggleArtworksList {
            self.toggle()?
        }

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

pub struct Artwork {
    id: usize,
    title: String,
    image_id: String,
}

#[derive(Deserialize)]
struct Datum {
    _score: f32,
    id: usize,
    image_id: String,
    title: String,
}

#[derive(Deserialize)]
struct SearchResults {
    data: Vec<Datum>,
}

pub async fn aic_artworks_search(q: String) -> Result<()> {
    info!("aic_artworks_search(q={})", q);

    let mut results_path = get_data_dir().clone();
    results_path.push("results.json");

    if results_path.exists() & false {
        info!("Already have results on disk");
    } else {
        let client = Client::new();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("user-agent", "AIC-TUI/1.0".parse().unwrap());

        let mut url = "https://api.artic.edu/api/v1/artworks/search".to_string();
        url = format!("{}?q={}", url, q);
        url = format!(
            "{}&query[term][is_public_domain]=true&fields=id,title,image_id",
            url
        );

        let response = client
            .get(url)
            .headers(headers)
            .send()
            .await
            .expect("search failed");
        let body = response.text().await.expect("failed to get text");
        info!("Writing search results to {:?}", results_path);
        fs::write(results_path, body.as_bytes()).expect("failed to write");
    };

    Ok(())
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
