use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize, palette::tailwind::SLATE},
    text::{Line, Span},
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState},
};

use super::{ACCENT, Component, centered};
use crate::saved::{SavedBase, SavedView};

/// The overlay `V` opens: the selected artwork's saved views, newest first (ADR 0017). `enter`
/// shows one, `e` renames it and `x` deletes it.
#[derive(Default)]
pub struct ViewList {
    /// Each view's name and a short description, such as `fill 3.2x · 20x12`.
    rows: Vec<(String, String)>,
    state: ListState,
    /// The artwork's title, for the border.
    title: String,
    /// The keys that show, rename and delete, as the keymap has them.
    pub keys: String,
    /// Where the last draw put the overlay, so a click outside it can close it.
    pub area: Rect,
}

impl ViewList {
    /// Lists `views` of the artwork titled `title`, keeping the highlight where it was if it can.
    pub fn open(&mut self, title: &str, views: &[SavedView]) {
        self.title = title.to_string();
        self.rows = views
            .iter()
            .map(|view| {
                let base = match view.base {
                    SavedBase::Fit => "fit",
                    SavedBase::Fill => "fill",
                };
                let zoom = if (view.zoom - 1.0).abs() < 0.05 {
                    base.to_string()
                } else {
                    format!("{base} {:.1}x", view.zoom)
                };
                let detail = match view.frame {
                    Some([cols, rows]) => format!("{zoom} · {cols}x{rows}"),
                    None => zoom,
                };
                (view.name.clone(), detail)
            })
            .collect();
        let at = self.state.selected().unwrap_or(0);
        self.state
            .select((!self.rows.is_empty()).then(|| at.min(self.rows.len() - 1)));
    }

    /// The highlighted view's place in the list, newest first.
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Moves the highlight by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let at = self.state.selected().unwrap_or(0);
        self.state.select(Some(
            at.saturating_add_signed(delta).min(self.rows.len() - 1),
        ));
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

impl Component for ViewList {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let width = area.width.saturating_sub(4).min(56);
        let height = u16::try_from(self.rows.len()).unwrap_or(u16::MAX).min(12) + 2;
        let overlay = centered(area, width, height);
        self.area = overlay;
        let name_width = self
            .rows
            .iter()
            .map(|(name, _)| name.chars().count())
            .max()
            .unwrap_or(0)
            .min(28);
        let items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|(name, detail)| {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("{name:<name_width$}  ")),
                    Span::styled(detail.clone(), Style::new().fg(Color::Gray)),
                ]))
            })
            .collect();
        let block = Block::bordered()
            .border_style(Style::new().fg(ACCENT))
            .title(format!(" Views · {} ", self.title))
            .title_bottom(
                Line::from(format!(" {} ", self.keys))
                    .fg(Color::Gray)
                    .right_aligned(),
            );
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD))
            .highlight_symbol("-")
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_widget(Clear, overlay);
        frame.render_stateful_widget(list, overlay, &mut self.state);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{
        art_view::{ArtView, FrameSize},
        test_support::render,
    };

    fn views() -> Vec<SavedView> {
        let fill = ArtView {
            zoom: 3.2,
            ..ArtView::fill()
        };
        vec![
            SavedView::new("Fuji", fill, Some(FrameSize::Custom { cols: 20, rows: 12 })),
            SavedView::new("whole", ArtView::default(), None),
        ]
    }

    #[test]
    fn renders_the_saved_views() {
        let mut list = ViewList {
            keys: "enter show · e rename · x delete".into(),
            ..Default::default()
        };
        list.open("The Great Wave", &views());
        insta::assert_snapshot!(render(&mut list, 60, 8));
    }

    #[test]
    fn the_highlight_stays_in_range_as_views_go() {
        let mut list = ViewList::default();
        list.open("Wave", &views());
        list.step(5);
        assert_eq!(list.selected(), Some(1));
        list.open("Wave", &views()[..1]);
        assert_eq!(list.selected(), Some(0));
        list.open("Wave", &[]);
        assert!(list.is_empty());
        assert_eq!(list.selected(), None);
    }
}
