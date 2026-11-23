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
    fn counts_cells() {
        let grid = Grid::fit(nz(320), nz(240), nz(160));
        assert_eq!(grid.cells(), 9_600);
    }
}
