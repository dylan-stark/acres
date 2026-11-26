use std::{
    fmt,
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

use charaoke::{
    BlockMode, Blocks, Cells, ColorDepth, Converter, Grid, PixelFormat, QUADRANTS, Ramp, View,
    ansi256,
};
use color_eyre::{Result, eyre};
use ratatui::{prelude::*, widgets::Block};
use tokio::sync::mpsc::UnboundedSender;

use super::{
    Component,
    art_view::{ArtView, FrameSize, Size as Dims},
    centered,
    ramps::DEFAULT_RAMP,
};
use crate::action::{Action, Timing};

/// An image decoded to RGB, ready to load into the art view.
///
/// Decoding takes milliseconds for a full image, so it runs on a blocking thread before the
/// image reaches the app loop (ADR 0013). The pixels are shared, so the image fetcher can keep
/// recent images to show again without decoding them twice.
#[derive(Clone, PartialEq, Eq)]
pub struct Decoded {
    width: NonZeroU32,
    height: NonZeroU32,
    pixels: Arc<[u8]>,
}

impl fmt::Debug for Decoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decoded({}x{})", self.width, self.height)
    }
}

impl Decoded {
    /// Decodes a JPEG, PNG or GIF, scaling a tiny one such as an `lqip` placeholder up smoothly.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut image = image::load_from_memory(bytes)?.into_rgb8();
        if (1..MIN_WIDTH).contains(&image.width()) {
            let height = (u64::from(image.height()) * u64::from(UPSCALED_WIDTH)
                / u64::from(image.width()))
            .max(1);
            image = image::imageops::resize(
                &image,
                UPSCALED_WIDTH,
                u32::try_from(height).unwrap_or(u32::MAX),
                image::imageops::FilterType::Triangle,
            );
        }
        let (Some(width), Some(height)) = (
            NonZeroU32::new(image.width()),
            NonZeroU32::new(image.height()),
        ) else {
            eyre::bail!("the image is empty");
        };
        Ok(Self {
            width,
            height,
            pixels: image.into_raw().into(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width.get()
    }

    pub fn height(&self) -> u32 {
        self.height.get()
    }
}

/// Images narrower than this, such as the few-pixel `lqip` placeholders, are scaled up smoothly
/// before loading, so they read as soft color instead of a few hard blocks.
const MIN_WIDTH: u32 = 64;

/// The width tiny images are scaled up to.
const UPSCALED_WIDTH: u32 = 256;

/// The minimap's largest size in cells, in half blocks (ADR 0017).
pub const MINIMAP: (u32, u32) = (16, 6);

/// How long the minimap stays up after the last pan or zoom.
pub const MINIMAP_FADE: Duration = Duration::from_secs(1);

/// Where the last draw put the art on screen, for mapping the pointer to the image.
#[derive(Debug, Clone, Copy)]
struct Shown {
    /// The part of the art on screen.
    area: Rect,
    /// Columns and rows of the art cut off left and above, when the frame is clipped.
    skip: (u16, u16),
    /// The art's whole size in cells.
    cols: usize,
    rows: usize,
}

/// The art view: renders a view of the loaded image (fit, fill, zoomed and panned) onto a frame
/// of cells (ADR 0017).
pub struct Home {
    converter: Converter,
    /// The loaded image's size in pixels.
    image_size: Option<(NonZeroU32, NonZeroU32)>,
    /// The artwork the image is of, so a new artwork resets the view and a full image replacing
    /// its placeholder does not.
    image_id: Option<u32>,
    /// Block glyphs to draw in place of characters, or `None` for ASCII.
    mode: Option<BlockMode>,
    cells: Cells,
    blocks: Blocks,
    /// The frame size `cells` or `blocks` was rendered for, so a redraw at the same size and view
    /// skips the render.
    rendered_for: Option<(u16, u16)>,
    view: ArtView,
    /// The view last rendered, in fractions of the image, for the minimap.
    rendered_view: View,
    /// The frame picked with `S`, or `None` for the area drawn into: the pane, or the screen in
    /// zen.
    frame: Option<FrameSize>,
    /// The art pane's and the terminal's sizes, which the explorer sets before each draw.
    pane: Option<(u16, u16)>,
    screen: Option<(u16, u16)>,
    /// The last frame drawn, which pans and zooms are worked out against.
    last_frame: Option<Dims>,
    shown: Option<Shown>,
    /// Whether the frame is larger than the area it was drawn into.
    clipped: bool,
    /// The whole image in half blocks, rendered once when it loads.
    minimap: Blocks,
    /// When the view last panned or zoomed, so the minimap can fade.
    moved_at: Option<Instant>,
    depth: ColorDepth,
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
            // Half blocks read best in the explorer's narrow art pane (ADR 0013).
            mode: Some(BlockMode::Half),
            cells: Cells::default(),
            blocks: Blocks::default(),
            rendered_for: None,
            image_id: None,
            view: ArtView::default(),
            rendered_view: View::FULL,
            frame: None,
            pane: None,
            screen: None,
            last_frame: None,
            shown: None,
            clipped: false,
            minimap: Blocks::default(),
            moved_at: None,
            depth,
            action_tx: None,
        }
    }

    /// The sizes the Pane and Screen frames mean, from the explorer's layout.
    pub fn set_sizes(&mut self, pane: (u16, u16), screen: (u16, u16)) {
        self.pane = Some(pane);
        self.screen = Some(screen);
    }

    /// The frame picked, or `None` for the area drawn into.
    pub fn frame(&self) -> Option<FrameSize> {
        self.frame
    }

    /// Whether the last draw cut off part of the frame.
    pub fn clipped(&self) -> bool {
        self.clipped
    }

    /// What the status bar says about the view and frame, such as `fill 2.4x · stamp 20x12`, or
    /// `None` at fit on the default frame.
    pub fn describe(&self) -> Option<String> {
        let view = match (self.image_size, self.last_frame) {
            (Some(image), Some(frame)) => self.view.describe(image, frame, self.mode),
            _ => None,
        };
        let parts: Vec<String> = view
            .into_iter()
            .chain(self.frame.map(FrameSize::name))
            .collect();
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// Back to fit on the default frame, as for a new artwork.
    fn reset_view(&mut self) {
        self.view = ArtView::default();
        self.frame = None;
        self.moved_at = None;
        self.rendered_for = None;
    }

    /// Re-renders for a view that moved, and shows the minimap for a while.
    fn moved(&mut self) {
        self.rendered_for = None;
        self.moved_at = Some(Instant::now());
    }

    /// The image and frame sizes, once something was drawn.
    fn geometry(&self) -> Option<(Dims, Dims)> {
        Some((self.image_size?, self.last_frame?))
    }

    fn zoom(&mut self, steps: i32, at: (f64, f64)) {
        let Some((image, frame)) = self.geometry() else {
            return;
        };
        if self.view.zoom_by(steps, at, image, frame, self.mode) {
            self.moved();
        }
    }

    fn pan(&mut self, cols: f64, rows: f64) {
        let Some((image, frame)) = self.geometry() else {
            return;
        };
        if self.view.pan((cols, rows), image, frame, self.mode) {
            self.moved();
        }
    }

    /// A tenth of the art drawn, in cells, and at least one.
    fn tenth(&self) -> (f64, f64) {
        let (cols, rows) = self.art_size();
        (
            (cols as f64 / 10.0).round().max(1.0),
            (rows as f64 / 10.0).round().max(1.0),
        )
    }

    /// Where the screen cell at `column`, `row` falls on the art, in fractions of it.
    fn pointer(&self, column: u16, row: u16) -> (f64, f64) {
        let Some(shown) = self.shown else {
            return (0.5, 0.5);
        };
        let along = |at: u16, start: u16, skip: u16, len: usize| {
            if len == 0 {
                return 0.5;
            }
            let cell = f64::from(at) - f64::from(start) + f64::from(skip) + 0.5;
            (cell / len as f64).clamp(0.0, 1.0)
        };
        (
            along(column, shown.area.x, shown.skip.0, shown.cols),
            along(row, shown.area.y, shown.skip.1, shown.rows),
        )
    }

    /// The frame's size in cells for drawing into `area`.
    fn frame_size(&self, area: Rect) -> (u16, u16) {
        let here = (area.width, area.height);
        match self.frame {
            None => here,
            Some(FrameSize::Pane) => self.pane.unwrap_or(here),
            Some(FrameSize::Screen) => self.screen.unwrap_or(here),
            Some(FrameSize::Custom { cols, rows }) => (cols, rows),
        }
    }

    fn send(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Puts `message` in the log drawer.
    fn log(&self, message: String) {
        tracing::warn!("{message}");
        self.send(Action::Log(message));
    }

    /// Loads a decoded image, adding the load time to `timing` for the HUD.
    fn set_image(&mut self, image: &Decoded, timing: Timing) -> Result<()> {
        let start = Instant::now();
        self.converter
            .set_image(&image.pixels, image.width, image.height, PixelFormat::Rgb8)?;
        let load = start.elapsed();
        self.image_size = Some((image.width, image.height));
        self.rendered_for = None;
        let (cols, rows) = MINIMAP;
        let grid = Grid::fit_within(
            image.width,
            image.height,
            NonZeroU32::new(cols).expect("not zero"),
            NonZeroU32::new(rows).expect("not zero"),
        );
        self.converter
            .render_blocks(grid, BlockMode::Half, &mut self.minimap);
        self.send(Action::Timing(timing.stage("load", load)));
        Ok(())
    }

    /// Renders the view onto a frame of `cols` by `rows`, unless it already is.
    fn render_for(&mut self, cols: u16, rows: u16) -> Option<Timing> {
        if self.rendered_for == Some((cols, rows)) {
            return None;
        }
        self.rendered_for = Some((cols, rows));
        let (Some(max_cols), Some(max_rows)) =
            (NonZeroU32::new(cols.into()), NonZeroU32::new(rows.into()))
        else {
            self.cells = Cells::default();
            self.blocks = Blocks::default();
            return None;
        };
        self.last_frame = Some((max_cols, max_rows));
        let image = self.image_size?;
        self.view
            .clamp_center(image, (max_cols, max_rows), self.mode);
        let (grid, view) = self.view.layout(image, (max_cols, max_rows), self.mode);
        self.rendered_view = view;
        let start = Instant::now();
        match self.mode {
            Some(mode) => self
                .converter
                .render_blocks_view(grid, view, mode, &mut self.blocks),
            None => self.converter.render_view(grid, view, &mut self.cells),
        }
        let label = match self.view.describe(image, (max_cols, max_rows), self.mode) {
            Some(view) => format!("view {view} {cols}x{rows} {}", mode_name(self.mode)),
            None => format!("resize {cols}x{rows} {}", mode_name(self.mode)),
        };
        Some(Timing::new(label).stage("render", start.elapsed()))
    }

    /// Draws the minimap in the bottom-right corner of `canvas`, with the view outlined, while
    /// a zoomed view is moving.
    fn draw_minimap(&self, frame: &mut Frame, canvas: Rect) {
        if self.view.is_fit()
            || self.minimap.is_empty()
            || !self.moved_at.is_some_and(|at| at.elapsed() < MINIMAP_FADE)
        {
            return;
        }
        let (cols, rows) = (self.minimap.cols() as u16, self.minimap.rows() as u16);
        if canvas.width < cols + 2 || canvas.height < rows + 2 {
            return;
        }
        let area = Rect::new(
            canvas.right() - cols - 1,
            canvas.bottom() - rows - 1,
            cols,
            rows,
        );
        frame.render_widget(
            BlocksWidget {
                blocks: &self.minimap,
                depth: self.depth,
                skip: (0, 0),
            },
            area,
        );
        let view = self.rendered_view;
        let span = |start: f64, size: f64, cells: u16| {
            let n = f64::from(cells);
            let first = (start * n).floor().clamp(0.0, n - 1.0) as u16;
            let mut last = ((start + size) * n).ceil().clamp(1.0, n) as u16 - 1;
            let mut first = first.min(last);
            if first == last && cells > 1 {
                if last + 1 < cells {
                    last += 1;
                } else {
                    first -= 1;
                }
            }
            (first, last)
        };
        let (left, right) = span(view.x(), view.width(), cols);
        let (top, bottom) = span(view.y(), view.height(), rows);
        let buf = frame.buffer_mut();
        for y in top..=bottom {
            for x in left..=right {
                let edge_x = x == left || x == right;
                let edge_y = y == top || y == bottom;
                let glyph = match (x, y) {
                    _ if !edge_x && !edge_y => continue,
                    (x, y) if x == left && y == top => '┌',
                    (x, y) if x == right && y == top => '┐',
                    (x, y) if x == left && y == bottom => '└',
                    (x, y) if x == right && y == bottom => '┘',
                    _ if edge_y => '─',
                    _ => '│',
                };
                if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                    cell.set_char(glyph).set_fg(Color::White);
                }
            }
        }
    }

    /// Columns and rows of the art last rendered.
    fn art_size(&self) -> (usize, usize) {
        match self.mode {
            Some(_) => (self.blocks.cols(), self.blocks.rows()),
            None => (self.cells.cols(), self.cells.rows()),
        }
    }
}

/// Where art `len` cells long goes on one axis of a canvas at `start`, `avail` cells long,
/// drawn for a frame `frame_len` cells long: its start on screen, how many cells are cut off
/// before it, and how many show.
fn place(start: u16, avail: u16, frame_len: u16, len: u16) -> (u16, u16, u16) {
    if frame_len <= avail {
        // Centered as the pane always has, so fit draws exactly as before.
        let at = centered(Rect::new(start, 0, avail, 1), len, 1);
        return (at.x, 0, at.width);
    }
    // A clipped frame shows its middle.
    let frame_start = i32::from(start) - (i32::from(frame_len) - i32::from(avail)) / 2;
    let art_start = frame_start + (i32::from(frame_len) - i32::from(len)) / 2;
    let first = art_start.max(i32::from(start));
    let end = (art_start + i32::from(len)).min(i32::from(start) + i32::from(avail));
    let shown = (end - first).max(0) as u16;
    (first as u16, (first - art_start) as u16, shown)
}

/// The art mode after `mode`: ASCII, then half blocks, then quadrant blocks.
fn next_mode(mode: Option<BlockMode>) -> Option<BlockMode> {
    match mode {
        None => Some(BlockMode::Half),
        Some(BlockMode::Half) => Some(BlockMode::Quadrant),
        Some(_) => None,
    }
}

/// How the HUD and logs name an art mode.
fn mode_name(mode: Option<BlockMode>) -> &'static str {
    match mode {
        None => "ascii",
        Some(BlockMode::Half) => "half",
        Some(_) => "quadrant",
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
    /// Columns and rows to leave off the left and top, for a clipped frame.
    pub skip: (u16, u16),
}

impl Widget for CellsWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.cells.cols();
        if cols == 0 {
            return;
        }
        let (skip_x, skip_y) = (usize::from(self.skip.0), usize::from(self.skip.1));
        let rows = self
            .cells
            .chars()
            .chunks(cols)
            .zip(self.cells.colors().chunks(cols))
            .skip(skip_y);
        for ((chars, colors), y) in rows.zip(area.top()..area.bottom()) {
            let cells = chars.iter().zip(colors).skip(skip_x);
            for ((&ch, &rgb), x) in cells.zip(area.left()..area.right()) {
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

/// Draws [`Blocks`] straight into a Ratatui buffer: one glyph with a foreground and a background
/// color per cell.
///
/// `charaoke/benches/tui_proxy.rs` carries a copy of this widget for its `drag_half` and
/// `drag_quadrant` benches, and a test below checks the two draw the same buffer.
pub struct BlocksWidget<'a> {
    pub blocks: &'a Blocks,
    pub depth: ColorDepth,
    /// Columns and rows to leave off the left and top, for a clipped frame.
    pub skip: (u16, u16),
}

impl Widget for BlocksWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cols = self.blocks.cols();
        if cols == 0 {
            return;
        }
        let color = |[r, g, b]: [u8; 3]| match self.depth {
            ColorDepth::TrueColor => Color::Rgb(r, g, b),
            _ => Color::Indexed(ansi256([r, g, b])),
        };
        let (skip_x, skip_y) = (usize::from(self.skip.0), usize::from(self.skip.1));
        let rows = self
            .blocks
            .masks()
            .chunks(cols)
            .zip(self.blocks.foreground().chunks(cols))
            .zip(self.blocks.background().chunks(cols))
            .skip(skip_y);
        for (((masks, fgs), bgs), y) in rows.zip(area.top()..area.bottom()) {
            let cells = masks.iter().zip(fgs).zip(bgs).skip(skip_x);
            for (((&mask, &fg), &bg), x) in cells.zip(area.left()..area.right()) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(QUADRANTS[usize::from(mask)])
                        .set_fg(color(fg))
                        .set_bg(color(bg));
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
            Action::UpdateImage {
                id, image, timing, ..
            } => {
                if self.image_id != Some(id) {
                    self.image_id = Some(id);
                    self.reset_view();
                }
                if let Err(err) = self.set_image(&image, timing) {
                    self.log(format!("failed to load image {id}: {err}"));
                }
            }
            Action::ClearImage => {
                self.image_size = None;
                self.image_id = None;
                self.cells = Cells::default();
                self.blocks = Blocks::default();
                self.minimap = Blocks::default();
                self.reset_view();
            }
            Action::ZoomIn => self.zoom(1, (0.5, 0.5)),
            Action::ZoomOut => self.zoom(-1, (0.5, 0.5)),
            Action::ZoomAt { column, row, steps } => {
                let at = self.pointer(column, row);
                self.zoom(steps.into(), at);
            }
            Action::PanLeft => self.pan(-1.0, 0.0),
            Action::PanRight => self.pan(1.0, 0.0),
            Action::PanUp => self.pan(0.0, -1.0),
            Action::PanDown => self.pan(0.0, 1.0),
            Action::PanLeftFar => self.pan(-self.tenth().0, 0.0),
            Action::PanRightFar => self.pan(self.tenth().0, 0.0),
            Action::PanUpFar => self.pan(0.0, -self.tenth().1),
            Action::PanDownFar => self.pan(0.0, self.tenth().1),
            Action::PanBy { cols, rows } => self.pan(cols.into(), rows.into()),
            Action::FitArt => {
                self.view = ArtView::default();
                self.rendered_for = None;
            }
            Action::FillArt => {
                self.view = ArtView::fill();
                self.moved();
            }
            Action::SetFrame(frame) => {
                self.frame = Some(frame);
                self.rendered_for = None;
            }
            Action::SetRamp(name) => match Ramp::preset(&name) {
                Some(ramp) => {
                    let ramp =
                        ramp.with_brightness_offset(self.converter.ramp().brightness_offset());
                    self.converter.set_ramp(ramp);
                    self.rendered_for = None;
                }
                None => self.log(format!("no ramp named {name}")),
            },
            Action::CycleArtMode => {
                self.mode = next_mode(self.mode);
                self.rendered_for = None;
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let (cols, rows) = self.frame_size(area);
        let framed = self.frame.is_some() && (cols, rows) != (area.width, area.height);
        self.clipped = cols > area.width || rows > area.height;
        // A frame smaller than the area floats in it with a thin border that is not the art.
        let bordered = framed && cols + 2 <= area.width && rows + 2 <= area.height;
        let canvas = if bordered {
            let outer = centered(area, cols + 2, rows + 2);
            let block = Block::bordered().border_style(Style::new().fg(Color::DarkGray));
            let inner = block.inner(outer);
            frame.render_widget(block, outer);
            inner
        } else {
            centered(area, cols, rows)
        };
        let rendered = self.render_for(cols, rows);
        let start = Instant::now();
        let (art_cols, art_rows) = self.art_size();
        let (x, skip_x, width) = place(canvas.x, canvas.width, cols, art_cols as u16);
        let (y, skip_y, height) = place(canvas.y, canvas.height, rows, art_rows as u16);
        let art_area = Rect::new(x, y, width, height);
        let skip = (skip_x, skip_y);
        self.shown = Some(Shown {
            area: art_area,
            skip,
            cols: art_cols,
            rows: art_rows,
        });
        match self.mode {
            Some(_) => frame.render_widget(
                BlocksWidget {
                    blocks: &self.blocks,
                    depth: self.depth,
                    skip,
                },
                art_area,
            ),
            None => frame.render_widget(
                CellsWidget {
                    cells: &self.cells,
                    depth: self.depth,
                    skip,
                },
                art_area,
            ),
        }
        self.draw_minimap(frame, canvas);
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
#[path = "../../../charaoke/benches/common/blocks_widget.rs"]
mod bench_blocks_widget;

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

    /// A view of the fixture image in ASCII, which most tests check.
    fn with_image() -> Home {
        with_image_in(None)
    }

    fn with_image_in(mode: Option<BlockMode>) -> Home {
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        home.mode = mode;
        home.update(Action::UpdateImage {
            id: 1,
            image: Decoded::decode(IMAGE).unwrap(),
            full: true,
            timing: Timing::new("image 1").stage("fetch", Duration::ZERO),
        })
        .unwrap();
        home
    }

    /// La Grande Jatte's `lqip` placeholder from the API, 8x5 pixels.
    const LQIP: &str = "R0lGODlhCAAFAPUAAE1WUFVTU1NWUFZYVllbVlpaW1tcWF1cXVxiWl1jWl9mX2NmVWVnXF9eZlxdaGZkYmpmZWRtYGZuY2htZGpsanBsZXVwZXB5Znp+ZXJwaYSAaYiBaYqIbIeAdI2DcYqFcI+FcoiCd5iRcouJgIyKhoyNi4mMjo2RkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAAAAAAALAAAAAAIAAUAAAYlwBLmMmEkCAHTaRTqfB4QEmgj8mgaGY6lsohIHAXKwQBQIAaCIAA7";

    #[test]
    fn a_tiny_placeholder_is_scaled_up_to_fill_the_pane() {
        use base64::Engine;
        let gif = base64::engine::general_purpose::STANDARD
            .decode(LQIP)
            .unwrap();
        let mut home = Home::with_depth(ColorDepth::TrueColor);
        home.update(Action::UpdateImage {
            id: 27992,
            image: Decoded::decode(&gif).unwrap(),
            full: false,
            timing: Timing::new("image 27992").stage("fetch", Duration::ZERO),
        })
        .unwrap();
        let (w, _) = home.image_size.unwrap();
        assert_eq!(w.get(), UPSCALED_WIDTH);
        // A tall pane, so the width is what limits the fit.
        render(&mut home, 40, 40);
        assert_eq!(home.blocks.cols(), 40, "fills the pane's width");

        home.update(Action::ClearImage).unwrap();
        assert!(render(&mut home, 40, 12).trim().is_empty());
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
        home.mode = None;
        home.register_action_handler(tx).unwrap();
        home.update(Action::UpdateImage {
            id: 7,
            image: Decoded::decode(IMAGE).unwrap(),
            full: true,
            timing: Timing::new("image 7").stage("fetch", Duration::ZERO),
        })
        .unwrap();
        let Ok(Action::Timing(image)) = rx.try_recv() else {
            panic!("expected image timing");
        };
        assert_eq!(image.label, "image 7");
        let stages: Vec<&str> = image.stages.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(stages, ["fetch", "load"]);

        render(&mut home, 40, 12);
        let Ok(Action::Timing(resize)) = rx.try_recv() else {
            panic!("expected resize timing");
        };
        assert_eq!(resize.label, "resize 40x12 ascii");
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
        home.render_for(60, 20);
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
                    skip: (0, 0),
                },
                area,
            )
        });
        let copy = draw(&|f| f.render_widget(bench_widget::CellsWidget(&home.cells), area));
        assert_eq!(real, copy);
    }

    #[test]
    fn starts_in_half_blocks() {
        assert_eq!(Home::new().mode, Some(BlockMode::Half));
    }

    #[test]
    fn renders_half_blocks_centered() {
        let mut home = with_image_in(Some(BlockMode::Half));
        insta::assert_snapshot!(render(&mut home, 40, 12));
    }

    #[test]
    fn renders_quadrant_blocks_centered() {
        let mut home = with_image_in(Some(BlockMode::Quadrant));
        insta::assert_snapshot!(render(&mut home, 40, 12));
    }

    #[test]
    fn cycle_art_mode_re_renders_and_wraps_to_ascii() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut home = with_image();
        home.register_action_handler(tx).unwrap();
        render(&mut home, 40, 12);
        for expected in ["half", "quadrant", "ascii"] {
            while rx.try_recv().is_ok() {}
            home.update(Action::CycleArtMode).unwrap();
            render(&mut home, 40, 12);
            let Ok(Action::Timing(resize)) = rx.try_recv() else {
                panic!("expected a {expected} render");
            };
            assert_eq!(resize.label, format!("resize 40x12 {expected}"));
            // Every mode draws on the same grid of cells.
            assert_eq!(home.art_size(), (35, 12), "{expected}");
        }
        assert_eq!(home.mode, None);
    }

    #[test]
    fn blocks_widget_draws_what_the_bench_copy_draws() {
        let mut home = with_image_in(Some(BlockMode::Quadrant));
        let area = Rect::new(0, 0, 60, 20);
        home.render_for(60, 20);
        let mut real = Buffer::empty(area);
        BlocksWidget {
            blocks: &home.blocks,
            depth: ColorDepth::TrueColor,
            skip: (0, 0),
        }
        .render(area, &mut real);
        let mut copy = Buffer::empty(area);
        bench_blocks_widget::BlocksWidget(&home.blocks).render(area, &mut copy);
        assert_eq!(real, copy);
    }

    #[test]
    fn blocks_widget_sets_both_colors() {
        let mut home = with_image_in(Some(BlockMode::Half));
        home.render_for(8, 4);
        for depth in [ColorDepth::TrueColor, ColorDepth::Ansi256] {
            let mut buf = Buffer::empty(Rect::new(0, 0, 8, 4));
            BlocksWidget {
                blocks: &home.blocks,
                depth,
                skip: (0, 0),
            }
            .render(buf.area, &mut buf);
            let ([fr, fg, fb], [br, bg, bb]) =
                (home.blocks.foreground()[0], home.blocks.background()[0]);
            let (fore, back) = match depth {
                ColorDepth::TrueColor => (Color::Rgb(fr, fg, fb), Color::Rgb(br, bg, bb)),
                _ => (
                    Color::Indexed(ansi256([fr, fg, fb])),
                    Color::Indexed(ansi256([br, bg, bb])),
                ),
            };
            assert_eq!(buf[(0, 0)].symbol(), "▀");
            assert_eq!((buf[(0, 0)].fg, buf[(0, 0)].bg), (fore, back));
        }
    }

    #[test]
    fn widget_uses_indexed_colors_below_truecolor() {
        let mut cells_home = with_image();
        cells_home.render_for(8, 4);
        let mut buf = Buffer::empty(Rect::new(0, 0, 8, 4));
        CellsWidget {
            cells: &cells_home.cells,
            depth: ColorDepth::Ansi256,
            skip: (0, 0),
        }
        .render(buf.area, &mut buf);
        let rgb = cells_home.cells.colors()[0];
        assert_eq!(buf[(0, 0)].fg, Color::Indexed(ansi256(rgb)));
    }

    /// Renders `home` in a `width` x `height` pane, then applies `actions`, as keys would.
    fn after(home: &mut Home, width: u16, height: u16, actions: &[Action]) -> String {
        render(home, width, height);
        for action in actions {
            home.update(action.clone()).unwrap();
        }
        render(home, width, height)
    }

    #[test]
    fn fill_covers_the_pane() {
        let mut home = with_image();
        let text = after(&mut home, 40, 12, &[Action::FillArt]);
        assert_eq!(home.art_size(), (40, 12));
        assert_eq!(home.describe().as_deref(), Some("fill"));
        // The minimap shows only while the view moves, and fill just moved it.
        home.moved_at = None;
        insta::assert_snapshot!(render(&mut home, 40, 12));
        assert_ne!(text, render(&mut home, 40, 12), "the minimap was up");
    }

    #[test]
    fn zooming_past_whole_pixels_draws_even_blocks() {
        let mut home = with_image_in(Some(BlockMode::Half));
        let zoom_in = vec![Action::ZoomIn; 20];
        after(&mut home, 40, 12, &zoom_in);
        let (image, frame) = home.geometry().unwrap();
        let scale = home.view.scale(image, frame, home.mode);
        assert!(scale >= 2.0 && scale.fract() == 0.0, "{scale}");
        // Every run of one color along the middle row, except at the edges, is `scale` cells.
        let row = home.blocks.rows() / 2;
        let cols = home.blocks.cols();
        let colors = &home.blocks.foreground()[row * cols..(row + 1) * cols];
        let mut runs = vec![1];
        for pair in colors.windows(2) {
            if pair[0] == pair[1] {
                *runs.last_mut().unwrap() += 1;
            } else {
                runs.push(1);
            }
        }
        let inner = &runs[1..runs.len() - 1];
        assert!(
            inner.iter().all(|&run| run % scale as usize == 0),
            "{runs:?}"
        );
    }

    #[test]
    fn zoom_out_and_fit_go_back_to_todays_art() {
        let mut home = with_image();
        let fit = render(&mut home, 40, 12);
        after(
            &mut home,
            40,
            12,
            &[Action::ZoomIn, Action::ZoomIn, Action::PanLeft],
        );
        assert_eq!(home.describe().as_deref(), Some("fit 1.6x"));
        home.update(Action::FitArt).unwrap();
        assert_eq!(render(&mut home, 40, 12), fit);
        assert_eq!(home.describe(), None);
    }

    #[test]
    fn a_new_artwork_resets_the_view_and_its_full_image_does_not() {
        let mut home = with_image();
        after(&mut home, 40, 12, &[Action::FillArt]);
        home.update(Action::SetFrame(FrameSize::Custom { cols: 20, rows: 12 }))
            .unwrap();
        let image = Decoded::decode(IMAGE).unwrap();
        let again = |id| Action::UpdateImage {
            id,
            image: image.clone(),
            full: true,
            timing: Timing::new("image"),
        };
        home.update(again(1)).unwrap();
        assert_eq!(home.describe().as_deref(), Some("fill · stamp 20x12"));
        home.update(again(2)).unwrap();
        assert_eq!(home.describe(), None);
        assert_eq!(home.frame(), None);
    }

    #[test]
    fn a_stamp_floats_in_a_border() {
        let mut home = with_image_in(Some(BlockMode::Half));
        home.update(Action::SetFrame(FrameSize::Custom { cols: 20, rows: 12 }))
            .unwrap();
        home.update(Action::FillArt).unwrap();
        home.moved_at = None;
        insta::assert_snapshot!(render(&mut home, 40, 16));
        assert_eq!(home.art_size(), (20, 12));
        assert!(!home.clipped());
    }

    #[test]
    fn a_frame_larger_than_the_pane_shows_its_middle() {
        let mut home = with_image();
        home.update(Action::SetFrame(FrameSize::Custom { cols: 60, rows: 20 }))
            .unwrap();
        let text = render(&mut home, 30, 10);
        assert!(home.clipped());
        assert_eq!(home.art_size(), (60, 20));
        let shown = home.shown.unwrap();
        assert_eq!((shown.area.width, shown.area.height), (30, 10));
        assert_eq!(shown.skip, (15, 5));
        // The middle of the 60x20 art, as drawn on a screen that fits it whole.
        let whole = render(&mut home, 60, 20);
        let middle: Vec<String> = whole
            .lines()
            .skip(5)
            .take(10)
            .map(|line| line.chars().skip(15).take(30).collect::<String>())
            .map(|line| line.trim_end().to_string())
            .collect();
        assert_eq!(text, middle.join("\n"));
    }

    #[test]
    fn the_minimap_outlines_the_view_while_it_moves() {
        let mut home = with_image_in(Some(BlockMode::Half));
        let text = after(
            &mut home,
            60,
            20,
            &[Action::ZoomIn, Action::ZoomIn, Action::ZoomIn],
        );
        insta::assert_snapshot!(text);
        home.moved_at = Some(Instant::now() - MINIMAP_FADE);
        assert!(!render(&mut home, 60, 20).contains('┌'), "faded");
    }

    #[test]
    fn scrolling_zooms_about_the_pointer() {
        let mut home = with_image();
        after(&mut home, 40, 12, &[Action::FillArt]);
        // The image point at the middle of the cell under the pointer stays under it.
        let (column, row) = (10, 3);
        let at = |view: View| {
            (
                view.x() + (f64::from(column) + 0.5) / 40.0 * view.width(),
                view.y() + (f64::from(row) + 0.5) / 12.0 * view.height(),
            )
        };
        let before = at(home.rendered_view);
        home.update(Action::ZoomAt {
            column,
            row,
            steps: 2,
        })
        .unwrap();
        render(&mut home, 40, 12);
        let after = at(home.rendered_view);
        assert!(home.rendered_view.width() < 0.7, "zoomed in");
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn a_new_frame_keeps_the_center_where_the_view_is_drawn() {
        let mut home = with_image_in(Some(BlockMode::Half));
        let mut actions = vec![Action::ZoomIn; 8];
        actions.extend(vec![Action::PanRightFar; 50]);
        after(&mut home, 60, 20, &actions);
        // At the right edge of a 60x20 frame, then onto a wider one, which shows more of the
        // image at the same zoom, so that center would put the view past the edge.
        home.update(Action::SetFrame(FrameSize::Custom { cols: 120, rows: 20 }))
            .unwrap();
        render(&mut home, 120, 20);
        let drawn = home.rendered_view;
        let middle = drawn.x() + drawn.width() / 2.0;
        assert!((home.view.center.0 - middle).abs() < 1e-9, "{:?}", home.view);
        // So a pan moves one cell, not first back to the edge.
        home.update(Action::PanLeft).unwrap();
        render(&mut home, 120, 20);
        let cell = drawn.width() / 120.0;
        assert!((drawn.x() - home.rendered_view.x() - cell).abs() < 1e-9);
    }

    #[test]
    fn color_depth_reads_colorterm() {
        let env = |value: &'static str| move |key: &str| (key == "COLORTERM").then(|| value.into());
        assert_eq!(color_depth(env("truecolor")), ColorDepth::TrueColor);
        assert_eq!(color_depth(env("24bit")), ColorDepth::TrueColor);
        assert_eq!(color_depth(env("")), ColorDepth::Ansi256);
        assert_eq!(color_depth(|_| None), ColorDepth::Ansi256);
    }
}
