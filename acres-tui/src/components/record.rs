use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Paragraph, Wrap},
};

use super::{Component, pane_block};
use crate::entry::Entry;

/// The record pane: what is known about the selected artwork, scrolled with `j`/`k`.
///
/// It shows what the results list carries: title, artist and date (a saved `Artworks` file has
/// only the title). The wall label, built from the full record, replaces this in step 3 of
/// ADR 0013.
#[derive(Default)]
pub struct Record {
    artwork: Option<Entry>,
    scroll: u16,
    /// Lines the record took and rows the pane showed in the last draw, to bound scrolling.
    lines: u16,
    rows: u16,
    pub focused: bool,
}

impl Record {
    /// Shows `artwork` from the top.
    pub fn show(&mut self, artwork: Option<Entry>) {
        self.artwork = artwork;
        self.scroll = 0;
    }

    /// Scrolls by `delta` lines, stopping at either end.
    pub fn scroll(&mut self, delta: isize) {
        let max = self.lines.saturating_sub(self.rows);
        let to = usize::from(self.scroll).saturating_add_signed(delta);
        self.scroll = u16::try_from(to).unwrap_or(u16::MAX).min(max);
    }

    /// Lines in half the pane's height, at least one.
    pub fn half_page(&self) -> isize {
        isize::try_from((self.rows / 2).max(1)).unwrap_or(1)
    }

    fn text(&self) -> Text<'_> {
        let Some(artwork) = &self.artwork else {
            return Text::from(Line::from("No artwork selected").fg(Color::DarkGray));
        };
        let quiet = Style::new().fg(Color::DarkGray);
        let summary = &artwork.summary;
        let mut lines = vec![Line::from(summary.title.as_str()).bold()];
        if let Some(date) = &summary.date_display {
            lines.push(Line::from(date.as_str()));
        }
        if let Some(artist) = &summary.artist_display {
            lines.push(Line::default());
            lines.extend(artist.lines().map(|line| Line::from(line.to_string())));
        }
        lines.push(Line::default());
        lines.push(Line::from(format!("id     {}", summary.id)).style(quiet));
        if let Some(image_id) = &summary.image_id {
            lines.push(Line::from(format!("image  {image_id}")).style(quiet));
        }
        Text::from(lines)
    }
}

impl Component for Record {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = pane_block(4, "Record", self.focused);
        let inner = block.inner(area);
        let paragraph = Paragraph::new(self.text()).wrap(Wrap { trim: false });
        let lines = u16::try_from(paragraph.line_count(inner.width)).unwrap_or(u16::MAX);
        let scroll = self.scroll.min(lines.saturating_sub(inner.height));
        frame.render_widget(paragraph.scroll((scroll, 0)).block(block), area);
        (self.lines, self.rows, self.scroll) = (lines, inner.height, scroll);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::test_support::{ARTWORKS_FIXTURE, render};

    fn record() -> Record {
        let artworks: acres::artworks::Artworks =
            serde_json::from_str(ARTWORKS_FIXTURE).expect("fixture parses");
        let first = Entry::from_file(artworks).into_iter().next();
        let mut record = Record::default();
        record.show(first);
        record
    }

    #[test]
    fn renders_the_selected_artwork() {
        let mut record = record();
        insta::assert_snapshot!(render(&mut record, 30, 8));
    }

    #[test]
    fn renders_the_artist_and_date_when_listed() {
        let mut record = Record::default();
        record.show(Some(Entry {
            summary: acres::artworks::ArtworkSummary {
                id: 16568,
                title: "Water Lilies".into(),
                artist_display: Some("Claude Monet\nFrench, 1840–1926".into()),
                date_display: Some("1906".into()),
                image_id: Some("3c27b499".into()),
                ..Default::default()
            },
            iiif_url: String::new(),
        }));
        insta::assert_snapshot!(render(&mut record, 30, 10));
    }

    #[test]
    fn scrolls_only_as_far_as_the_text_goes() {
        let mut record = record();
        // Five rows inside the border, and the title wraps to two lines at this width.
        render(&mut record, 20, 7);
        record.scroll(100);
        let bottom = record.scroll;
        assert!(bottom > 0, "the text is taller than the pane");
        record.scroll(1);
        assert_eq!(record.scroll, bottom);
        record.scroll(-100);
        assert_eq!(record.scroll, 0);
    }
}
