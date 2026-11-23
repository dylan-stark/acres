# 0011. Switch the TUI and CLI to `charaoke`

- Status: Accepted
- Date: 2026-11-23

## Context

Both consumers still go through `image-to-ascii-builder`. ADRs 0008 to 0010 built `charaoke` to replace it, and a color resize now takes 0.96 ms on the M1 Air, against an 8 ms budget.

The TUI (`acres-tui`):

- Fetches each artwork 843 px wide from IIIF (`components/iiif.rs`) and sends the JPEG bytes to `ImageToAsciiBuilder`.
- That component rebuilds the art in a Tokio task whenever the image, alphabet, algorithm, font or metric changes. Each rebuild redoes decode, font loading and conversion, about 34 ms end to end on the Air.
- The width is always the default, `CharWidth::ImageWidthInChars`, so the art never fits the pane. A terminal resize only redraws the same text, centered and clipped.
- `Home` parses the ANSI string back with `ansi-to-tui` on every update (about 1.1 ms) and draws it as a `Paragraph`.
- Users can browse alphabets (`a`), conversion algorithms (`c`), fonts (`f`) and metrics (`m`) in lists.

The CLI (`acres-cli ascii-art <image>`):

- Takes `--alphabet`, `--brightness-offset`, `--conversion-algorithm`, `--font`, `--metric` and `--width` (default: image width).
- Always writes 24-bit ANSI, even when piped.

So this step gives the TUI its first real fit-to-pane resize, not just a faster version of the old one.

`image-to-ascii` chooses each character by matching glyph shapes from a font against the image. `charaoke` chooses by brightness along a ramp, which is why half of the old options have no counterpart:

| Before | Where | Now | Cost to `render` |
| --- | --- | --- | --- |
| `--alphabet` (7 named character sets) | CLI, TUI list `a` | `--ramp`, taking a preset name or a custom string. The presets are the same 7 sets, each ordered by how much ink its glyphs cover, worked out once and stored as constants | None |
| `--brightness-offset` (0 to 255, subtracted from brightness) | CLI | Kept, and built into the `Ramp` table, so it's free at render time | None |
| `--width` (default: image width) | CLI | Kept. The default becomes the terminal width, or 80 when output isn't a terminal | None |
| `--font`, `--metric`, `--conversion-algorithm` | CLI, TUI lists `f`, `m` and `c` | Removed. They only make sense with glyph matching, which comes back later as edge-aware characters | n/a |
| Width in the TUI | Fixed | Fits the pane on every resize (new) | One `render` per resize |
| Color | Always 24-bit | `--color auto\|truecolor\|256\|never`. `auto` reads `NO_COLOR`, `COLORTERM` and `TERM` (ADR 0010), and turns color off when stdout isn't a terminal. `truecolor` or `256` forces color either way | None |

## Decision

The TUI and CLI switch to `charaoke` in one PR, and `image-to-ascii-builder` stays in the workspace only for the baseline benches. We drop the glyph-matching options rather than keep the old path behind a toggle: building the new path right is worth more than maintaining a legacy one.

`charaoke` gains a small API for this:

- `Ramp::PRESETS` and `Ramp::preset(name)`, the 7 named ramps. The binaries default to `alphabet`, to stay close to the old look. The library default is still the 10-character `Ramp::DEFAULT`.
- `Ramp::with_brightness_offset(u8)`.
- `Grid::fit_within(width, height, max_cols, max_rows)`, which picks the most columns whose rows still fit the pane.
- `Converter::set_ramp`, so the TUI can change ramps without reloading the image.

TUI:

- When an image arrives, it's decoded once with `image` 0.25 and loaded into a `Converter` kept on `Home`. No Tokio task is needed, since everything after decode takes about 1 ms.
- On every draw, `Home` renders the image to fit its pane with `Grid::fit_within`, skipping the render when the pane and ramp haven't changed. A `CellsWidget` then writes the cells straight into the `Buffer`, as in the `tui_proxy` bench.
- This replaces `UpdateAscii(Ascii)`, `ansi-to-tui` and the `Paragraph`, so `acres-tui` drops the `ansi-to-tui` dependency.
- The color depth is chosen once at startup from `COLORTERM`: `Color::Rgb` for truecolor, `Color::Indexed(ansi256(..))` otherwise. Neither crossterm nor Ratatui downgrades colors, so the choice has to be ours.
- The alphabets list (`a`) becomes a ramps list. The font (`f`), metric (`m`) and algorithm (`c`) lists, their actions and their keybindings are removed.

CLI:

- `ascii-art` decodes the image, renders once, and writes with `write_ansi` or `write_plain`, using `--ramp`, `--brightness-offset`, `--width` and `--color` from the table above.
- `--font`, `--metric` and `--conversion-algorithm` are removed, so passing one gives clap's usual unknown-argument error.

Timing:

`charaoke` itself stays free of timing code. The binaries time each stage with `std::time::Instant`, which costs tens of nanoseconds a call, so timing is always on and only showing it is optional.

- TUI HUD, toggled with `t`: a 34-column drawer slides out over the right edge of the art. The art keeps its grid and isn't re-rendered: each frame draws the art as usual, then clears the strip with Ratatui's `Clear` and draws the HUD on top, so toggling changes no art timings. It shows a scrolling feed of the last 200 events, newest at the bottom, each as a label followed by its stages:
  - `image <id>`: `fetch`, `decode`, `load`.
  - `resize <cols>x<rows>`: `render`, `draw` (the `CellsWidget` write).
  - `frame`: `total`, the whole `terminal.draw` measured in the app loop, plus `hud`, the HUD's own draw, while it's open, so `total` can be read with or without it.

  The app loop draws at the frame rate, so a `frame` entry is only added after an event that did work (a new image, a resize or a toggle). Otherwise the feed would fill with idle frames.
- CLI `--timings`: the stages `read`, `decode`, `load`, `render`, `write` and `total` go to stderr, one per line, after stdout has been written and flushed. An image shown in the terminal is never split by timing lines, and redirecting stdout keeps the file clean.
- Each binary has one small type for this, a label plus stage names and `Duration`s: `Timings` in the CLI and `action::Timing` in the TUI.

Dependencies and docs:

- `acres-cli` and `acres-tui` gain `charaoke` and `image` 0.25 (JPEG and PNG only), and drop `image-to-ascii-builder`. CLAUDE.md's dependency rules list `charaoke` as allowed for both.
- The README's `ascii-art` examples and TUI key list are updated, including `--alphabet fast`, which becomes `--ramp fast`.

## Consequences

- Benches and tests:
  - `charaoke/tui_proxy/drag` draws with the bench's copy of `CellsWidget` (`charaoke/benches/common/cells_widget.rs`), because a library can't depend on its consumer. A unit test in `acres-tui` includes that file and checks that the real widget draws the same buffer, so the test fails if the two drift apart.
  - New `charaoke/tui_proxy/drag_fit`: the drag again, through `Grid::fit_within` on a pane that changes both width and height. Its cost should match `drag`.
  - TUI snapshots: `renders_ascii_centered` is re-recorded from a fixture image, the alphabets browser becomes a ramps browser, the metrics browser is deleted, and a HUD snapshot uses fixed durations.
  - CLI tests run `ascii-art` on the fixture JPEG: `--color never` gives plain text of the expected size, `--color 256` writes `38;5;` escapes, `auto` is plain through a pipe, and `--timings` leaves stdout unchanged and writes every stage to stderr.
- The TUI fits the art to the pane and re-renders on every resize, at under 1 ms per frame on the Air instead of a 34 ms rebuild on each option change. A new image costs about one decode (1.3 ms) on top of that. These are from one `make bench` run per machine, on the synthetic 843x632 bench image (results on PR #71; the Ryzen ran only the `tui_proxy` group):

  | | M1 Air (target) | Ryzen 9700X (dev) |
  | --- | --- | --- |
  | `charaoke/tui_proxy/drag` (120-200 col drag, width only) | 0.94 ms | 0.53 ms |
  | `charaoke/tui_proxy/drag_fit` (width and height, through `fit_within`) | 0.76 ms | 0.46 ms |
  | `charaoke/tui_proxy/drag_ansi` (old `ansi-to-tui` path) | 1.65 ms | 1.10 ms |
  | `charaoke/render` at 160 columns | 0.52 ms | n/a |
  | `charaoke/decode` | 1.31 ms | n/a |

  `drag_fit` is cheaper than `drag` because its 30 to 60 row panes limit most frames by height, so they average about half the cells. The gap is smaller than that because Ratatui's fixed cost for drawing the whole terminal is the same in both.
- Character choice is brightness-only until edge-aware characters return. Line art and text in artworks look softer than with the old edge-augmented mode. This is the biggest visible change.
- Removing CLI flags breaks scripts that pass `--font`, `--metric` or `--conversion-algorithm`. The project is pre-1.0 and these flags only steered glyph matching, so we remove them instead of keeping no-op flags.
- Both binaries drop `image` 0.24 and `image-to-ascii`'s font loading.
- `image-to-ascii-builder` no longer has users in the workspace. It stays for `baseline/*` until edge-aware characters land in `charaoke`, then it can go, along with its CLAUDE.md rules.
- The x86-64 render investigation, half blocks, edge-aware characters, light-terminal support and gamma-correct averaging each follow as their own ADR.
