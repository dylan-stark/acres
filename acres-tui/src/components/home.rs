use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct Home {
    action_tx: Option<UnboundedSender<Action>>,
    log: Vec<String>,
    messages_width: u16,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, _area: Size) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => return Ok(None),
            Action::Render => return Ok(None),
            _ => {
                let message = format!("action: {}", action);
                self.messages_width = self.messages_width.max(message.len() as u16);
                self.log.push(message);
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, right] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Max(self.messages_width)])
                .areas(area);

        let messages = self
            .log
            .iter()
            .rev()
            .take(right.height.into())
            .map(|m| m.to_string());
        let messages = List::new(messages)
            .direction(ListDirection::BottomToTop)
            .style(Style::new().light_green().on_dark_gray().italic());
        frame.render_widget(messages, right);
        Ok(())
    }
}
