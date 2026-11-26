use color_eyre::Result;
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};

use super::{ACCENT, Component, centered};

/// The one-line input `e` opens to edit a saved artwork's note, like the search bar. It also
/// names a saved view.
#[derive(Default)]
pub struct NoteEditor {
    /// What is being edited, for the border, or `None` for a note.
    pub label: Option<&'static str>,
    /// The saved artwork whose note this edits.
    pub id: Option<u32>,
    title: String,
    text: String,
    /// The keys that save and cancel, as the keymap has them, such as `enter` and `esc`.
    pub save_keys: String,
    pub cancel_keys: String,
    /// Where the last draw put it, so a click outside it can close it.
    pub area: Rect,
}

impl NoteEditor {
    /// An editor titled `label` instead of Note, such as `View`.
    pub fn labelled(label: &'static str) -> Self {
        Self {
            label: Some(label),
            ..Self::default()
        }
    }

    /// Starts editing the note on artwork `id`, titled `title`, from `note`.
    pub fn open(&mut self, id: u32, title: &str, note: Option<&str>) {
        self.id = Some(id);
        self.title = title.to_string();
        self.text = note.unwrap_or_default().to_string();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn push(&mut self, c: char) {
        self.text.push(c);
    }

    pub fn backspace(&mut self) {
        self.text.pop();
    }

    pub fn clear(&mut self) {
        self.text.clear();
    }
}

impl Component for NoteEditor {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let width = area.width.saturating_sub(4).min(70);
        let editor = centered(area, width, 3);
        self.area = editor;
        let key = |keys: &str| Span::styled(keys.to_string(), Style::new().fg(ACCENT).bold());
        let quiet = |text: &'static str| Span::styled(text, Style::new().fg(Color::Gray));
        let hint = Line::from(vec![
            key(&self.save_keys),
            quiet(" save · "),
            key(&self.cancel_keys),
            quiet(" cancel "),
        ]);
        let block = Block::bordered()
            .border_style(Style::new().fg(ACCENT))
            .title(format!(
                " {} · {} ",
                self.label.unwrap_or("Note"),
                self.title
            ))
            .title_bottom(hint.right_aligned());
        let inner = block.inner(editor);
        frame.render_widget(Clear, editor);
        frame.render_widget(Paragraph::new(self.text.as_str()).block(block), editor);
        let typed = u16::try_from(self.text.chars().count()).unwrap_or(u16::MAX);
        let x = inner
            .x
            .saturating_add(typed)
            .min(inner.right().saturating_sub(1));
        frame.set_cursor_position(Position::new(x, inner.y));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::render;

    #[test]
    fn renders_the_note_being_edited() {
        let mut editor = NoteEditor {
            save_keys: "enter".into(),
            cancel_keys: "esc".into(),
            ..Default::default()
        };
        editor.open(27992, "A Sunday on La Grande Jatte", Some("see it"));
        editor.push('!');
        insta::assert_snapshot!(render(&mut editor, 60, 5));
        editor.backspace();
        assert_eq!(editor.text(), "see it");
        editor.clear();
        assert_eq!(editor.text(), "");
    }
}
