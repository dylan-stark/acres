use std::num::NonZeroU32;

use crate::{Error, Grid, Ramp};

/// Widest image a converter accepts: a row segment's sum must fit in a `u32`.
const MAX_WIDTH: u32 = u32::MAX / 255;

/// Layout of the pixels passed to [`Converter::set_image`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PixelFormat {
    /// One byte of brightness per pixel.
    Luma8,
    /// Three bytes per pixel: red, green, blue.
    Rgb8,
    /// Four bytes per pixel: red, green, blue, alpha (alpha is ignored).
    Rgba8,
}

impl PixelFormat {
    fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Luma8 => 1,
            PixelFormat::Rgb8 => 3,
            PixelFormat::Rgba8 => 4,
        }
    }
}

/// Converts an image to ASCII, reusing its buffers from one render to the next.
///
/// Loading an image ([`set_image`](Self::set_image)) computes its brightness once. Each
/// [`render`](Self::render) then only downsamples that brightness onto a [`Grid`] and looks up
/// characters, so a terminal resize costs one render and no allocation once the buffers have
/// grown to fit.
///
/// ```rust
/// use std::num::NonZeroU32;
/// use charaoke::{Converter, Grid, PixelFormat};
///
/// let nz = |n| NonZeroU32::new(n).unwrap();
/// // A 4x2 image: black on the left half, white on the right.
/// let pixels = [0, 0, 255, 255, 0, 0, 255, 255];
///
/// let mut converter = Converter::default();
/// converter.set_image(&pixels, nz(4), nz(2), PixelFormat::Luma8).unwrap();
///
/// let mut out = String::new();
/// converter.render(Grid::fit(nz(4), nz(2), nz(4)), &mut out);
/// assert_eq!(out, "  @@");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Converter {
    ramp: Ramp,
    luma: Vec<u8>,
    width: usize,
    height: usize,
    sums: Vec<u64>,
    col_bounds: Vec<usize>,
}

impl Converter {
    /// A converter that draws with `ramp`.
    pub fn new(ramp: Ramp) -> Self {
        Self {
            ramp,
            ..Self::default()
        }
    }

    /// The ramp this converter draws with.
    pub fn ramp(&self) -> &Ramp {
        &self.ramp
    }

    /// Loads an image of `width` by `height` pixels, stored row by row with no padding.
    ///
    /// Colour pixels are reduced to brightness with integer BT.601 weights
    /// (`(77 R + 150 G + 29 B) / 256`).
    pub fn set_image(
        &mut self,
        pixels: &[u8],
        width: NonZeroU32,
        height: NonZeroU32,
        format: PixelFormat,
    ) -> Result<(), Error> {
        if width.get() > MAX_WIDTH {
            return Err(Error::ImageTooWide(width.get()));
        }
        let (w, h) = (width.get() as usize, height.get() as usize);
        let expected = w
            .checked_mul(h)
            .and_then(|n| n.checked_mul(format.bytes_per_pixel()))
            .unwrap_or(usize::MAX);
        if pixels.len() != expected {
            return Err(Error::PixelCountMismatch {
                expected,
                actual: pixels.len(),
            });
        }

        self.luma.clear();
        match format {
            PixelFormat::Luma8 => self.luma.extend_from_slice(pixels),
            PixelFormat::Rgb8 => self
                .luma
                .extend(pixels.as_chunks::<3>().0.iter().map(bt601)),
            PixelFormat::Rgba8 => self
                .luma
                .extend(pixels.as_chunks::<4>().0.iter().map(bt601)),
        }
        self.width = w;
        self.height = h;
        Ok(())
    }

    /// Renders the loaded image onto `grid`, replacing the contents of `out`.
    ///
    /// Each cell takes the average brightness of the pixels it covers, and rows are separated by
    /// `\n` with none after the last. When the grid is finer than the image, neighbouring cells
    /// share pixels. With no image loaded, `out` is left empty.
    pub fn render(&mut self, grid: Grid, out: &mut String) {
        out.clear();
        if self.luma.is_empty() {
            return;
        }
        let (cols, rows) = (grid.cols() as usize, grid.rows() as usize);
        let (w, h) = (self.width, self.height);

        // Pixel column bounds of each cell column, shared by every row.
        self.col_bounds.clear();
        self.col_bounds.extend((0..=cols).map(|c| c * w / cols));
        self.sums.clear();
        self.sums.resize(cols, 0);
        out.reserve(rows * (cols + 1));

        for r in 0..rows {
            let y0 = r * h / rows;
            let y1 = ((r + 1) * h / rows).max(y0 + 1).min(h);
            let y0 = y0.min(y1 - 1);

            self.sums.fill(0);
            for row in self.luma[y0 * w..y1 * w].chunks_exact(w) {
                for (c, sum) in self.sums.iter_mut().enumerate() {
                    let (x0, x1) = cell_span(&self.col_bounds, c, w);
                    *sum += u64::from(row[x0..x1].iter().map(|&v| u32::from(v)).sum::<u32>());
                }
            }

            let band = (y1 - y0) as u64;
            for (c, &sum) in self.sums.iter().enumerate() {
                let (x0, x1) = cell_span(&self.col_bounds, c, w);
                let count = band * (x1 - x0) as u64;
                let luma = ((sum + count / 2) / count) as u8;
                out.push(char::from(self.ramp.lookup(luma)));
            }
            if r + 1 < rows {
                out.push('\n');
            }
        }
    }
}

/// Pixel span `[x0, x1)` of cell column `c`, at least one pixel wide.
#[inline]
fn cell_span(bounds: &[usize], c: usize, width: usize) -> (usize, usize) {
    let x1 = bounds[c + 1].max(bounds[c] + 1).min(width);
    (bounds[c].min(x1 - 1), x1)
}

#[inline]
fn bt601<const N: usize>(px: &[u8; N]) -> u8 {
    ((77 * u32::from(px[0]) + 150 * u32::from(px[1]) + 29 * u32::from(px[2])) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    fn render(pixels: &[u8], w: u32, h: u32, format: PixelFormat, grid: Grid) -> String {
        let mut converter = Converter::default();
        converter.set_image(pixels, nz(w), nz(h), format).unwrap();
        let mut out = String::new();
        converter.render(grid, &mut out);
        out
    }

    fn grid(cols: u32, rows: u32) -> Grid {
        // Grid::fit halves rows, so ask for an image twice as tall as the wanted rows.
        Grid::fit(nz(cols), nz(rows * 2), nz(cols))
    }

    #[test]
    fn averages_each_cell() {
        // 4x2 image, two cells of 2x2: the left averages 0 and 255 (128), the right is 255.
        let pixels = [0, 255, 255, 255, 255, 0, 255, 255];
        let ramp = Ramp::default();
        let out = render(&pixels, 4, 2, PixelFormat::Luma8, grid(2, 1));
        assert_eq!(out.as_bytes(), [ramp.lookup(128), b'@']);
    }

    #[test]
    fn separates_rows_with_newlines() {
        let pixels = [0, 0, 255, 255];
        let out = render(&pixels, 2, 2, PixelFormat::Luma8, grid(2, 2));
        assert_eq!(out, "  \n@@");
    }

    #[test]
    fn horizontal_gradient_brightens_left_to_right() {
        // Ten flat bands, 0 to 255 in even steps, each six pixels wide.
        let w = 60;
        let pixels: Vec<u8> = (0..w * 8).map(|i| ((i % w) / 6 * 255 / 9) as u8).collect();
        let out = render(&pixels, w, 8, PixelFormat::Luma8, grid(10, 1));
        assert_eq!(out, Ramp::DEFAULT);
    }

    #[test]
    fn upsamples_when_grid_is_finer_than_image() {
        let pixels = [0, 255];
        let out = render(&pixels, 2, 1, PixelFormat::Luma8, grid(4, 2));
        assert_eq!(out, "  @@\n  @@");
    }

    #[test]
    fn colour_formats_use_bt601_weights() {
        let ramp = Ramp::default();
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255];
        let rgba = [255, 0, 0, 9, 0, 255, 0, 9, 0, 0, 255, 9];
        let expected: Vec<u8> = [76, 149, 28].into_iter().map(|l| ramp.lookup(l)).collect();
        assert_eq!(
            render(&rgb, 3, 1, PixelFormat::Rgb8, grid(3, 1)).as_bytes(),
            expected
        );
        assert_eq!(
            render(&rgba, 3, 1, PixelFormat::Rgba8, grid(3, 1)).as_bytes(),
            expected
        );
    }

    #[test]
    fn render_is_repeatable_and_reuses_output() {
        let pixels: Vec<u8> = (0..=255).collect();
        let mut converter = Converter::default();
        converter
            .set_image(&pixels, nz(16), nz(16), PixelFormat::Luma8)
            .unwrap();
        let mut out = String::from("stale");
        converter.render(grid(8, 4), &mut out);
        let first = out.clone();
        converter.render(grid(8, 4), &mut out);
        assert_eq!(out, first);
        assert_eq!(out.lines().count(), 4);
        assert!(out.lines().all(|line| line.len() == 8));
    }

    #[test]
    fn renders_nothing_without_an_image() {
        let mut out = String::from("stale");
        Converter::default().render(grid(4, 2), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn rejects_mismatched_pixel_buffers() {
        let mut converter = Converter::default();
        let err = converter
            .set_image(&[0; 5], nz(2), nz(1), PixelFormat::Rgb8)
            .unwrap_err();
        assert_eq!(
            err,
            Error::PixelCountMismatch {
                expected: 6,
                actual: 5
            }
        );
    }
}
