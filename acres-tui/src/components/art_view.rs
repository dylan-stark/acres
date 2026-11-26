//! What the art pane shows of its image: fit or fill, a zoom and a center, worked out against a
//! frame of cells (ADR 0017).
//!
//! Sizes are in units of a cell's width, so a cell is 1 unit wide and [`CELL_ASPECT`] units tall,
//! and the scale is units per image pixel. Fit is the largest scale that shows the whole image
//! and fill the smallest that leaves no letterbox. Zoom multiplies the scale by [`ZOOM_STEP`]
//! until an image pixel covers whole output pixels on both axes, and from there steps by whole
//! output pixels, so every block of a zoomed image is the same size.

use std::num::NonZeroU32;

use charaoke::{BlockMode, CELL_ASPECT, Grid, View};
use serde::{Deserialize, Serialize};

/// How much one zoom step scales the image, below whole output pixels.
pub const ZOOM_STEP: f64 = 1.25;

/// Zooming in stops once the view's longer side is this many image pixels.
pub const MIN_ACROSS: f64 = 16.0;

/// Slack for comparing scales that went through a few multiplications.
const EPS: f64 = 1e-6;

/// What a zoom is relative to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Base {
    /// The whole image, letterboxed.
    #[default]
    Fit,
    /// The image covering the frame, cropped.
    Fill,
}

/// The size of the canvas the art is drawn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameSize {
    /// The art pane of the three-pane layout.
    Pane,
    /// The whole terminal.
    Screen,
    /// A fixed size in cells, at most the terminal's.
    Custom { cols: u16, rows: u16 },
}

/// The presets the frame picker offers, as `(name, cols, rows)`.
pub const PRESETS: [(&str, u16, u16); 2] = [("stamp", 20, 12), ("postcard", 60, 20)];

impl FrameSize {
    /// How the status bar names the frame, such as `stamp 20x12` or `screen`.
    pub fn name(self) -> String {
        match self {
            FrameSize::Pane => "pane".into(),
            FrameSize::Screen => "screen".into(),
            FrameSize::Custom { cols, rows } => {
                match PRESETS.iter().find(|&&(_, c, r)| (c, r) == (cols, rows)) {
                    Some((name, ..)) => format!("{name} {cols}x{rows}"),
                    None => format!("{cols}x{rows}"),
                }
            }
        }
    }
}

/// An image or frame size, as `(width, height)` in pixels or cells.
pub type Size = (NonZeroU32, NonZeroU32);

/// The view the art pane shows: the base, a zoom relative to it, and the center in fractions of
/// the image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArtView {
    pub base: Base,
    /// Relative to the base's scale; below 1 for a fill zoomed out toward fit.
    pub zoom: f64,
    pub center: (f64, f64),
}

impl Default for ArtView {
    fn default() -> Self {
        Self {
            base: Base::Fit,
            zoom: 1.0,
            center: (0.5, 0.5),
        }
    }
}

/// Output pixels across and down a cell in `mode`, or `None` for ASCII.
fn pixels_per_cell(mode: Option<BlockMode>) -> (f64, f64) {
    let (across, down) = mode.map_or((1, 1), BlockMode::pixels_per_cell);
    (f64::from(across), f64::from(down))
}

/// The smallest scale at which an image pixel covers whole output pixels on both axes: one
/// output pixel down and as many across as keeps it square, such as two cells across and one
/// down in ASCII. Zooming past it steps by whole multiples of it.
pub fn whole_pixel_scale(mode: Option<BlockMode>) -> f64 {
    let (across, down) = pixels_per_cell(mode);
    (CELL_ASPECT / down).max(1.0 / across)
}

fn dims(size: Size) -> (f64, f64) {
    (f64::from(size.0.get()), f64::from(size.1.get()))
}

/// The scale of `base` for an `image` on a `frame`.
fn base_scale(base: Base, image: Size, frame: Size) -> f64 {
    let ((w, h), (cols, rows)) = (dims(image), dims(frame));
    let across = cols / w;
    let down = rows * CELL_ASPECT / h;
    match base {
        Base::Fit => across.min(down),
        Base::Fill => across.max(down),
    }
}

/// The largest scale, where the view's longer side is [`MIN_ACROSS`] image pixels.
fn max_scale(image: Size, frame: Size) -> f64 {
    let (cols, rows) = dims(frame);
    (cols.max(rows * CELL_ASPECT) / MIN_ACROSS).max(base_scale(Base::Fit, image, frame))
}

/// The view's size in fractions of the image at `scale`, and how many cells it takes.
fn extent(scale: f64, image: Size, frame: Size) -> ((f64, f64), (u32, u32)) {
    let ((w, h), (cols, rows)) = (dims(image), dims(frame));
    let axis = |pixels: f64, cells: f64, unit: f64| {
        let wanted = pixels * scale / unit;
        if wanted <= cells + EPS {
            // The whole image fits along this axis.
            (1.0, wanted.round().clamp(1.0, cells) as u32)
        } else {
            ((cells * unit / scale / pixels).min(1.0), cells as u32)
        }
    };
    let (vw, used_cols) = axis(w, cols, 1.0);
    let (vh, used_rows) = axis(h, rows, CELL_ASPECT);
    ((vw, vh), (used_cols, used_rows))
}

impl ArtView {
    /// The fill view, centered.
    pub fn fill() -> Self {
        Self {
            base: Base::Fill,
            ..Self::default()
        }
    }

    /// Whether this is today's fit, which draws exactly as the pane always has.
    pub fn is_fit(&self) -> bool {
        self.base == Base::Fit && (self.zoom - 1.0).abs() < EPS
    }

    /// The scale it draws at: between fit and the closest zoom, and snapped to whole output
    /// pixels once past them, so switching modes re-snaps.
    pub fn scale(&self, image: Size, frame: Size, mode: Option<BlockMode>) -> f64 {
        let fit = base_scale(Base::Fit, image, frame);
        let max = max_scale(image, frame);
        let raw = (base_scale(self.base, image, frame) * self.zoom).clamp(fit, max);
        let whole = whole_pixel_scale(mode);
        if raw + EPS < whole {
            return raw;
        }
        let steps = (raw / whole).round().max(1.0);
        let snapped = steps * whole;
        if snapped <= max + EPS {
            snapped
        } else if (steps - 1.0) >= 1.0 {
            (steps - 1.0) * whole
        } else {
            raw
        }
    }

    /// The grid to render on and the view to render, for an `image` on a `frame` in `mode`.
    pub fn layout(&self, image: Size, frame: Size, mode: Option<BlockMode>) -> (Grid, View) {
        if self.is_fit() {
            return (
                Grid::fit_within(image.0, image.1, frame.0, frame.1),
                View::FULL,
            );
        }
        let scale = self.scale(image, frame, mode);
        let ((vw, vh), (cols, rows)) = extent(scale, image, frame);
        let grid = Grid::new(
            NonZeroU32::new(cols).unwrap_or(NonZeroU32::MIN),
            NonZeroU32::new(rows).unwrap_or(NonZeroU32::MIN),
        );
        let view = View::centered(self.center.0, self.center.1, vw, vh).unwrap_or(View::FULL);
        (grid, view)
    }

    /// Keeps the view inside the image, and centered along an axis it already shows whole.
    fn clamp_center(&mut self, image: Size, frame: Size, mode: Option<BlockMode>) {
        let ((vw, vh), _) = extent(self.scale(image, frame, mode), image, frame);
        let clamp = |c: f64, size: f64| {
            if size >= 1.0 - EPS {
                0.5
            } else {
                c.clamp(size / 2.0, 1.0 - size / 2.0)
            }
        };
        self.center = (clamp(self.center.0, vw), clamp(self.center.1, vh));
    }

    /// Zooms by `steps`, in if positive, keeping the image point at `at` where it is. `at` is in
    /// fractions of the art drawn, so `(0.5, 0.5)` zooms about the center. Returns whether the
    /// view changed.
    pub fn zoom_by(
        &mut self,
        steps: i32,
        at: (f64, f64),
        image: Size,
        frame: Size,
        mode: Option<BlockMode>,
    ) -> bool {
        let before = *self;
        let fit = base_scale(Base::Fit, image, frame);
        let max = max_scale(image, frame);
        let whole = whole_pixel_scale(mode);
        let base = base_scale(self.base, image, frame);
        let (_, old) = self.layout(image, frame, mode);
        let mut scale = self.scale(image, frame, mode);
        for _ in 0..steps.unsigned_abs() {
            let previous = scale;
            scale = if steps > 0 {
                if scale + EPS >= whole {
                    ((scale / whole).round() + 1.0) * whole
                } else {
                    (scale * ZOOM_STEP).min(whole)
                }
            } else if scale > whole + EPS {
                ((scale / whole).round() - 1.0) * whole
            } else {
                scale / ZOOM_STEP
            };
            if scale > max + EPS {
                // A whole-pixel step past the limit stays put; a smaller one stops at it.
                scale = if previous + EPS >= whole {
                    previous
                } else {
                    max
                };
            }
            scale = scale.max(fit);
        }
        self.zoom = scale / base;
        if (scale - fit).abs() < EPS {
            // Zoomed all the way out, a fill is a fit too.
            *self = Self::default();
        }
        if self.is_fit() {
            self.center = (0.5, 0.5);
        } else {
            // The image point under `at` before, placed under `at` again.
            let point = (old.x() + at.0 * old.width(), old.y() + at.1 * old.height());
            let ((vw, vh), _) = extent(self.scale(image, frame, mode), image, frame);
            self.center = (
                point.0 + (0.5 - at.0) * vw.min(1.0),
                point.1 + (0.5 - at.1) * vh.min(1.0),
            );
            self.clamp_center(image, frame, mode);
        }
        *self != before
    }

    /// Pans by `cells` across and down, clamped to the image. Returns whether the view moved.
    pub fn pan(
        &mut self,
        cells: (f64, f64),
        image: Size,
        frame: Size,
        mode: Option<BlockMode>,
    ) -> bool {
        let before = self.center;
        let scale = self.scale(image, frame, mode);
        let (w, h) = dims(image);
        self.center.0 += cells.0 / (scale * w);
        self.center.1 += cells.1 * CELL_ASPECT / (scale * h);
        self.clamp_center(image, frame, mode);
        self.center != before
    }

    /// How the status bar names the view, such as `fill` or `fit 2.4x`, or `None` at fit.
    pub fn describe(&self, image: Size, frame: Size, mode: Option<BlockMode>) -> Option<String> {
        if self.is_fit() {
            return None;
        }
        let scale = self.scale(image, frame, mode);
        let zoom = scale / base_scale(self.base, image, frame);
        let base = match self.base {
            Base::Fit => "fit",
            Base::Fill => "fill",
        };
        Some(if (zoom - 1.0).abs() < 0.05 {
            base.to_string()
        } else {
            format!("{base} {zoom:.1}x")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: u32, h: u32) -> Size {
        (NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
    }

    /// La Grande Jatte, as the full image.
    const IMAGE: (u32, u32) = (843, 565);
    const HALF: Option<BlockMode> = Some(BlockMode::Half);

    fn image() -> Size {
        size(IMAGE.0, IMAGE.1)
    }

    #[test]
    fn fit_is_todays_layout() {
        let frame = size(200, 60);
        let (grid, view) = ArtView::default().layout(image(), frame, HALF);
        assert_eq!(
            grid,
            Grid::fit_within(image().0, image().1, frame.0, frame.1)
        );
        assert_eq!(view, View::FULL);
    }

    #[test]
    fn fill_covers_the_whole_frame() {
        let frame = size(200, 60);
        let (grid, view) = ArtView::fill().layout(image(), frame, HALF);
        assert_eq!((grid.cols(), grid.rows()), (200, 60));
        // A landscape image on a wider frame keeps its width and crops its height, centered.
        assert!((view.width() - 1.0).abs() < 1e-9);
        assert!(view.height() < 1.0);
        assert!((view.y() + view.height() / 2.0 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn zooming_in_steps_then_snaps_to_whole_pixels() {
        let (frame, mut view) = (size(200, 60), ArtView::default());
        let mut scales = Vec::new();
        for _ in 0..9 {
            view.zoom_by(1, (0.5, 0.5), image(), frame, HALF);
            scales.push(view.scale(image(), frame, HALF));
        }
        let fit = base_scale(Base::Fit, image(), frame);
        assert!((scales[0] - fit * ZOOM_STEP).abs() < 1e-9);
        // Half blocks are square pixels, so whole pixels are scales 1, 2, 3 and so on.
        let whole: Vec<f64> = scales.iter().copied().filter(|&s| s >= 1.0).collect();
        assert!(whole.len() >= 2, "{scales:?}");
        for (i, s) in whole.iter().enumerate() {
            assert!((s - (i + 1) as f64).abs() < 1e-9, "{scales:?}");
        }
    }

    #[test]
    fn whole_pixels_depend_on_the_mode() {
        assert_eq!(whole_pixel_scale(None), 2.0);
        assert_eq!(whole_pixel_scale(Some(BlockMode::Half)), 1.0);
        assert_eq!(whole_pixel_scale(Some(BlockMode::Quadrant)), 1.0);
        // A zoom between whole pixels snaps when drawn, in whichever mode.
        let frame = size(200, 60);
        let mut view = ArtView {
            zoom: 2.6 / base_scale(Base::Fit, image(), frame),
            ..ArtView::default()
        };
        assert_eq!(view.scale(image(), frame, HALF), 3.0);
        assert_eq!(view.scale(image(), frame, None), 2.0);
        view.zoom_by(1, (0.5, 0.5), image(), frame, None);
        assert_eq!(view.scale(image(), frame, None), 4.0);
    }

    #[test]
    fn zoom_out_stops_at_fit_and_in_at_sixteen_pixels() {
        let (frame, mut view) = (size(200, 60), ArtView::fill());
        for _ in 0..20 {
            view.zoom_by(-1, (0.5, 0.5), image(), frame, HALF);
        }
        let fit = base_scale(Base::Fit, image(), frame);
        assert!((view.scale(image(), frame, HALF) - fit).abs() < 1e-9);
        assert!(!view.zoom_by(-1, (0.5, 0.5), image(), frame, HALF));
        for _ in 0..40 {
            view.zoom_by(1, (0.5, 0.5), image(), frame, HALF);
        }
        let (_, shown) = view.layout(image(), frame, HALF);
        assert!(shown.width() * f64::from(IMAGE.0) >= MIN_ACROSS - 1e-6);
        assert!(!view.zoom_by(1, (0.5, 0.5), image(), frame, HALF));
    }

    #[test]
    fn zooming_back_out_to_fit_is_fit_again() {
        let (frame, mut view) = (size(200, 60), ArtView::default());
        view.zoom_by(3, (0.2, 0.2), image(), frame, HALF);
        assert!(!view.is_fit());
        view.zoom_by(-3, (0.5, 0.5), image(), frame, HALF);
        assert!(view.is_fit(), "{view:?}");
        assert_eq!(view.center, (0.5, 0.5));
    }

    #[test]
    fn zoom_keeps_the_point_under_the_pointer() {
        let (frame, mut view) = (size(200, 60), ArtView::fill());
        view.zoom_by(2, (0.5, 0.5), image(), frame, HALF);
        let (_, before) = view.layout(image(), frame, HALF);
        let at = (0.3, 0.6);
        let point = (
            before.x() + at.0 * before.width(),
            before.y() + at.1 * before.height(),
        );
        view.zoom_by(1, at, image(), frame, HALF);
        let (_, after) = view.layout(image(), frame, HALF);
        let moved = (
            after.x() + at.0 * after.width(),
            after.y() + at.1 * after.height(),
        );
        assert!((moved.0 - point.0).abs() < 1e-9 && (moved.1 - point.1).abs() < 1e-9);
    }

    #[test]
    fn a_pan_moves_one_cell_and_stops_at_the_edge() {
        let (frame, mut view) = (size(200, 60), ArtView::default());
        view.zoom_by(8, (0.5, 0.5), image(), frame, HALF);
        let scale = view.scale(image(), frame, HALF);
        let start = view.center;
        assert!(view.pan((1.0, 0.0), image(), frame, HALF));
        assert!((view.center.0 - start.0 - 1.0 / (scale * 843.0)).abs() < 1e-12);
        for _ in 0..10_000 {
            view.pan((-1.0, -1.0), image(), frame, HALF);
        }
        let (_, shown) = view.layout(image(), frame, HALF);
        assert_eq!((shown.x(), shown.y()), (0.0, 0.0));
        assert!(!view.pan((-1.0, -1.0), image(), frame, HALF));
    }

    #[test]
    fn fit_pans_nowhere() {
        let (frame, mut view) = (size(200, 60), ArtView::default());
        assert!(!view.pan((5.0, 5.0), image(), frame, HALF));
        // A fill of a landscape image on a wide frame shows its whole width, so only pans down.
        let mut fill = ArtView::fill();
        assert!(!fill.pan((5.0, 0.0), image(), frame, HALF));
        assert!(fill.pan((0.0, 1.0), image(), frame, HALF));
    }

    #[test]
    fn describes_the_view_against_its_base() {
        let frame = size(200, 60);
        let mode = HALF;
        assert_eq!(ArtView::default().describe(image(), frame, mode), None);
        assert_eq!(
            ArtView::fill().describe(image(), frame, mode).as_deref(),
            Some("fill")
        );
        let mut view = ArtView::default();
        view.zoom_by(1, (0.5, 0.5), image(), frame, mode);
        assert_eq!(
            view.describe(image(), frame, mode).as_deref(),
            Some("fit 1.2x")
        );
    }

    #[test]
    fn frames_name_their_presets() {
        assert_eq!(
            FrameSize::Custom { cols: 20, rows: 12 }.name(),
            "stamp 20x12"
        );
        assert_eq!(FrameSize::Custom { cols: 30, rows: 9 }.name(), "30x9");
        assert_eq!(FrameSize::Screen.name(), "screen");
    }
}
