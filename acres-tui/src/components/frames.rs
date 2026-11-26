use color_eyre::Result;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize, palette::tailwind::SLATE},
    text::{Line, Span},
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState},
};

use super::{
    ACCENT, Component,
    art_view::{FrameSize, PRESETS},
    centered,
};
use crate::action::Action;

/// The rows of the picker: Pane, Screen, the presets, then a custom size.
const ROWS: usize = 2 + PRESETS.len() + 1;

/// The overlay `S` opens to pick the art's frame size (ADR 0017): the pane, the whole screen,
/// the Stamp and Postcard presets, or a custom size typed as `cols x rows`, at most the
/// terminal's.
#[derive(Default)]
pub struct Frames {
    state: ListState,
    /// The custom size as typed so far.
    custom: String,
    /// Why the custom size was refused.
    error: Option<String>,
    /// The frame in use, which shows in green.
    current: Option<FrameSize>,
    /// The terminal's size, the largest custom frame.
    max: (u16, u16),
    /// Where the last draw put the overlay, so a click outside it can close it.
    pub area: Rect,
}

impl Frames {
    /// Opens on the frame in use, `current`, for a terminal of `max` cells.
    pub fn open(&mut self, current: Option<FrameSize>, max: (u16, u16)) {
        self.current = current;
        self.max = max;
        self.error = None;
        let row = match current {
            None | Some(FrameSize::Pane) => 0,
            Some(FrameSize::Screen) => 1,
            Some(FrameSize::Custom { cols, rows }) => {
                match PRESETS.iter().position(|&(_, c, r)| (c, r) == (cols, rows)) {
                    Some(at) => 2 + at,
                    None => {
                        self.custom = format!("{cols}x{rows}");
                        ROWS - 1
                    }
                }
            }
        };
        self.state.select(Some(row));
    }

    /// Moves the highlight by `delta` rows, stopping at either end.
    pub fn step(&mut self, delta: isize) {
        let at = self.state.selected().unwrap_or(0);
        self.state
            .select(Some(at.saturating_add_signed(delta).min(ROWS - 1)));
        self.error = None;
    }

    /// Whether the custom row is highlighted, so digits and `x` type into it.
    pub fn is_typing(&self) -> bool {
        self.state.selected() == Some(ROWS - 1)
    }

    pub fn push(&mut self, c: char) {
        self.custom.push(c);
        self.error = None;
    }

    pub fn backspace(&mut self) {
        self.custom.pop();
        self.error = None;
    }

    /// The frame on the highlighted row, or why the custom size will not do.
    pub fn choose(&mut self) -> Option<Action> {
        let frame = match self.state.selected()? {
            0 => FrameSize::Pane,
            1 => FrameSize::Screen,
            at if at < ROWS - 1 => {
                let (_, cols, rows) = PRESETS[at - 2];
                FrameSize::Custom { cols, rows }
            }
            _ => match parse(&self.custom, self.max) {
                Ok((cols, rows)) => FrameSize::Custom { cols, rows },
                Err(error) => {
                    self.error = Some(error);
                    return None;
                }
            },
        };
        Some(Action::SetFrame(frame))
    }
}

/// Reads a size typed as `cols x rows`, such as `40x20` or `40 x 20`, at most `max`.
fn parse(text: &str, max: (u16, u16)) -> Result<(u16, u16), String> {
    let usage = || "type a size as cols x rows, such as 40x20".to_string();
    let lower = text.to_ascii_lowercase();
    let (cols, rows) = lower.split_once('x').ok_or_else(usage)?;
    let number = |s: &str| s.trim().parse::<u16>().ok().filter(|&n| n > 0);
    let (Some(cols), Some(rows)) = (number(cols), number(rows)) else {
        return Err(usage());
    };
    if cols > max.0 || rows > max.1 {
        return Err(format!("at most the terminal's {}x{}", max.0, max.1));
    }
    Ok((cols, rows))
}

impl Component for Frames {
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let overlay = centered(area, 34, ROWS as u16 + 2);
        self.area = overlay;
        let current_row = match self.current {
            None => None,
            Some(FrameSize::Pane) => Some(0),
            Some(FrameSize::Screen) => Some(1),
            Some(FrameSize::Custom { cols, rows }) => Some(
                PRESETS
                    .iter()
                    .position(|&(_, c, r)| (c, r) == (cols, rows))
                    .map_or(ROWS - 1, |at| 2 + at),
            ),
        };
        let mut labels: Vec<Line> = vec![
            Line::from(Span::raw("Pane")),
            Line::from(Span::raw("Screen")),
        ];
        for (name, cols, rows) in PRESETS {
            let mut title = name.to_string();
            title[..1].make_ascii_uppercase();
            labels.push(Line::from(vec![
                Span::raw(format!("{title:<9}")),
                Span::styled(format!("{cols} x {rows}"), Style::new().fg(Color::Gray)),
            ]));
        }
        let typed = if self.custom.is_empty() && !self.is_typing() {
            Span::styled("cols x rows", Style::new().fg(Color::DarkGray))
        } else {
            Span::raw(format!("{}_", self.custom))
        };
        labels.push(Line::from(vec![
            Span::raw(format!("{:<9}", "Custom")),
            typed,
        ]));
        let items: Vec<ListItem> = labels
            .into_iter()
            .enumerate()
            .map(|(i, line)| {
                let line = if Some(i) == current_row {
                    line.fg(Color::Green)
                } else {
                    line
                };
                ListItem::new(line)
            })
            .collect();
        let mut block = Block::bordered()
            .border_style(Style::new().fg(ACCENT))
            .title(" Frame size ");
        if let Some(error) = &self.error {
            block = block.title_bottom(Line::from(format!(" {error} ")).fg(Color::Red));
        }
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
    use crate::components::test_support::render;

    fn frames() -> Frames {
        let mut frames = Frames::default();
        frames.open(None, (200, 60));
        frames
    }

    #[test]
    fn renders_the_frame_picker() {
        let mut frames = frames();
        frames.step(2);
        insta::assert_snapshot!(render(&mut frames, 40, 10));
    }

    #[test]
    fn picks_the_pane_the_screen_and_the_presets() {
        let mut frames = frames();
        assert_eq!(frames.choose(), Some(Action::SetFrame(FrameSize::Pane)));
        frames.step(1);
        assert_eq!(frames.choose(), Some(Action::SetFrame(FrameSize::Screen)));
        frames.step(1);
        assert_eq!(
            frames.choose(),
            Some(Action::SetFrame(FrameSize::Custom { cols: 20, rows: 12 }))
        );
        frames.step(1);
        assert_eq!(
            frames.choose(),
            Some(Action::SetFrame(FrameSize::Custom { cols: 60, rows: 20 }))
        );
    }

    #[test]
    fn a_custom_size_is_typed_and_bounded_by_the_terminal() {
        let mut frames = frames();
        assert!(!frames.is_typing());
        frames.step(100);
        assert!(frames.is_typing());
        for c in "40 x 2".chars() {
            frames.push(c);
        }
        frames.push('0');
        assert_eq!(
            frames.choose(),
            Some(Action::SetFrame(FrameSize::Custom { cols: 40, rows: 20 }))
        );
        frames.push('0');
        assert_eq!(frames.choose(), None);
        assert_eq!(
            frames.error.as_deref(),
            Some("at most the terminal's 200x60")
        );
        for _ in 0..3 {
            frames.backspace();
        }
        assert_eq!(frames.choose(), None, "40 x is not a size");
    }

    #[test]
    fn opens_on_the_frame_in_use() {
        let mut frames = Frames::default();
        frames.open(Some(FrameSize::Custom { cols: 60, rows: 20 }), (200, 60));
        assert_eq!(frames.state.selected(), Some(3));
        frames.open(Some(FrameSize::Custom { cols: 33, rows: 9 }), (200, 60));
        assert!(frames.is_typing());
        assert_eq!(frames.custom, "33x9");
    }
}
