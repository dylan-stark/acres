use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::{ACCENT, Component, centered};

/// The prompt `q` opens before quitting, so a stray `q` never ends the session.
#[derive(Default)]
pub struct QuitPrompt {
    /// The keys that quit and that stay, as the keymap has them, such as `q, enter` and `esc`.
    pub quit_keys: String,
    pub stay_keys: String,
    /// Where the last draw put the prompt, so a click outside it can close it.
    pub area: Rect,
}

impl Component for QuitPrompt {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let key = |keys: &str| Span::styled(keys.to_string(), Style::new().fg(ACCENT).bold());
        let quiet = |text: &'static str| Span::styled(text, Style::new().fg(Color::Gray));
        let keys = Line::from(vec![
            key(&self.quit_keys),
            quiet(" quit · "),
            key(&self.stay_keys),
            quiet(" stay"),
        ]);
        let width = u16::try_from(keys.width()).unwrap_or(u16::MAX).max(14) + 4;
        let prompt = centered(area, width, 5);
        self.area = prompt;
        let text = vec![Line::from("Quit acres?").bold(), Line::default(), keys];
        frame.render_widget(Clear, prompt);
        frame.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(Block::bordered().border_style(Style::new().fg(ACCENT))),
            prompt,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn renders_the_keys_it_is_given() {
        let mut prompt = QuitPrompt {
            quit_keys: "q, enter".into(),
            stay_keys: "esc".into(),
            ..Default::default()
        };
        insta::assert_snapshot!(render(&mut prompt, 34, 7));
    }
}
