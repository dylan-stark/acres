use std::{num::NonZeroU32, time::Instant};

use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat, Ramp, ansi256};
use color_eyre::{Result, eyre};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::{Component, ramps::DEFAULT_RAMP};
use crate::action::{Action, Timing};

/// The art view: renders the loaded image to fit its pane, with an optional log pane.
pub struct Home {
    converter: Converter,
    /// The loaded image's size in pixels.
    image_size: Option<(NonZeroU32, NonZeroU32)>,
    cells: Cells,
    /// The pane `cells` was rendered for, so a redraw at the same size skips the render.
    rendered_for: Option<Rect>,
    depth: ColorDepth,
    show_logs: bool,
    log: Vec<String>,
    messages_width: u16,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Default for Home {
    fn default() -> Self {
        Self::new()
    }
}

impl Home {
    pub fn new() -> Self {
        Self::with_depth(color_depth(|key| std::env::var(key).ok()))
    }

    /// An art view that draws colors at `depth`, ignoring the environment.
    pub fn with_depth(depth: ColorDepth) -> Self {
        Self {
            converter: Converter::new(Ramp::preset(DEFAULT_RAMP).expect("default is a preset")),
            image_size: None,
            cells: Cells::default(),
            rendered_for: None,
            depth,
            show_logs: false,
            log: Vec::new(),
            messages_width: 0,
            action_tx: None,
        }
    }

    fn send(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    fn log(&mut self, message: String) {
        self.messages_width = self.messages_width.max(message.len() as u16);
        self.log.push(message);
    }

    /// Decodes and loads a fetched image, reporting each stage's time.
    fn set_image(&mut self, id: u32, image: &[u8], timing: Timing) -> Result<()> {
        let start = Instant::now();
        let image = image::load_from_memory(image)?.into_rgb8();
        let decode = start.elapsed();
        let (Some(w), Some(h)) = (
            NonZeroU32::new(image.width()),
            NonZeroU32::new(image.height()),
        ) else {
            eyre::bail!("image {id} is empty");
        };
        let start = Instant::now();
        self.converter
            .set_image(image.as_raw(), w, h, PixelFormat::Rgb8)?;
        let load = start.elapsed();
        self.image_size = Some((w, h));
        self.rendered_for = None;
        self.send(Action::Timing(
            timing.stage("decode", decode).stage("load", load),
        ));
        Ok(())
    }

    /// Renders the image to fit `area`, unless it already fits.
    fn fit(&mut self, area: Rect) -> Option<Timing> {
        if self.rendered_for == Some(area) {
            return None;
        }
        self.rendered_for = Some(area);
        let (Some(max_cols), Some(max_rows)) = (
            NonZeroU32::new(area.width.into()),
            NonZeroU32::new(area.height.into()),
        ) else {
            self.cells = Cells::default();
            return None;
        };
        let (w, h) = self.image_size?;
        let start = Instant::now();
        self.converter
            .render(Grid::fit_within(w, h, max_cols, max_rows), &mut self.cells);
        Some(
            Timing::new(format!("resize {}x{}", area.width, area.height))
                .stage("render", start.elapsed()),
        )
    }
}

/// Picks the TUI's color depth from `COLORTERM`, falling back to 256-color when unsure.
fn color_depth(env: impl Fn(&str) -> Option<String>) -> ColorDepth {
    match env("COLORTERM").as_deref() {
        Some("truecolor" | "24bit") => ColorDepth::TrueColor,
        _ => ColorDepth::Ansi256,
    }
}

/// Draws [`Cells`] straight into a Ratatui buffer, one character and foreground color per cell.
///
/// `charaoke/benches/tui_proxy.rs` carries a copy of this widget for its `drag` bench, and a test
/// below checks the two draw the same buffer.
pub struct CellsWidget<'a> {
    pub cells: &'a Cells,
    pub depth: ColorDepth,
}

impl Widget for CellsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.cells.cols();
        if cols == 0 {
            return;
        }
        let rows = self
            .cells
            .chars()
            .chunks(cols)
            .zip(self.cells.colors().chunks(cols));
        for ((chars, colors), y) in rows.zip(area.top()..area.bottom()) {
            for ((&ch, &rgb), x) in chars.iter().zip(colors).zip(area.left()..area.right()) {
                let color = match self.depth {
                    ColorDepth::TrueColor => Color::Rgb(rgb[0], rgb[1], rgb[2]),
                    _ => Color::Indexed(ansi256(rgb)),
                };
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(char::from(ch)).set_fg(color);
                }
            }
        }
    }
}

impl Component for Home {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick | Action::Render | Action::FrameTime(_) | Action::Timing(_) => {}
            Action::UpdateImage { id, image, fetch } => {
                self.log(format!("action: UpdateImage(id: {id})"));
                let timing = Timing::new(format!("image {id}")).stage("fetch", fetch);
                if let Err(err) = self.set_image(id, &image, timing) {
                    self.log(format!("failed to load image {id}: {err}"));
                }
            }
            Action::SetRamp(name) => {
                self.log(format!("action: SetRamp({name})"));
                match Ramp::preset(&name) {
                    Some(ramp) => {
                        let ramp =
                            ramp.with_brightness_offset(self.converter.ramp().brightness_offset());
                        self.converter.set_ramp(ramp);
                        self.rendered_for = None;
                    }
                    None => self.log(format!("no ramp named {name}")),
                }
            }
            Action::ToggleLogs => self.show_logs = !self.show_logs,
            Action::Log(message) => self.log(message),
            _ => self.log(format!("action: {}", action)),
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

        let rendered = self.fit(view_area);
        let start = Instant::now();
        let art = Rect::new(0, 0, self.cells.cols() as u16, self.cells.rows() as u16);
        let [_, middle, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(art.width),
            Constraint::Fill(1),
        ])
        .areas(view_area);
        let [_, art_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(art.height),
            Constraint::Fill(1),
        ])
        .areas(middle);
        frame.render_widget(
            CellsWidget {
                cells: &self.cells,
                depth: self.depth,
            },
            art_area,
        );
        if let Some(timing) = rendered {
            self.send(Action::Timing(timing.stage("draw", start.elapsed())));
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../charaoke/benches/common/cells_widget.rs"]
mod bench_widget;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::components::test_support::render;

    /// A Sunday on La Grande Jatte (843x565), from the demo fixtures.
    const IMAGE: &[u8] = include_bytes!(
        "../../tests/fixtures/iiif/2d484387-2509-5e8e-2c43-22f9981972eb/full/843,/0/default.jpg"
    );

    fn with_image() -> Home {
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        home.update(Action::UpdateImage {
            id: 1,
            image: IMAGE.into(),
            fetch: Duration::ZERO,
        })
        .unwrap();
        home
    }

    #[test]
    fn renders_empty() {
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        insta::assert_snapshot!(render(&mut home, 20, 6));
    }

    #[test]
    fn renders_ascii_centered() {
        let mut home = with_image();
        insta::assert_snapshot!(render(&mut home, 40, 12));
    }

    #[test]
    fn fits_art_to_the_pane() {
        let mut home = with_image();
        render(&mut home, 200, 20);
        // An 843x565 image in 20 rows of 2:1 cells is 59 columns wide.
        assert_eq!((home.cells.cols(), home.cells.rows()), (59, 20));
        render(&mut home, 40, 50);
        assert_eq!((home.cells.cols(), home.cells.rows()), (40, 13));
    }

    #[test]
    fn reports_timings_only_when_it_renders() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        home.register_action_handler(tx).unwrap();
        home.update(Action::UpdateImage {
            id: 7,
            image: IMAGE.into(),
            fetch: Duration::from_millis(5),
        })
        .unwrap();
        let Ok(Action::Timing(image)) = rx.try_recv() else {
            panic!("expected image timing");
        };
        assert_eq!(image.label, "image 7");
        let stages: Vec<&str> = image.stages.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(stages, ["fetch", "decode", "load"]);

        render(&mut home, 40, 12);
        let Ok(Action::Timing(resize)) = rx.try_recv() else {
            panic!("expected resize timing");
        };
        assert_eq!(resize.label, "resize 40x12");
        render(&mut home, 40, 12);
        assert!(rx.try_recv().is_err(), "same size skips the render");

        home.update(Action::SetRamp("minimal".into())).unwrap();
        render(&mut home, 40, 12);
        assert!(matches!(rx.try_recv(), Ok(Action::Timing(_))));
    }

    #[test]
    fn set_ramp_changes_the_characters() {
        let mut home = with_image();
        home.update(Action::SetRamp("fast".into())).unwrap();
        let text = render(&mut home, 40, 12);
        assert!(text.chars().all(|c| matches!(c, ' ' | '#' | '\n')));
    }

    #[test]
    fn widget_draws_what_the_bench_copy_draws() {
        let mut home = with_image();
        let area = Rect::new(0, 0, 60, 20);
        home.fit(area);
        let draw = |widget: &dyn Fn(&mut Frame)| {
            let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
            terminal.draw(|f| widget(f)).unwrap();
            terminal.backend().buffer().clone()
        };
        let real = draw(&|f| {
            f.render_widget(
                CellsWidget {
                    cells: &home.cells,
                    depth: ColorDepth::TrueColor,
                },
                area,
            )
        });
        let copy = draw(&|f| f.render_widget(bench_widget::CellsWidget(&home.cells), area));
        assert_eq!(real, copy);
    }

    #[test]
    fn widget_uses_indexed_colors_below_truecolor() {
        let mut cells_home = with_image();
        cells_home.fit(Rect::new(0, 0, 8, 4));
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 4));
        CellsWidget {
            cells: &cells_home.cells,
            depth: ColorDepth::Ansi256,
        }
        .render(buf.area, &mut buf);
        let rgb = cells_home.cells.colors()[0];
        assert_eq!(buf[(0, 0)].fg, Color::Indexed(ansi256(rgb)));
    }

    #[test]
    fn color_depth_reads_colorterm() {
        let env = |value: &'static str| move |key: &str| (key == "COLORTERM").then(|| value.into());
        assert_eq!(color_depth(env("truecolor")), ColorDepth::TrueColor);
        assert_eq!(color_depth(env("24bit")), ColorDepth::TrueColor);
        assert_eq!(color_depth(env("")), ColorDepth::Ansi256);
        assert_eq!(color_depth(|_| None), ColorDepth::Ansi256);
    }

    #[test]
    fn renders_logs_only_when_toggled() {
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        home.update(Action::Log("first".into())).unwrap();
        home.update(Action::Log("second".into())).unwrap();
        assert!(!render(&mut home, 24, 4).contains("second"));

        home.update(Action::ToggleLogs).unwrap();
        insta::assert_snapshot!(render(&mut home, 24, 4));
    }
}
