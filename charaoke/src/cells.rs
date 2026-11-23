/// The output of a [`Converter::render`](crate::Converter::render): one character and one color
/// per cell, row by row.
///
/// Slot `r * cols + c` of [`chars`](Self::chars) and [`colors`](Self::colors) holds the cell in
/// row `r`, column `c`. A `Cells` can be reused across renders, so its buffers stop allocating once
/// they have grown to fit the largest grid.
///
/// ```rust
/// use std::num::NonZeroU32;
/// use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat};
///
/// let nz = |n| NonZeroU32::new(n).unwrap();
/// // Two pixels, red then white.
/// let pixels = [255, 0, 0, 255, 255, 255];
///
/// let mut converter = Converter::default();
/// converter.set_image(&pixels, nz(2), nz(1), PixelFormat::Rgb8).unwrap();
/// let mut cells = Cells::default();
/// converter.render(Grid::fit(nz(2), nz(2), nz(2)), &mut cells);
///
/// let mut plain = String::new();
/// cells.write_plain(&mut plain);
/// assert_eq!(plain, "-@");
///
/// let mut ansi = String::new();
/// cells.write_ansi(ColorDepth::TrueColor, &mut ansi);
/// assert_eq!(ansi, "\x1b[38;2;255;0;0m-\x1b[38;2;255;255;255m@\x1b[0m");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cells {
    cols: usize,
    rows: usize,
    chars: Vec<u8>,
    colors: Vec<[u8; 3]>,
}

/// How many colors [`Cells::write_ansi`] may use.
///
/// `charaoke` does not detect what the terminal supports. Callers usually pick a depth from
/// `COLORTERM` (`truecolor` or `24bit` means [`TrueColor`](Self::TrueColor)) and fall back to
/// [`Ansi256`](Self::Ansi256) when unsure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorDepth {
    /// 24-bit color, written as `ESC[38;2;R;G;Bm`.
    TrueColor,
    /// The fixed 256-color palette, written as `ESC[38;5;Nm` with `N` from [`ansi256`].
    Ansi256,
}

impl Cells {
    /// Number of cells per row.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Whether there are no cells, as after rendering with no image loaded.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Each cell's character, row by row. Every byte is printable ASCII.
    pub fn chars(&self) -> &[u8] {
        &self.chars
    }

    /// Each cell's mean color as `[red, green, blue]`, row by row.
    pub fn colors(&self) -> &[[u8; 3]] {
        &self.colors
    }

    /// Writes the characters alone, replacing the contents of `out`.
    ///
    /// Rows are separated by `\n`, with none after the last.
    pub fn write_plain(&self, out: &mut String) {
        out.clear();
        out.reserve(self.chars.len() + self.rows);
        for (r, row) in self.char_rows().enumerate() {
            if r > 0 {
                out.push('\n');
            }
            out.extend(row.iter().map(|&b| char::from(b)));
        }
    }

    /// Writes the characters colored with ANSI escapes, replacing the contents of `out`.
    ///
    /// An escape is written only where a cell's color differs from the cell before it in the same
    /// row. Each row ends with a reset (`ESC[0m`), and rows are separated by `\n`, with none after
    /// the last.
    pub fn write_ansi(&self, depth: ColorDepth, out: &mut String) {
        out.clear();
        // About a third of cells change color in a photo; this avoids most regrowth.
        out.reserve(self.chars.len() * 8 + self.rows * 5);
        for (r, (chars, colors)) in self.char_rows().zip(self.color_rows()).enumerate() {
            if r > 0 {
                out.push('\n');
            }
            let mut prev: Option<[u8; 3]> = None;
            let mut prev_index = None;
            for (&ch, &color) in chars.iter().zip(colors) {
                if prev != Some(color) {
                    match depth {
                        ColorDepth::TrueColor => {
                            out.push_str("\x1b[38;2;");
                            push_u8(out, color[0]);
                            out.push(';');
                            push_u8(out, color[1]);
                            out.push(';');
                            push_u8(out, color[2]);
                            out.push('m');
                        }
                        ColorDepth::Ansi256 => {
                            // Different colors can share a palette entry, so compare indices too.
                            let index = ansi256(color);
                            if prev_index != Some(index) {
                                out.push_str("\x1b[38;5;");
                                push_u8(out, index);
                                out.push('m');
                                prev_index = Some(index);
                            }
                        }
                    }
                    prev = Some(color);
                }
                out.push(char::from(ch));
            }
            out.push_str("\x1b[0m");
        }
    }

    fn char_rows(&self) -> std::slice::Chunks<'_, u8> {
        self.chars.chunks(self.cols.max(1))
    }

    fn color_rows(&self) -> std::slice::Chunks<'_, [u8; 3]> {
        self.colors.chunks(self.cols.max(1))
    }

    /// Empties the cells, keeping their buffers.
    pub(crate) fn clear(&mut self) {
        self.cols = 0;
        self.rows = 0;
        self.chars.clear();
        self.colors.clear();
    }

    /// Sets the grid size and makes room for its cells, which [`push`](Self::push) then fills.
    pub(crate) fn reserve(&mut self, cols: usize, rows: usize) {
        self.cols = cols;
        self.rows = rows;
        self.chars.reserve(cols * rows);
        self.colors.reserve(cols * rows);
    }

    #[inline]
    pub(crate) fn push(&mut self, ch: u8, color: [u8; 3]) {
        self.chars.push(ch);
        self.colors.push(color);
    }
}

/// Channel levels of the 6x6x6 color cube at palette indices 16 to 231.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Nearest cube level for each channel value.
const CUBE_INDEX: [u8; 256] = {
    let mut table = [0; 256];
    let mut v = 0;
    while v < 256 {
        table[v] = match v {
            0..48 => 0,
            48..115 => 1,
            115..155 => 2,
            155..195 => 3,
            195..235 => 4,
            _ => 5,
        };
        v += 1;
    }
    table
};

/// The 256-color palette index nearest to `rgb`.
///
/// Only the fixed part of the palette is used: the 6x6x6 color cube (16 to 231) and the gray ramp
/// (232 to 255). Indices 0 to 15 are skipped because terminal themes recolor them. "Nearest" is by
/// squared distance in RGB, and the cube wins ties.
///
/// ```rust
/// assert_eq!(charaoke::ansi256([255, 0, 0]), 196);
/// assert_eq!(charaoke::ansi256([128, 128, 128]), 244);
/// ```
pub fn ansi256(rgb: [u8; 3]) -> u8 {
    let [r, g, b] = rgb.map(|v| CUBE_INDEX[usize::from(v)]);
    let cube = [r, g, b].map(|i| CUBE_LEVELS[usize::from(i)]);
    // The gray ramp is `8 + 10 k` at index `232 + k`. The nearest step to a color is the one
    // nearest its channel mean, `sum / 3`, so `k = round((sum - 24) / 30)`.
    let sum = u16::from(rgb[0]) + u16::from(rgb[1]) + u16::from(rgb[2]);
    let k = (sum.saturating_sub(24) + 15).div_euclid(30).min(23) as u8;
    let gray = 8 + 10 * k;
    if distance(rgb, cube) <= distance(rgb, [gray; 3]) {
        16 + 36 * r + 6 * g + b
    } else {
        232 + k
    }
}

#[inline]
fn distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    (0..3).map(|i| u32::from(a[i].abs_diff(b[i])).pow(2)).sum()
}

/// Appends `v` in decimal.
#[inline]
fn push_u8(out: &mut String, v: u8) {
    if v >= 100 {
        out.push(char::from(b'0' + v / 100));
    }
    if v >= 10 {
        out.push(char::from(b'0' + v / 10 % 10));
    }
    out.push(char::from(b'0' + v % 10));
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use ansi_to_tui::IntoText;
    use ratatui::style::Color;

    use super::*;
    use crate::{Converter, Grid, PixelFormat};

    fn nz(n: u32) -> NonZeroU32 {
        NonZeroU32::new(n).unwrap()
    }

    fn cells(cols: usize, rows: usize, cells: &[(u8, [u8; 3])]) -> Cells {
        let mut out = Cells::default();
        out.reserve(cols, rows);
        for &(ch, color) in cells {
            out.push(ch, color);
        }
        out
    }

    fn ansi(cells: &Cells, depth: ColorDepth) -> String {
        let mut out = String::new();
        cells.write_ansi(depth, &mut out);
        out
    }

    const RED: [u8; 3] = [255, 0, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    #[test]
    fn merges_runs_of_the_same_color() {
        let cells = cells(
            4,
            1,
            &[(b'a', RED), (b'b', RED), (b'c', BLUE), (b'd', BLUE)],
        );
        assert_eq!(
            ansi(&cells, ColorDepth::TrueColor),
            "\x1b[38;2;255;0;0mab\x1b[38;2;0;0;255mcd\x1b[0m"
        );
        assert_eq!(
            ansi(&cells, ColorDepth::Ansi256),
            "\x1b[38;5;196mab\x1b[38;5;21mcd\x1b[0m"
        );
    }

    #[test]
    fn merges_colors_that_share_a_palette_entry() {
        let cells = cells(2, 1, &[(b'a', [250, 0, 0]), (b'b', RED)]);
        assert_eq!(ansi(&cells, ColorDepth::Ansi256), "\x1b[38;5;196mab\x1b[0m");
    }

    #[test]
    fn each_row_resets_and_starts_its_own_run() {
        let cells = cells(1, 2, &[(b'a', RED), (b'b', RED)]);
        assert_eq!(
            ansi(&cells, ColorDepth::TrueColor),
            "\x1b[38;2;255;0;0ma\x1b[0m\n\x1b[38;2;255;0;0mb\x1b[0m"
        );
    }

    #[test]
    fn writes_nothing_for_empty_cells() {
        let empty = Cells::default();
        let mut out = String::from("stale");
        empty.write_ansi(ColorDepth::TrueColor, &mut out);
        assert!(out.is_empty());
        empty.write_plain(&mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn ansi256_maps_cube_corners_and_grays() {
        assert_eq!(ansi256([0, 0, 0]), 16);
        assert_eq!(ansi256([255, 255, 255]), 231);
        assert_eq!(ansi256([0, 255, 0]), 46);
        assert_eq!(ansi256([95, 135, 175]), 16 + 36 + 12 + 3);
        assert_eq!(ansi256([8, 8, 8]), 232);
        assert_eq!(ansi256([238, 238, 238]), 255);
        assert_eq!(ansi256([128, 128, 128]), 244);
    }

    #[test]
    fn ansi256_picks_the_nearest_entry() {
        let palette: Vec<(u8, [u8; 3])> = (16..=255u8)
            .map(|i| {
                let color = if i < 232 {
                    let n = usize::from(i - 16);
                    [n / 36, n / 6 % 6, n % 6].map(|l| CUBE_LEVELS[l])
                } else {
                    [8 + 10 * (i - 232); 3]
                };
                (i, color)
            })
            .collect();
        for rgb in (0..=255u8).step_by(5).flat_map(|r| {
            (0..=255u8)
                .step_by(15)
                .flat_map(move |g| (0..=255u8).step_by(17).map(move |b| [r, g, b]))
        }) {
            let best = palette
                .iter()
                .map(|&(_, c)| distance(rgb, c))
                .min()
                .unwrap();
            let got = palette[usize::from(ansi256(rgb) - 16)].1;
            assert_eq!(distance(rgb, got), best, "{rgb:?}");
        }
    }

    #[test]
    fn round_trips_through_ansi_to_tui() {
        // A 4x4 image: red, green, blue and white quadrants.
        let mut pixels = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                let color = match (x < 2, y < 2) {
                    (true, true) => RED,
                    (false, true) => [0, 255, 0],
                    (true, false) => BLUE,
                    (false, false) => [255; 3],
                };
                pixels.extend(color);
            }
        }
        let mut converter = Converter::default();
        converter
            .set_image(&pixels, nz(4), nz(4), PixelFormat::Rgb8)
            .unwrap();
        let mut rendered = Cells::default();
        converter.render(Grid::fit(nz(4), nz(4), nz(2)), &mut rendered);
        assert_eq!((rendered.cols(), rendered.rows()), (2, 1));

        let text = ansi(&rendered, ColorDepth::TrueColor)
            .into_text()
            .expect("parses");
        assert_eq!(text.lines.len(), 1);
        let spans: Vec<(String, Option<Color>)> = text.lines[0]
            .spans
            .iter()
            .filter(|span| !span.content.is_empty())
            .map(|span| (span.content.to_string(), span.style.fg))
            .collect();
        let expected: Vec<(String, Option<Color>)> = rendered
            .chars()
            .iter()
            .zip(rendered.colors())
            .map(|(&ch, &[r, g, b])| (char::from(ch).to_string(), Some(Color::Rgb(r, g, b))))
            .collect();
        assert_eq!(spans, expected);
    }
}
