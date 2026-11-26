use std::num::NonZeroU32;

/// Height-to-width ratio of a terminal character cell.
///
/// Most terminal fonts draw cells about twice as tall as they are wide, so an image needs half as
/// many rows as its pixel aspect ratio alone would suggest.
pub const CELL_ASPECT: f64 = 2.0;

/// The character grid an image is converted onto.
///
/// Every cell covers the same block of source pixels, so the grid also fixes how the image is
/// downsampled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Grid {
    cols: NonZeroU32,
    rows: NonZeroU32,
}

impl Grid {
    /// Fits an image of `width` by `height` pixels onto a grid `cols` characters wide.
    ///
    /// The row count keeps the image's aspect ratio, corrected by [`CELL_ASPECT`], and is never
    /// less than one.
    pub fn fit(width: NonZeroU32, height: NonZeroU32, cols: NonZeroU32) -> Self {
        let rows = (f64::from(cols.get()) * f64::from(height.get())
            / (f64::from(width.get()) * CELL_ASPECT))
            .round()
            .clamp(1.0, f64::from(u32::MAX)) as u32;
        Self {
            cols,
            rows: NonZeroU32::new(rows).unwrap_or(NonZeroU32::MIN),
        }
    }

    /// Fits an image of `width` by `height` pixels onto the largest grid that fits in `max_cols`
    /// by `max_rows`, as for a terminal pane.
    ///
    /// The grid keeps the image's aspect ratio, corrected by [`CELL_ASPECT`], and never exceeds
    /// either bound. A wide image fills the columns; a tall one fills the rows.
    ///
    /// ```rust
    /// use std::num::NonZeroU32;
    /// use charaoke::Grid;
    ///
    /// let nz = |n| NonZeroU32::new(n).unwrap();
    /// // A square image in a wide pane is limited by the pane's height.
    /// let grid = Grid::fit_within(nz(100), nz(100), nz(200), nz(40));
    /// assert_eq!((grid.cols(), grid.rows()), (80, 40));
    /// ```
    pub fn fit_within(
        width: NonZeroU32,
        height: NonZeroU32,
        max_cols: NonZeroU32,
        max_rows: NonZeroU32,
    ) -> Self {
        let grid = Self::fit(width, height, max_cols);
        if grid.rows <= max_rows {
            return grid;
        }
        let cols = (f64::from(max_rows.get()) * f64::from(width.get()) * CELL_ASPECT
            / f64::from(height.get()))
        .floor()
        .clamp(1.0, f64::from(max_cols.get())) as u32;
        let cols = NonZeroU32::new(cols).unwrap_or(NonZeroU32::MIN);
        let grid = Self::fit(width, height, cols);
        Self {
            cols,
            rows: grid.rows.min(max_rows),
        }
    }

    /// A grid of exactly `cols` by `rows` cells, for drawing a [`View`](crate::View) onto a
    /// frame whose size is already known, such as a whole terminal pane.
    ///
    /// ```rust
    /// use std::num::NonZeroU32;
    /// use charaoke::Grid;
    ///
    /// let nz = |n| NonZeroU32::new(n).unwrap();
    /// let grid = Grid::new(nz(20), nz(12));
    /// assert_eq!((grid.cols(), grid.rows()), (20, 12));
    /// ```
    pub fn new(cols: NonZeroU32, rows: NonZeroU32) -> Self {
        Self { cols, rows }
    }

    /// Number of characters per row.
    pub fn cols(&self) -> u32 {
        self.cols.get()
    }

    /// Number of rows.
    pub fn rows(&self) -> u32 {
        self.rows.get()
    }

    /// Total number of cells.
    pub fn cells(&self) -> u64 {
        u64::from(self.cols()) * u64::from(self.rows())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    #[test]
    fn square_image_gets_half_as_many_rows() {
        let grid = Grid::fit(nz(100), nz(100), nz(80));
        assert_eq!((grid.cols(), grid.rows()), (80, 40));
    }

    #[test]
    fn keeps_aspect_ratio() {
        let grid = Grid::fit(nz(320), nz(240), nz(160));
        assert_eq!((grid.cols(), grid.rows()), (160, 60));
    }

    #[test]
    fn very_wide_image_still_has_one_row() {
        let grid = Grid::fit(nz(10_000), nz(1), nz(80));
        assert_eq!(grid.rows(), 1);
    }

    #[test]
    fn very_tall_image_does_not_overflow() {
        let grid = Grid::fit(nz(1), nz(u32::MAX), nz(u32::MAX));
        assert_eq!(grid.rows(), u32::MAX);
    }

    #[test]
    fn fit_within_fills_the_limiting_side() {
        // Wide image, tall pane: limited by columns.
        let grid = Grid::fit_within(nz(200), nz(100), nz(80), nz(100));
        assert_eq!((grid.cols(), grid.rows()), (80, 20));
        // Tall image, wide pane: limited by rows.
        let grid = Grid::fit_within(nz(100), nz(400), nz(200), nz(50));
        assert_eq!((grid.cols(), grid.rows()), (25, 50));
    }

    #[test]
    fn fit_within_never_exceeds_either_bound() {
        for w in [1, 7, 100, 843, 5000] {
            for h in [1, 3, 100, 632, 5000] {
                for max_cols in [1, 13, 80, 200] {
                    for max_rows in [1, 9, 48, 80] {
                        let grid = Grid::fit_within(nz(w), nz(h), nz(max_cols), nz(max_rows));
                        assert!(grid.cols() <= max_cols, "{w}x{h} in {max_cols}x{max_rows}");
                        assert!(grid.rows() <= max_rows, "{w}x{h} in {max_cols}x{max_rows}");
                    }
                }
            }
        }
    }

    #[test]
    fn counts_cells() {
        let grid = Grid::fit(nz(320), nz(240), nz(160));
        assert_eq!(grid.cells(), 9_600);
    }
}
