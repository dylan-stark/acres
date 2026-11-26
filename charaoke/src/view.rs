use std::num::NonZeroU32;

use crate::{CELL_ASPECT, Error, Grid};

/// The part of an image to render: a rectangle in fractions of the image's width and height.
///
/// [`View::FULL`] is the whole image, which is what [`Converter::render`](crate::Converter::render)
/// draws. A smaller view crops and zooms: rendered onto the same grid, half the width and height
/// shows a quarter of the image at twice the scale. Once a view is narrower than its grid, image
/// pixels repeat across neighboring cells and draw as flat blocks (see ADR 0017).
///
/// ```rust
/// use charaoke::View;
///
/// // The middle of the image, at twice the scale.
/// let view = View::centered(0.5, 0.5, 0.5, 0.5).unwrap();
/// assert_eq!((view.x(), view.y(), view.width(), view.height()), (0.25, 0.25, 0.5, 0.5));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Default for View {
    fn default() -> Self {
        Self::FULL
    }
}

impl View {
    /// The whole image.
    pub const FULL: View = View {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    };

    /// The view whose top-left corner is at `x`, `y` and which is `width` by `height`, all in
    /// fractions of the image.
    ///
    /// Fails unless every value is finite, the size is positive, and the view lies within the
    /// image.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Result<Self, Error> {
        // Sums of fractions can land a hair past 1.
        const SLACK: f64 = 1e-9;
        let finite = [x, y, width, height].iter().all(|v| v.is_finite());
        if !finite
            || width <= 0.0
            || height <= 0.0
            || x < 0.0
            || y < 0.0
            || x + width > 1.0 + SLACK
            || y + height > 1.0 + SLACK
        {
            return Err(Error::InvalidView {
                x,
                y,
                width,
                height,
            });
        }
        Ok(Self {
            x,
            y,
            width: width.min(1.0 - x),
            height: height.min(1.0 - y),
        })
    }

    /// The view `width` by `height` centered as near `cx`, `cy` as it can be while staying within
    /// the image, all in fractions of the image.
    ///
    /// A size larger than the image is cut to the whole of that axis. Fails unless every value is
    /// finite and the size is positive.
    pub fn centered(cx: f64, cy: f64, width: f64, height: f64) -> Result<Self, Error> {
        let invalid = Error::InvalidView {
            x: cx,
            y: cy,
            width,
            height,
        };
        if ![cx, cy, width, height].iter().all(|v| v.is_finite()) || width <= 0.0 || height <= 0.0 {
            return Err(invalid);
        }
        let place = |center: f64, size: f64| {
            let size = size.min(1.0);
            ((center - size / 2.0).clamp(0.0, 1.0 - size), size)
        };
        let (x, width) = place(cx, width);
        let (y, height) = place(cy, height);
        Self::new(x, y, width, height).map_err(|_| invalid)
    }

    /// The largest view of a `width` by `height` image that covers `grid` with no letterbox,
    /// centered: the whole image along one axis, cropped along the other.
    ///
    /// ```rust
    /// use std::num::NonZeroU32;
    /// use charaoke::{Grid, View};
    ///
    /// let nz = |n| NonZeroU32::new(n).unwrap();
    /// // A square image on a grid of square proportions (40 columns by 20 rows of tall cells)
    /// // needs no crop.
    /// let grid = Grid::fit(nz(100), nz(100), nz(40));
    /// assert_eq!(View::fill(nz(100), nz(100), grid), View::FULL);
    /// // On a grid twice as wide, it keeps the full width and the middle half of the height.
    /// let wide = Grid::fit(nz(200), nz(100), nz(80));
    /// let view = View::fill(nz(100), nz(100), wide);
    /// assert_eq!((view.y(), view.height()), (0.25, 0.5));
    /// ```
    pub fn fill(width: NonZeroU32, height: NonZeroU32, grid: Grid) -> Self {
        let image = f64::from(width.get()) / f64::from(height.get());
        let target = f64::from(grid.cols()) / (f64::from(grid.rows()) * CELL_ASPECT);
        let (w, h) = if image > target {
            (target / image, 1.0)
        } else {
            (1.0, image / target)
        };
        Self::centered(0.5, 0.5, w, h).unwrap_or(Self::FULL)
    }

    /// Left edge, in fractions of the image's width.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Top edge, in fractions of the image's height.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Width, in fractions of the image's width.
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Height, in fractions of the image's height.
    pub fn height(&self) -> f64 {
        self.height
    }

    /// The view in pixels of a `w` by `h` image: left, top, width and height.
    pub(crate) fn window(&self, w: usize, h: usize) -> Window {
        let (w, h) = (w as f64, h as f64);
        Window {
            x: self.x * w,
            y: self.y * h,
            width: self.width * w,
            height: self.height * h,
        }
    }
}

/// A view in image pixels, which may fall between pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Window {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl Window {
    /// The pixel bound `i` of `n` steps across `start..start + span`, rounded down.
    ///
    /// With `start` 0 and `span` a whole number of pixels this equals `i * span / n` in integers,
    /// so the full view renders exactly as before views existed.
    #[inline]
    pub(crate) fn bound(start: f64, span: f64, i: usize, n: usize) -> usize {
        (start + i as f64 * span / n as f64).floor() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn new_rejects_views_outside_the_image() {
        assert!(View::new(0.0, 0.0, 1.0, 1.0).is_ok());
        assert!(View::new(0.5, 0.0, 0.6, 1.0).is_err());
        assert!(View::new(-0.1, 0.0, 0.5, 0.5).is_err());
        assert!(View::new(0.0, 0.0, 0.0, 0.5).is_err());
        assert!(View::new(f64::NAN, 0.0, 0.5, 0.5).is_err());
    }

    #[test]
    fn centered_clamps_to_the_edges() {
        let view = View::centered(0.0, 1.0, 0.5, 0.5).unwrap();
        assert_eq!((view.x(), view.y()), (0.0, 0.5));
        let view = View::centered(0.5, 0.5, 2.0, 0.5).unwrap();
        assert_eq!((view.x(), view.width()), (0.0, 1.0));
        assert!(View::centered(0.5, 0.5, -1.0, 0.5).is_err());
    }

    #[test]
    fn fill_crops_a_wide_image_on_a_tall_grid() {
        // 200x100 image on a 20x20 grid of tall cells: the grid is 1:2, the image 2:1.
        let grid = Grid::fit(nz(100), nz(200), nz(20));
        assert_eq!((grid.cols(), grid.rows()), (20, 20));
        let view = View::fill(nz(200), nz(100), grid);
        assert_eq!(view.height(), 1.0);
        assert!((view.width() - 0.25).abs() < 1e-12);
        assert!((view.x() - 0.375).abs() < 1e-12);
    }

    #[test]
    fn integer_bounds_match_integer_division() {
        for (span, n) in [(843usize, 80usize), (843, 160), (632, 37), (7, 3), (3, 7)] {
            for i in 0..=n {
                assert_eq!(Window::bound(0.0, span as f64, i, n), i * span / n);
            }
        }
    }
}
