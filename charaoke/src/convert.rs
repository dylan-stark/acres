use std::num::NonZeroU32;

use crate::{Blocks, Cells, Error, Grid, Ramp};

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

/// Converts an image to colored ASCII, reusing its buffers from one render to the next.
///
/// Loading an image ([`set_image`](Self::set_image)) keeps its RGB pixels. Each
/// [`render`](Self::render) then averages those pixels onto a [`Grid`], giving every cell a mean
/// color and a character picked by that color's brightness. A terminal resize costs one render and
/// no allocation once the buffers have grown to fit.
///
/// ```rust
/// use std::num::NonZeroU32;
/// use charaoke::{Cells, Converter, Grid, PixelFormat};
///
/// let nz = |n| NonZeroU32::new(n).unwrap();
/// // A 4x2 image: black on the left half, white on the right.
/// let pixels = [0, 0, 255, 255, 0, 0, 255, 255];
///
/// let mut converter = Converter::default();
/// converter.set_image(&pixels, nz(4), nz(2), PixelFormat::Luma8).unwrap();
///
/// let mut cells = Cells::default();
/// converter.render(Grid::fit(nz(4), nz(2), nz(4)), &mut cells);
/// assert_eq!(cells.chars(), b"  @@");
/// assert_eq!(cells.colors()[3], [255, 255, 255]);
/// ```
#[derive(Clone, Debug, Default)]
pub struct Converter {
    ramp: Ramp,
    rgb: Vec<u8>,
    width: usize,
    height: usize,
    scratch: Scratch,
    /// The pending top row of pixels in [`render_blocks`](Self::render_blocks).
    top: Vec<[u8; 3]>,
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

    /// Switches to `ramp` for later renders, keeping the loaded image.
    pub fn set_ramp(&mut self, ramp: Ramp) {
        self.ramp = ramp;
    }

    /// Loads an image of `width` by `height` pixels, stored row by row with no padding.
    ///
    /// The pixels are kept as RGB, so grayscale input takes three bytes per pixel once loaded.
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

        self.rgb.clear();
        match format {
            PixelFormat::Luma8 => self.rgb.extend(pixels.iter().flat_map(|&v| [v, v, v])),
            PixelFormat::Rgb8 => self.rgb.extend_from_slice(pixels),
            PixelFormat::Rgba8 => self.rgb.extend(
                pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|&[r, g, b, _]| [r, g, b]),
            ),
        }
        self.width = w;
        self.height = h;
        Ok(())
    }

    /// Renders the loaded image onto `grid`, replacing the contents of `cells`.
    ///
    /// Each cell takes the mean color of the pixels it covers, and its character comes from that
    /// color's brightness, with integer BT.601 weights (`(77 R + 150 G + 29 B) / 256`). When the
    /// grid is finer than the image, neighboring cells share pixels. With no image loaded, `cells`
    /// is left empty.
    pub fn render(&mut self, grid: Grid, cells: &mut Cells) {
        cells.clear();
        if self.rgb.is_empty() {
            return;
        }
        let (cols, rows) = (grid.cols() as usize, grid.rows() as usize);
        cells.reserve(cols, rows);
        let ramp = &self.ramp;
        self.scratch
            .mean_rows(&self.rgb, self.width, self.height, cols, rows, |means| {
                for &mean in means {
                    cells.push(ramp.lookup(bt601(&mean)), mean);
                }
            });
    }

    /// Renders the loaded image onto `grid` as block glyphs, replacing the contents of `blocks`.
    ///
    /// Each cell is split into pixels ([`BlockMode::pixels_per_cell`]), and each pixel takes the
    /// mean color of the image pixels it covers, as in [`render`](Self::render). Half blocks draw
    /// `▀` with the top pixel as foreground and the bottom one as background. Quadrant blocks try
    /// the 7 ways to split a cell's 4 pixels into two groups and keep the split whose group means
    /// are closest to the pixels, by squared RGB distance. With no image loaded, `blocks` is left
    /// empty.
    ///
    /// ```rust
    /// use std::num::NonZeroU32;
    /// use charaoke::{BlockMode, Blocks, Converter, Grid, PixelFormat};
    ///
    /// let nz = |n| NonZeroU32::new(n).unwrap();
    /// // A 2x2 image: black on the left, white on the right.
    /// let pixels = [0, 255, 0, 255];
    ///
    /// let mut converter = Converter::default();
    /// converter.set_image(&pixels, nz(2), nz(2), PixelFormat::Luma8).unwrap();
    /// let mut blocks = Blocks::default();
    /// converter.render_blocks(Grid::fit(nz(2), nz(2), nz(1)), BlockMode::Quadrant, &mut blocks);
    /// assert_eq!(blocks.glyphs().collect::<String>(), "▌");
    /// assert_eq!((blocks.foreground()[0], blocks.background()[0]), ([0; 3], [255; 3]));
    /// ```
    pub fn render_blocks(&mut self, grid: Grid, mode: BlockMode, blocks: &mut Blocks) {
        blocks.clear();
        if self.rgb.is_empty() {
            return;
        }
        let (cols, rows) = (grid.cols() as usize, grid.rows() as usize);
        let (px_w, px_h) = mode.pixels_per_cell();
        let (px_w, px_h) = (px_w as usize, px_h as usize);
        blocks.reserve(cols, rows);

        // Two rows of pixels make one row of cells, so keep the top row until its pair arrives.
        let mut top = std::mem::take(&mut self.top);
        top.clear();
        self.scratch.mean_rows(
            &self.rgb,
            self.width,
            self.height,
            cols * px_w,
            rows * px_h,
            |means| {
                if top.is_empty() {
                    top.extend_from_slice(means);
                    return;
                }
                match mode {
                    BlockMode::Half => {
                        for (&t, &b) in top.iter().zip(means) {
                            blocks.push(HALF_MASK, t, b);
                        }
                    }
                    BlockMode::Quadrant => {
                        let tops = top.as_chunks::<2>().0;
                        let bottoms = means.as_chunks::<2>().0;
                        for (&[tl, tr], &[bl, br]) in tops.iter().zip(bottoms) {
                            let (mask, fg, bg) = split_quadrants([tl, tr, bl, br]);
                            blocks.push(mask, fg, bg);
                        }
                    }
                }
                top.clear();
            },
        );
        self.top = top;
    }
}

/// How [`Converter::render_blocks`] splits each cell into pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BlockMode {
    /// `▀` in two colors: one pixel on top of another, each about square.
    #[default]
    Half,
    /// One of the quadrant glyphs in two colors: a 2x2 grid of pixels, each twice as tall as it is
    /// wide.
    Quadrant,
}

impl BlockMode {
    /// Pixels across and down each cell.
    pub fn pixels_per_cell(self) -> (u32, u32) {
        match self {
            BlockMode::Half => (1, 2),
            BlockMode::Quadrant => (2, 2),
        }
    }
}

/// Quadrant mask of `▀`: the top-left and top-right quarters.
const HALF_MASK: u8 = 0b0011;

/// Splits to try, as masks of the foreground group (bit 0 top left, 1 top right, 2 bottom left,
/// 3 bottom right). Every split of four pixels into two groups appears once, with the bottom-right
/// pixel in the background group. `▀` comes first, so a flat cell draws as a half block.
const SPLITS: [u8; 7] = [HALF_MASK, 0b0101, 0b0001, 0b0010, 0b0100, 0b0110, 0b0111];

/// The split of four pixels into two groups whose means are closest to the pixels, as the
/// foreground mask, the foreground mean and the background mean.
///
/// A group's squared error is the sum of its squares less `sum² / n`, and the sum of squares is the
/// same for every split, so the best split has the largest `sum² / n` over both groups. That is
/// compared as `6 sum² / n`, which stays an integer for groups of 1 to 3 pixels.
#[inline]
fn split_quadrants(px: [[u8; 3]; 4]) -> (u8, [u8; 3], [u8; 3]) {
    // Sums of each channel over every subset of the first three pixels, indexed by mask.
    let sums: [[u32; 3]; 8] = std::array::from_fn(|mask| {
        std::array::from_fn(|k| {
            (0..3)
                .filter(|&q| mask >> q & 1 == 1)
                .map(|q| u32::from(px[q][k]))
                .sum()
        })
    });
    let total: [u32; 3] = std::array::from_fn(|k| sums[7][k] + u32::from(px[3][k]));
    let score = |mask: u8| -> u32 {
        let n = mask.count_ones();
        let (wf, wb) = (6 / n, 6 / (4 - n));
        let fg = sums[usize::from(mask)];
        (0..3)
            .map(|k| {
                let bg = total[k] - fg[k];
                wf * fg[k] * fg[k] + wb * bg * bg
            })
            .sum()
    };
    let mut best = (score(SPLITS[0]), SPLITS[0]);
    for &mask in &SPLITS[1..] {
        let s = score(mask);
        if s > best.0 {
            best = (s, mask);
        }
    }
    let mask = best.1;
    let fg = sums[usize::from(mask)];
    let n = mask.count_ones();
    let mean = |sum: u32, n: u32| ((sum + n / 2) / n) as u8;
    (
        mask,
        std::array::from_fn(|k| mean(fg[k], n)),
        std::array::from_fn(|k| mean(total[k] - fg[k], 4 - n)),
    )
}

/// Buffers for averaging an image onto a grid, reused from one render to the next.
#[derive(Clone, Debug, Default)]
struct Scratch {
    sums: Vec<[u64; 3]>,
    col_bounds: Vec<usize>,
    means: Vec<[u8; 3]>,
}

impl Scratch {
    /// Averages a `width` by `height` RGB image onto `cols` by `rows` cells, passing each row of
    /// mean colors to `emit`, top to bottom.
    fn mean_rows(
        &mut self,
        rgb: &[u8],
        w: usize,
        h: usize,
        cols: usize,
        rows: usize,
        mut emit: impl FnMut(&[[u8; 3]]),
    ) {
        // Pixel column bounds of each cell column, shared by every row.
        self.col_bounds.clear();
        self.col_bounds.extend((0..=cols).map(|c| c * w / cols));
        self.sums.clear();
        self.sums.resize(cols, [0; 3]);

        for r in 0..rows {
            let y0 = r * h / rows;
            let y1 = ((r + 1) * h / rows).max(y0 + 1).min(h);
            let y0 = y0.min(y1 - 1);

            self.sums.fill([0; 3]);
            for row in rgb[y0 * w * 3..y1 * w * 3].chunks_exact(w * 3) {
                let row = row.as_chunks::<3>().0;
                for (c, sum) in self.sums.iter_mut().enumerate() {
                    let (x0, x1) = cell_span(&self.col_bounds, c, w);
                    let [mut r, mut g, mut b] = [0u32; 3];
                    for px in &row[x0..x1] {
                        r += u32::from(px[0]);
                        g += u32::from(px[1]);
                        b += u32::from(px[2]);
                    }
                    sum[0] += u64::from(r);
                    sum[1] += u64::from(g);
                    sum[2] += u64::from(b);
                }
            }

            let band = (y1 - y0) as u64;
            self.means.clear();
            self.means
                .extend(self.sums.iter().enumerate().map(|(c, sum)| {
                    let (x0, x1) = cell_span(&self.col_bounds, c, w);
                    let count = band * (x1 - x0) as u64;
                    sum.map(|s| ((s + count / 2) / count) as u8)
                }));
            emit(&self.means);
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
fn bt601(px: &[u8; 3]) -> u8 {
    ((77 * u32::from(px[0]) + 150 * u32::from(px[1]) + 29 * u32::from(px[2])) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    fn render(pixels: &[u8], w: u32, h: u32, format: PixelFormat, grid: Grid) -> Cells {
        let mut converter = Converter::default();
        converter.set_image(pixels, nz(w), nz(h), format).unwrap();
        let mut cells = Cells::default();
        converter.render(grid, &mut cells);
        cells
    }

    fn plain(cells: &Cells) -> String {
        let mut out = String::new();
        cells.write_plain(&mut out);
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
        let cells = render(&pixels, 4, 2, PixelFormat::Luma8, grid(2, 1));
        assert_eq!(cells.chars(), [ramp.lookup(128), b'@']);
        assert_eq!(cells.colors(), [[128; 3], [255; 3]]);
    }

    #[test]
    fn averages_each_channel_separately() {
        // One cell over a red and a blue pixel.
        let pixels = [255, 0, 0, 0, 0, 255];
        let cells = render(&pixels, 2, 1, PixelFormat::Rgb8, grid(1, 1));
        assert_eq!(cells.colors(), [[128, 0, 128]]);
        assert_eq!(
            cells.chars(),
            [Ramp::default().lookup(bt601(&[128, 0, 128]))]
        );
    }

    #[test]
    fn mean_color_brightness_is_within_one_of_per_pixel_brightness() {
        // Pseudo-random 16x16 RGB, rendered onto 4x2 cells of 4x8 pixels each.
        let mut state = 0x2545_f491_u32;
        let pixels: Vec<u8> = (0..16 * 16 * 3)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect();
        let cells = render(&pixels, 16, 16, PixelFormat::Rgb8, grid(4, 2));
        for (i, color) in cells.colors().iter().enumerate() {
            let (cx, cy) = (i % 4 * 4, i / 4 * 8);
            let per_pixel: u32 = (cy..cy + 8)
                .flat_map(|y| (cx..cx + 4).map(move |x| (y * 16 + x) * 3))
                .map(|p| u32::from(bt601(&[pixels[p], pixels[p + 1], pixels[p + 2]])))
                .sum();
            let per_pixel = (per_pixel + 16) / 32;
            assert!(per_pixel.abs_diff(u32::from(bt601(color))) <= 1, "cell {i}");
        }
    }

    #[test]
    fn separates_rows_with_newlines() {
        let pixels = [0, 0, 255, 255];
        let cells = render(&pixels, 2, 2, PixelFormat::Luma8, grid(2, 2));
        assert_eq!(plain(&cells), "  \n@@");
    }

    #[test]
    fn horizontal_gradient_brightens_left_to_right() {
        // Ten flat bands, 0 to 255 in even steps, each six pixels wide.
        let w = 60;
        let pixels: Vec<u8> = (0..w * 8).map(|i| ((i % w) / 6 * 255 / 9) as u8).collect();
        let cells = render(&pixels, w, 8, PixelFormat::Luma8, grid(10, 1));
        assert_eq!(plain(&cells), Ramp::DEFAULT);
    }

    #[test]
    fn upsamples_when_grid_is_finer_than_image() {
        let pixels = [0, 255];
        let cells = render(&pixels, 2, 1, PixelFormat::Luma8, grid(4, 2));
        assert_eq!(plain(&cells), "  @@\n  @@");
    }

    #[test]
    fn color_formats_use_bt601_weights() {
        let ramp = Ramp::default();
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255];
        let rgba = [255, 0, 0, 9, 0, 255, 0, 9, 0, 0, 255, 9];
        let expected: Vec<u8> = [76, 149, 28].into_iter().map(|l| ramp.lookup(l)).collect();
        let colors = [[255, 0, 0], [0, 255, 0], [0, 0, 255]];
        for (pixels, format) in [(&rgb[..], PixelFormat::Rgb8), (&rgba, PixelFormat::Rgba8)] {
            let cells = render(pixels, 3, 1, format, grid(3, 1));
            assert_eq!(cells.chars(), expected);
            assert_eq!(cells.colors(), colors);
        }
    }

    #[test]
    fn render_is_repeatable_and_reuses_output() {
        let pixels: Vec<u8> = (0..=255).collect();
        let mut converter = Converter::default();
        converter
            .set_image(&pixels, nz(16), nz(16), PixelFormat::Luma8)
            .unwrap();
        let mut cells = Cells::default();
        converter.render(grid(4, 2), &mut cells);
        converter.render(grid(8, 4), &mut cells);
        let first = cells.clone();
        converter.render(grid(8, 4), &mut cells);
        assert_eq!(cells, first);
        assert_eq!((cells.cols(), cells.rows()), (8, 4));
        assert_eq!(cells.chars().len(), 32);
        assert_eq!(cells.colors().len(), 32);
    }

    #[test]
    fn renders_nothing_without_an_image() {
        let mut cells = render(&[0], 1, 1, PixelFormat::Luma8, grid(1, 1));
        Converter::default().render(grid(4, 2), &mut cells);
        assert!(cells.is_empty());
        assert_eq!((cells.cols(), cells.rows()), (0, 0));
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
