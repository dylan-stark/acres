use acres::artworks::{Artwork, ArtworkSummary};
use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use super::{ACCENT, Component, centered, pane_block};
use crate::entry::Entry;

/// The full record of the artwork shown, once asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Full {
    Loading,
    Loaded(Box<Artwork>),
    Failed(String),
}

/// The record pane: a wall label for the selected artwork, scrolled with `j`/`k`.
///
/// The label reads like the ones beside works in the museum's galleries: artist, title and
/// date, medium, then the credit line and reference number. It comes from the results list, so
/// it costs no request. `d` expands it in place to the full record, which is fetched then, and
/// the same component draws the full-record overlay that `enter` opens (ADR 0013).
#[derive(Default)]
pub struct Record {
    artwork: Option<Entry>,
    full: Option<Full>,
    expanded: bool,
    /// Whether this draws as the large full-record overlay instead of the pane.
    overlay: bool,
    scroll: u16,
    /// Lines the record took and rows the pane showed in the last draw, to bound scrolling.
    lines: u16,
    rows: u16,
    /// Where the last draw put it, so a click outside the overlay can close it.
    pub area: Rect,
    pub focused: bool,
}

impl Record {
    /// The full-record overlay: always expanded.
    pub fn overlay() -> Self {
        Self {
            expanded: true,
            overlay: true,
            ..Self::default()
        }
    }

    /// Shows `artwork` from the top, collapsed to its label in the pane.
    ///
    /// The full record is kept when it is the same artwork.
    pub fn show(&mut self, artwork: Option<Entry>) {
        let same = artwork.as_ref().map(Entry::id) == self.artwork.as_ref().map(Entry::id);
        if !same {
            self.full = None;
            self.expanded = self.overlay;
        }
        self.artwork = artwork;
        self.scroll = 0;
    }

    /// The selected artwork's id when it is expanded and its full record still has to be
    /// fetched, marking it as loading.
    pub fn wants_full(&mut self) -> Option<u32> {
        if !self.expanded || matches!(self.full, Some(Full::Loading | Full::Loaded(_))) {
            return None;
        }
        let id = self.artwork.as_ref()?.id();
        self.full = Some(Full::Loading);
        Some(id)
    }

    /// Expands the label to the full record, or collapses it, and returns the id to fetch when
    /// the full record is needed.
    pub fn toggle_expanded(&mut self) -> Option<u32> {
        self.expanded = !self.expanded;
        self.scroll = 0;
        self.wants_full()
    }

    pub fn expanded(&self) -> bool {
        self.expanded
    }

    /// Takes in the full record for `id`, if it is still the one shown.
    pub fn loaded(&mut self, id: u32, result: &Result<Artwork, String>) {
        if self.artwork.as_ref().map(Entry::id) != Some(id) {
            return;
        }
        self.full = Some(match result {
            Ok(artwork) => Full::Loaded(Box::new(artwork.clone())),
            Err(error) => Full::Failed(error.clone()),
        });
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
        let quiet = Style::new().fg(Color::DarkGray);
        let Some(artwork) = &self.artwork else {
            return Text::from(Line::from("No artwork selected").style(quiet));
        };
        // The full record has the whole label even when the list, such as a saved file, did not.
        let summary = match &self.full {
            Some(Full::Loaded(full)) => &full.summary,
            _ => &artwork.summary,
        };
        let mut lines = label(summary);
        if self.expanded {
            lines.push(Line::default());
            lines.push(Line::from("───").style(quiet));
            lines.push(Line::default());
            match &self.full {
                None | Some(Full::Loading) => {
                    lines.push(Line::from("Loading the full record…").style(quiet));
                }
                Some(Full::Failed(error)) => {
                    lines.push(Line::from(format!(
                        "Could not load the full record: {error}"
                    )));
                }
                Some(Full::Loaded(full)) => lines.extend(details(full)),
            }
        }
        Text::from(lines)
    }
}

/// The wall label: artist, title and date, medium, then credit line and reference number.
fn label(summary: &ArtworkSummary) -> Vec<Line<'static>> {
    let quiet = Style::new().fg(Color::DarkGray);
    let mut lines = Vec::new();
    if let Some(artist) = &summary.artist_display {
        let mut artist = artist.lines();
        if let Some(name) = artist.next() {
            lines.push(Line::from(name.to_string()).bold());
        }
        lines.extend(artist.map(|line| Line::from(line.to_string())));
        lines.push(Line::default());
    }
    lines.push(Line::from(summary.title.clone()).bold().italic());
    if let Some(date) = &summary.date_display {
        lines.push(Line::from(date.clone()));
    }
    if let Some(medium) = &summary.medium_display {
        lines.push(Line::default());
        lines.push(Line::from(medium.clone()));
    }
    let credit = [&summary.credit_line, &summary.main_reference_number];
    if credit.iter().any(|field| field.is_some()) {
        lines.push(Line::default());
        lines.extend(
            credit
                .into_iter()
                .flatten()
                .map(|field| Line::from(field.clone()).style(quiet)),
        );
    }
    if summary.medium_display.is_none() && summary.credit_line.is_none() {
        // A saved file carries only the title, so show what identifies the record.
        lines.push(Line::default());
        lines.push(Line::from(format!("id     {}", summary.id)).style(quiet));
        if let Some(image_id) = &summary.image_id {
            lines.push(Line::from(format!("image  {image_id}")).style(quiet));
        }
    }
    lines
}

/// The rest of the full record, below the label.
fn details(full: &Artwork) -> Vec<Line<'static>> {
    let quiet = Style::new().fg(Color::DarkGray);
    let on_view = if full.is_on_view {
        Some(
            full.gallery_title
                .clone()
                .unwrap_or_else(|| "On view".into()),
        )
    } else {
        Some("Not on view".into())
    };
    let fields = [
        ("Dimensions", full.dimensions.clone()),
        ("Made in", full.place_of_origin.clone()),
        ("Department", full.department_title.clone()),
        ("On view", on_view),
    ];
    let mut lines = Vec::new();
    for (name, value) in fields {
        let Some(value) = value else { continue };
        lines.push(Line::from(name).style(quiet));
        lines.push(Line::from(value));
    }
    if let Some(description) = &full.description {
        for paragraph in strip_html(description) {
            lines.push(Line::default());
            lines.push(Line::from(paragraph));
        }
        lines.push(Line::default());
        lines.push(Line::from("Description: CC BY 4.0, Art Institute of Chicago").style(quiet));
    }
    lines
}

/// The paragraphs of an HTML description, with tags dropped and common entities decoded.
fn strip_html(html: &str) -> Vec<String> {
    let mut text = String::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = "";
            break;
        };
        let tag = rest[open + 1..open + close]
            .trim_start_matches('/')
            .to_lowercase();
        if tag.starts_with("p") && (tag.len() == 1 || tag.as_bytes()[1] == b' ')
            || tag.starts_with("br")
        {
            text.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    text.push_str(rest);
    let text = text
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&rsquo;", "\u{2019}")
        .replace("&lsquo;", "\u{2018}")
        .replace("&ldquo;", "\u{201c}")
        .replace("&rdquo;", "\u{201d}")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    text.split('\n')
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect()
}

impl Component for Record {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let (block, area) = if self.overlay {
            let width = area.width.saturating_sub(8).min(90);
            let height = area.height.saturating_sub(4);
            let area = centered(area, width, height);
            frame.render_widget(Clear, area);
            let block = Block::bordered()
                .border_style(Style::new().fg(ACCENT))
                .title(" Full record ");
            (block, area)
        } else {
            (pane_block(4, "Record", self.focused), area)
        };
        self.area = area;
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

    fn jatte() -> Entry {
        Entry {
            summary: ArtworkSummary {
                id: 27992,
                title: "A Sunday on La Grande Jatte — 1884".into(),
                artist_display: Some("Georges Seurat\nFrench, 1859–1891".into()),
                date_display: Some("1884/86".into()),
                image_id: Some("2d484387".into()),
                medium_display: Some("Oil on canvas".into()),
                credit_line: Some("Helen Birch Bartlett Memorial Collection".into()),
                main_reference_number: Some("1926.224".into()),
                ..Default::default()
            },
            config: Default::default(),
        }
    }

    fn full_record() -> Artwork {
        Artwork {
            summary: jatte().summary,
            dimensions: Some("207.5 × 308.1 cm (81 3/4 × 121 1/4 in.)".into()),
            place_of_origin: Some("France".into()),
            department_title: Some("Painting and Sculpture of Europe".into()),
            is_on_view: true,
            gallery_title: Some("Gallery 240".into()),
            description: Some(
                "<p>In <em>Ferris Bueller&rsquo;s Day Off</em>, Cameron studies it.</p>\n<p>Up close, it dissolves into dots.</p>".into(),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn renders_what_a_saved_file_has() {
        let mut record = record();
        insta::assert_snapshot!(render(&mut record, 30, 8));
    }

    #[test]
    fn renders_a_wall_label() {
        let mut record = Record::default();
        record.show(Some(jatte()));
        insta::assert_snapshot!(render(&mut record, 44, 14));
    }

    #[test]
    fn expanding_asks_for_the_full_record_once_and_then_shows_it() {
        let mut record = Record::default();
        record.show(Some(jatte()));
        assert_eq!(record.toggle_expanded(), Some(27992));
        assert!(render(&mut record, 44, 20).contains("Loading the full record"));
        record.loaded(1, &Ok(Artwork::default()));
        assert!(
            render(&mut record, 44, 20).contains("Loading"),
            "a record for another artwork is ignored"
        );
        record.loaded(27992, &Ok(full_record()));
        assert_eq!(record.toggle_expanded(), None, "collapsing fetches nothing");
        assert_eq!(record.toggle_expanded(), None, "already loaded");
        insta::assert_snapshot!(render(&mut record, 44, 34));
    }

    #[test]
    fn moving_to_another_artwork_collapses_the_pane() {
        let mut record = Record::default();
        record.show(Some(jatte()));
        record.toggle_expanded();
        record.show(Some(Entry::default()));
        assert!(!record.expanded());
        assert_eq!(record.wants_full(), None);
    }

    #[test]
    fn the_overlay_is_always_expanded_and_says_why_a_fetch_failed() {
        let mut overlay = Record::overlay();
        overlay.show(Some(jatte()));
        assert_eq!(overlay.wants_full(), Some(27992));
        overlay.loaded(27992, &Err("offline".into()));
        insta::assert_snapshot!(render(&mut overlay, 60, 22));
    }

    #[test]
    fn strip_html_keeps_paragraphs_and_decodes_entities() {
        assert_eq!(
            strip_html("<p>A <em>B</em> &amp; C&rsquo;s</p>\n<p>D<br>E</p>"),
            ["A B & C\u{2019}s", "D", "E"]
        );
        assert_eq!(
            strip_html("<pre>x</pre>"),
            ["x"],
            "only p and br break lines"
        );
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
