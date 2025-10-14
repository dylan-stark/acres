use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct Home {
    action_tx: Option<UnboundedSender<Action>>,
    text: Text<'static>,
}

impl Home {
    pub fn new() -> Self {
        Self::default()
    }

    fn set_text(&mut self) -> Result<()> {
        self.text = Text::from("Hi!");
        Ok(())
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, _area: Size) -> Result<()> {
        self.text = Text::from("Hi!");
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        #[allow(clippy::single_match)]
        match action {
            Action::ToText => self.set_text()?,
            _ => {}
        }
        Ok(None)
    }

    /// Draws the ASCII image (`text`) centered in the viewing area.
    ///
    /// This is called either while Rendering or Resizing. In the case
    /// of Resizing, it is called after the TUI is resized but before
    /// the component receives the Resize action.
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Min(self.text.width() as u16),
            Constraint::Fill(1),
        ])
        .areas(area);
        let [_, middle, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Min(self.text.height() as u16),
            Constraint::Fill(1),
        ])
        .areas(middle);
        let widget = Paragraph::new(self.text.clone());
        frame.render_widget(widget, middle);
        Ok(())
    }
}
