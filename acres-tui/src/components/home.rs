use ansi_to_tui::IntoText;
use color_eyre::{
    Result,
    eyre::{self, Context},
};
use ratatui::{prelude::*, text::Text, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct Home<'a> {
    text: Text<'a>,
    show_logs: bool,
    log: Vec<String>,
    messages_width: u16,
    action_tx: Option<UnboundedSender<Action>>,
}

impl<'a> Home<'a> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Component for Home<'a> {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, action: Action) -> eyre::Result<Option<Action>> {
        match action {
            Action::Tick => return Ok(None),
            Action::Render => return Ok(None),
            Action::UpdateAscii(ascii) => {
                let text = ascii
                    .to_string()
                    .as_bytes()
                    .into_text()
                    .context("failed to convert ascii bytes to text")?;
                self.text = text;
            }
            Action::ToggleLogs => self.show_logs = !self.show_logs,
            Action::Log(message) => {
                self.messages_width = self.messages_width.max(message.len() as u16);
                self.log.push(message);
            }
            _ => {
                let message = format!("action: {}", action);
                self.messages_width = self.messages_width.max(message.len() as u16);
                self.log.push(message);
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let logs_width = if self.show_logs {
            self.messages_width
        } else {
            0
        };
        let [view_area, logs_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Max(logs_width)]).areas(area);
        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Max(self.text.width() as u16),
            Constraint::Fill(1),
        ])
        .areas(view_area);
        let [_, ascii_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Max(self.text.height() as u16),
            Constraint::Fill(1),
        ])
        .areas(middle);

        let messages = self
            .log
            .iter()
            .rev()
            .take(logs_area.height.into())
            .map(|m| m.to_string());
        let messages = List::new(messages)
            .direction(ListDirection::BottomToTop)
            .style(Style::new().light_green().on_dark_gray().italic());
        frame.render_widget(messages, logs_area);

        let paragraph = Paragraph::new(self.text.clone());
        frame.render_widget(paragraph, ascii_area);

        Ok(())
    }
}
