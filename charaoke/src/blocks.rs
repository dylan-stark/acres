use crate::cells::push_u8;
use crate::{ColorDepth, ansi256};

/// Block glyphs indexed by quadrant mask: bit 0 is the top-left quarter, bit 1 the top right,
/// bit 2 the bottom left and bit 3 the bottom right. A set bit is drawn in the foreground color.
pub const QUADRANTS: [char; 16] = [
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
];

/// The output of a [`Converter::render_blocks`](crate::Converter::render_blocks): one block glyph,
/// a foreground and a background color per cell, row by row.
///
/// Slot `r * cols + c` of each slice holds the cell in row `r`, column `c`. Each cell's glyph is a
/// quadrant mask (see [`QUADRANTS`]), so half blocks and quadrant blocks share this type. Like
/// [`Cells`](crate::Cells), a `Blocks` can be reused across renders.
///
/// ```rust
/// use std::num::NonZeroU32;
/// use charaoke::{BlockMode, Blocks, ColorDepth, Converter, Grid, PixelFormat};
///
/// let nz = |n| NonZeroU32::new(n).unwrap();
/// // Red above blue.
/// let pixels = [255, 0, 0, 0, 0, 255];
///
/// let mut converter = Converter::default();
/// converter.set_image(&pixels, nz(1), nz(2), PixelFormat::Rgb8).unwrap();
/// let mut blocks = Blocks::default();
/// converter.render_blocks(Grid::fit(nz(1), nz(2), nz(1)), BlockMode::Half, &mut blocks);
///
/// let mut ansi = String::new();
/// blocks.write_ansi(ColorDepth::TrueColor, &mut ansi);
/// assert_eq!(ansi, "\x1b[38;2;255;0;0;48;2;0;0;255m▀\x1b[0m");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blocks {
    cols: usize,
    rows: usize,
    masks: Vec<u8>,
    foreground: Vec<[u8; 3]>,
    background: Vec<[u8; 3]>,
}

impl Blocks {
    /// Number of cells per row.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Number of rows of cells.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Whether there are no cells, as after rendering with no image loaded.
    pub fn is_empty(&self) -> bool {
        self.masks.is_empty()
    }

    /// Each cell's quadrant mask, row by row. Every mask is below 16.
    pub fn masks(&self) -> &[u8] {
        &self.masks
    }

    /// Each cell's glyph, row by row: its mask looked up in [`QUADRANTS`].
    pub fn glyphs(&self) -> impl Iterator<Item = char> + '_ {
        self.masks.iter().map(|&m| QUADRANTS[usize::from(m)])
    }

    /// Each cell's foreground color as `[red, green, blue]`, row by row.
    pub fn foreground(&self) -> &[[u8; 3]] {
        &self.foreground
    }

    /// Each cell's background color as `[red, green, blue]`, row by row.
    pub fn background(&self) -> &[[u8; 3]] {
        &self.background
    }

    /// Writes the glyphs with ANSI foreground and background colors, replacing the contents of
    /// `out`.
    ///
    /// The foreground and background are tracked separately, and an escape is written only for
    /// those that differ from the cell before in the same row. Each row ends with a reset
    /// (`ESC[0m`), and rows are separated by `\n`, with none after the last. There is no plain
    /// writer, because block glyphs without their colors are only shapes.
    pub fn write_ansi(&self, depth: ColorDepth, out: &mut String) {
        out.clear();
        // A glyph is 3 bytes, and colors change often in a photo.
        out.reserve(self.masks.len() * 20 + self.rows * 5);
        let cols = self.cols.max(1);
        let rows = self
            .masks
            .chunks(cols)
            .zip(self.foreground.chunks(cols))
            .zip(self.background.chunks(cols));
        for (r, ((masks, fgs), bgs)) in rows.enumerate() {
            if r > 0 {
                out.push('\n');
            }
            let mut prev_fg = None;
            let mut prev_bg = None;
            for ((&mask, &fg), &bg) in masks.iter().zip(fgs).zip(bgs) {
                let (fg, bg) = match depth {
                    ColorDepth::TrueColor => (Color::Rgb(fg), Color::Rgb(bg)),
                    ColorDepth::Ansi256 => {
                        (Color::Indexed(ansi256(fg)), Color::Indexed(ansi256(bg)))
                    }
                };
                let new_fg = prev_fg != Some(fg);
                let new_bg = prev_bg != Some(bg);
                if new_fg || new_bg {
                    out.push_str("\x1b[");
                    if new_fg {
                        fg.push(out, b'3');
                    }
                    if new_bg {
                        if new_fg {
                            out.push(';');
                        }
                        bg.push(out, b'4');
                    }
                    out.push('m');
                    prev_fg = Some(fg);
                    prev_bg = Some(bg);
                }
                out.push(QUADRANTS[usize::from(mask)]);
            }
            out.push_str("\x1b[0m");
        }
    }

    /// Empties the blocks, keeping their buffers.
    pub(crate) fn clear(&mut self) {
        self.cols = 0;
        self.rows = 0;
        self.masks.clear();
        self.foreground.clear();
        self.background.clear();
    }

    /// Sets the grid size and makes room for its cells, which [`push`](Self::push) then fills.
    pub(crate) fn reserve(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.masks.reserve(cols * rows);
        self.foreground.reserve(cols * rows);
        self.background.reserve(cols * rows);
    }

    #[inline]
    pub(crate) fn push(&mut self, mask: u8, foreground: [u8; 3], background: [u8; 3]) {
        self.masks.push(mask);
        self.foreground.push(foreground);
        self.background.push(background);
    }
}

/// A color as written, so that runs compare by what reaches the terminal.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    Rgb([u8; 3]),
    Indexed(u8),
}

impl Color {
    /// Appends the SGR parameters for this color, with `layer` `3` for foreground or `4` for
    /// background.
    fn push(self, out: &mut String, layer: u8) {
        out.push(char::from(layer));
        match self {
            Color::Rgb([r, g, b]) => {
                out.push_str("8;2;");
                push_u8(out, r);
                out.push(';');
                push_u8(out, g);
                out.push(';');
                push_u8(out, b);
            }
            Color::Indexed(i) => {
                out.push_str("8;5;");
                push_u8(out, i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{BlockMode, Converter, Grid, PixelFormat};

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];
    const WHITE: [u8; 3] = [255; 3];

    /// Renders an RGB image of `w` by `h` pixels onto `cols` by `rows` cells.
    fn render(pixels: &[[u8; 3]], w: u32, h: u32, cols: u32, mode: BlockMode) -> Blocks {
        let mut converter = Converter::default();
        converter
            .set_image(pixels.as_flattened(), nz(w), nz(h), PixelFormat::Rgb8)
            .unwrap();
        let mut blocks = Blocks::default();
        converter.render_blocks(Grid::fit(nz(w), nz(h), nz(cols)), mode, &mut blocks);
        blocks
    }

    fn ansi(blocks: &Blocks, depth: ColorDepth) -> String {
        let mut out = String::new();
        blocks.write_ansi(depth, &mut out);
        out
    }

    #[test]
    fn half_blocks_take_top_and_bottom_colors() {
        // A 2x2 image of four colors fits a grid of 2x1 cells.
        let blocks = render(&[RED, GREEN, BLUE, WHITE], 2, 2, 2, BlockMode::Half);
        assert_eq!((blocks.cols(), blocks.rows()), (2, 1));
        assert_eq!(blocks.glyphs().collect::<String>(), "▀▀");
        assert_eq!(blocks.foreground(), [RED, GREEN]);
        assert_eq!(blocks.background(), [BLUE, WHITE]);
    }

    #[test]
    fn quadrants_split_left_from_right() {
        let blocks = render(&[RED, BLUE, RED, BLUE], 2, 2, 1, BlockMode::Quadrant);
        assert_eq!(blocks.glyphs().collect::<String>(), "▌");
        assert_eq!(
            (blocks.foreground(), blocks.background()),
            (&[RED][..], &[BLUE][..])
        );
    }

    #[test]
    fn quadrants_pick_out_one_odd_quarter() {
        for (odd, glyph) in [(0, '▘'), (1, '▝'), (2, '▖')] {
            let mut pixels = [WHITE; 4];
            pixels[odd] = RED;
            let blocks = render(&pixels, 2, 2, 1, BlockMode::Quadrant);
            assert_eq!(blocks.glyphs().next(), Some(glyph), "quarter {odd}");
            assert_eq!(blocks.foreground(), [RED]);
            assert_eq!(blocks.background(), [WHITE]);
        }
        // The bottom-right pixel always lands in the background group, so its complement shows.
        let blocks = render(&[WHITE, WHITE, WHITE, RED], 2, 2, 1, BlockMode::Quadrant);
        assert_eq!(blocks.glyphs().next(), Some('▛'));
        assert_eq!(
            (blocks.foreground(), blocks.background()),
            (&[WHITE][..], &[RED][..])
        );
    }

    #[test]
    fn quadrants_average_each_group() {
        // A diagonal: two near-reds and two near-blues.
        let pixels = [[250, 0, 0], [0, 0, 250], [0, 0, 240], [240, 0, 0]];
        let blocks = render(&pixels, 2, 2, 1, BlockMode::Quadrant);
        assert_eq!(blocks.glyphs().next(), Some('▞'));
        assert_eq!(blocks.foreground(), [[0, 0, 245]]);
        assert_eq!(blocks.background(), [[245, 0, 0]]);
    }

    #[test]
    fn flat_cells_draw_as_half_blocks() {
        let blocks = render(&[GREEN; 4], 2, 2, 1, BlockMode::Quadrant);
        assert_eq!(blocks.glyphs().next(), Some('▀'));
        assert_eq!(
            (blocks.foreground(), blocks.background()),
            (&[GREEN][..], &[GREEN][..])
        );
    }

    #[test]
    fn writes_only_the_colors_that_change() {
        // Cells: red/blue, red/green, white/green.
        let mut blocks = Blocks::default();
        blocks.reserve(3, 1);
        blocks.push(HALF, RED, BLUE);
        blocks.push(HALF, RED, GREEN);
        blocks.push(HALF, WHITE, GREEN);
        assert_eq!(
            ansi(&blocks, ColorDepth::TrueColor),
            "\x1b[38;2;255;0;0;48;2;0;0;255m▀\x1b[48;2;0;255;0m▀\x1b[38;2;255;255;255m▀\x1b[0m"
        );
        assert_eq!(
            ansi(&blocks, ColorDepth::Ansi256),
            "\x1b[38;5;196;48;5;21m▀\x1b[48;5;46m▀\x1b[38;5;231m▀\x1b[0m"
        );
    }

    #[test]
    fn merges_colors_that_share_a_palette_entry() {
        let mut blocks = Blocks::default();
        blocks.reserve(2, 1);
        blocks.push(HALF, [250, 0, 0], BLUE);
        blocks.push(HALF, RED, [0, 0, 250]);
        assert_eq!(
            ansi(&blocks, ColorDepth::Ansi256),
            "\x1b[38;5;196;48;5;21m▀▀\x1b[0m"
        );
    }

    #[test]
    fn each_row_resets_and_starts_its_own_run() {
        let mut blocks = Blocks::default();
        blocks.reserve(1, 2);
        blocks.push(0b0101, RED, BLUE);
        blocks.push(0b0101, RED, BLUE);
        assert_eq!(
            ansi(&blocks, ColorDepth::TrueColor),
            "\x1b[38;2;255;0;0;48;2;0;0;255m▌\x1b[0m\n\x1b[38;2;255;0;0;48;2;0;0;255m▌\x1b[0m"
        );
    }

    #[test]
    fn renders_nothing_without_an_image() {
        let mut blocks = render(&[RED; 4], 2, 2, 1, BlockMode::Half);
        Converter::default().render_blocks(
            Grid::fit(nz(2), nz(2), nz(1)),
            BlockMode::Half,
            &mut blocks,
        );
        assert!(blocks.is_empty());
        let mut out = String::from("stale");
        blocks.write_ansi(ColorDepth::TrueColor, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn render_blocks_is_repeatable_and_reuses_output() {
        let pixels: Vec<[u8; 3]> = (0..=255u8).map(|v| [v, 255 - v, v / 2]).collect();
        let mut converter = Converter::default();
        converter
            .set_image(pixels.as_flattened(), nz(16), nz(16), PixelFormat::Rgb8)
            .unwrap();
        let mut blocks = Blocks::default();
        for mode in [BlockMode::Half, BlockMode::Quadrant] {
            let grid = Grid::fit(nz(16), nz(16), nz(8));
            converter.render_blocks(Grid::fit(nz(16), nz(16), nz(4)), mode, &mut blocks);
            converter.render_blocks(grid, mode, &mut blocks);
            let first = blocks.clone();
            converter.render_blocks(grid, mode, &mut blocks);
            assert_eq!(blocks, first);
            assert_eq!((blocks.cols(), blocks.rows()), (8, 4));
            assert_eq!(blocks.masks().len(), 32);
        }
    }

    const HALF: u8 = 0b0011;
}
