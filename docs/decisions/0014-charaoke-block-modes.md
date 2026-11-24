# 0014. `charaoke` adds half-block and quadrant modes

- Status: Accepted
- Date: 2026-11-24

## Context

ASCII art spends each cell on one character and one color. A character can only suggest brightness, and a cell is about twice as tall as it is wide, so a 100-column render of an 843x660 painting has only about 39 rows of detail.

Edge-aware glyphs (ADR 0012) were rejected because they added noise without adding detail. A probe of other ways to use a cell rendered the same fixtures at 100 columns:

- Half blocks: each cell draws `▀` in two colors, which gives about 78 rows of roughly square pixels. The Bedroom and The Great Wave are clearly recognizable in full color.
- Quadrant blocks (U+2596 to U+259F): each cell holds a 2x2 grid of pixels in two colors. Edges sharpen side to side, as in the Bedroom's chair rungs and floorboards and the Great Wave's foam. With only two colors for four pixels, flat areas such as the Wave's sky can show faint vertical streaks.
- Braille (U+2800 to U+28FF): each cell holds a 2x4 grid of dots, picked by dithering. The shapes come through, but the dots cover little of the cell and each cell gets only one color, so the image is dim and hard to read.

How terminals handle it:

- `▀` and the quadrants are in Unicode's Block Elements, which almost every terminal font includes. Most terminals draw block elements edge to edge, but a few leave thin gaps between rows when their line spacing is above 1.
- A background color uses the same SGR codes as a foreground color, with 48 in place of 38. It works at truecolor and at 256 colors.
- Setting the background ourselves means the art no longer depends on whether the terminal is light or dark.

The budget has room: an ASCII resize takes 0.94 ms on the M1 Air (`charaoke/tui_proxy/drag`), against the 8 ms target.

| Option | Pixels per cell | Colors per cell | Font support | Trade-off |
| --- | --- | --- | --- | --- |
| A. Half blocks `▀` (included) | 1x2 | 2 (foreground and background) | Block Elements, in nearly every font | Clear and full color, with twice the vertical detail. No longer "ASCII" art |
| B. Quadrant blocks `▖▗▘▝▚▞▙▛▜▟` (included) | 2x2 | 2, chosen per cell | Block Elements, in nearly every font | Twice the horizontal detail too, though each pixel is twice as tall as it is wide. Each cell picks the best of 7 two-color splits of its 4 pixels, which costs more and can streak flat areas |
| C. Braille `⠀` to `⣿` | 2x4 dots | 1 | Wide, but dot size varies by font | Finest detail for lines. Dim and sparse for paintings, as the probe showed |
| D. Sextants `🬀` and up | 2x3 | 2 | Unicode 13 (2020), missing from many fonts | Fine blocks, but too many terminals would show boxes |

A gives the biggest visible gain for the least code. B adds horizontal detail for a split search per cell, and shares A's pixel grid, output type and writer, so both ship together.

## Decision

We add two block modes, half blocks and quadrant blocks, beside ASCII, in `charaoke` and in both binaries. Both split each terminal cell into pixels, average each pixel's color, and draw a block glyph with a foreground and a background color.

Grid:

- All modes use the same `Grid` from `Grid::fit` and `Grid::fit_within`. A cell is twice as tall as it is wide, so splitting it into 1x2 pixels gives square pixels, and 2x2 gives pixels twice as tall as they are wide. No new fitting functions are needed.
- `BlockMode::{Half, Quadrant}` names the block mode, and `BlockMode::pixels_per_cell()` gives 1x2 or 2x2.

Render:

- `Converter::render_blocks(grid, mode, &mut Blocks)` averages each pixel's color with the same summing pass `render` uses, on a pixel grid of `cols * px_w` by `rows * px_h`. The pass moves into a shared private function, so every mode reads each image pixel once.
- Half blocks: the glyph is always `▀`, with foreground the top pixel and background the bottom one.
- Quadrant blocks: each cell tries the 7 ways to split its 4 pixels into two groups, colors each group with its mean, and keeps the split with the smallest squared RGB error. The bottom-right pixel is always in the background group, so each split is tried once. `▀` is tried first and wins ties, so a flat cell draws as a half block.
- `Blocks` holds, per terminal cell, a 4-bit quadrant mask, a foreground and a background color, plus `cols()` and `rows()`. `QUADRANTS` maps a mask to its glyph. Half blocks are the case where the mask is always the top two quarters.

Output:

- `Blocks::write_ansi(depth, &mut String)` writes each cell's glyph with its foreground and background.
- It writes an escape only when a color changes, tracking foreground and background separately, and resets at the end of each row.
- 256-color maps both colors through `ansi256`, after the split is chosen in full color.
- There is no plain-text writer, because block glyphs without color are only shapes.

Binaries:

- CLI: `--mode ascii|half|quadrant`, default `ascii`. A block mode with `--color never`, or with `auto` resolving to no color, is an error that says block modes need color. `--ramp` and `--brightness-offset` apply only to ASCII.
- TUI: `b` cycles ASCII, half blocks and quadrant blocks, starting in ASCII. A `BlocksWidget` sets each cell's glyph, foreground and background. The HUD's `resize` entries name the mode.

Braille stays out and waits for its own ADR, if line art ever calls for it. Sextants stay out because too few fonts have them.

## Consequences

- Benches and tests:
  - `charaoke/render_blocks/{half,quadrant}` sit beside `charaoke/render`, and `charaoke/ansi_blocks` covers quadrant output at both color depths.
  - `charaoke/tui_proxy/drag_half` and `drag_quadrant` use the same drag as `drag`, drawing with the bench's copy of `BlocksWidget` (`charaoke/benches/common/blocks_widget.rs`). A unit test in `acres-tui` checks that the real widget draws the same buffer.
  - The gates, on the Air: `drag_half` under 2 ms and `drag_quadrant` under 3 ms, against 0.94 ms for ASCII.
  - Unit tests cover half-block colors, quadrant splits, ANSI runs and resets at both depths, and empty renders. The existing ASCII tests pass unchanged on the shared summing pass.
  - TUI snapshots show a fixture artwork in each block mode. Snapshots hold symbols only, so colors are checked on the widget's buffer.
  - CLI tests check `48;2;` and `48;5;` escapes, the ASCII grid size in block modes, and the error when there is no color.
- Paintings become clearly recognizable at normal widths, in full color, at about the cost of an ASCII render.
- Quadrants add horizontal detail where it helps, but with two colors for four pixels they can leave faint streaks in flat areas. Users can pick whichever suits the artwork.
- Block output is no longer ASCII art. ASCII stays the default and the project's identity, and the block modes are options.
- Each block cell sets its own background. That makes the art independent of light or dark terminal themes, which covers much of the planned light-terminal support for these modes.
- A few terminals leave thin gaps between rows of block characters, and some fonts draw quadrant glyphs short of the cell edge, which shows as thin seams. Several terminals, including kitty, WezTerm and Alacritty, draw block elements themselves and avoid both. Users elsewhere can switch modes.
- `charaoke` gains a second output type, `Blocks`, beside `Cells`, shared by both block modes. The shared summing pass keeps every mode in step, and any later speedup, such as the x86-64 work, benefits all of them.
- ANSI output in the block modes is two to three times as long, which matters only for very large CLI output.
