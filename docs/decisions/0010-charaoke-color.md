# 0010. `charaoke` renders colored cells and writes ANSI separately

- Status: Accepted
- Date: 2026-11-23

## Context

The TUI shows color today, and `charaoke` was grayscale-only (ADR 0009), so color has to come back before the TUI can switch over. There is room in the budget: a grayscale resize takes 0.98 ms on the M1 Air (`charaoke/tui_proxy`), which leaves about 7 ms of the 8 ms target (ADR 0008).

The old color path costs 6.4 ms on the Air and 3.9 ms on the Ryzen at 843x632 and 160 columns (`baseline/color`). `char_rows_to_terminal_color_string` in `image-to-ascii` 0.7 shows why:

- It resizes the whole image again (nearest-neighbor) and converts it to `f32` RGBA on every call.
- It builds a separate `String` for each cell through the `colored` crate, then joins them by row and again by line, which is about 9,600 small allocations at 160 columns.
- Every cell gets a full 24-bit escape plus a reset, even when its neighbor has the same color.

The TUI then parses that string back into Ratatui `Text` with `ansi-to-tui` (`acres-tui/src/components/home.rs`), a cost no bench measured.

We considered three outputs for `render`:

| Option | What `render` produces | Path to the TUI | Trade-off |
| --- | --- | --- | --- |
| A. ANSI string | One `String` with an escape per cell | `ansi-to-tui` parses it back, as today | Smallest TUI change, but an escape round trip every frame |
| B. Structured cells | One character and one mean RGB per cell | A widget writes them straight into Ratatui's `Buffer` | No escapes on the TUI path; the CLI gets ANSI from a separate writer |
| C. Reduced palette | Like B, with colors quantized | Same as B | Shorter ANSI, but visibly lower fidelity |

For B, the cell colors could come from the kept RGB pixels (B1) or from a coarse color plane built at load (B2, cheaper to render but blocky when the grid is finer than the plane).

## Decision

Build B1. `render` produces structured cells in full color from a kept RGB plane, and ANSI is a separate writer.

- `set_image` keeps the pixels as RGB (grayscale input is expanded), and the separate brightness plane goes away.
- `render(grid, &mut Cells)` fills a reusable `Cells` with `chars: Vec<u8>` and `colors: Vec<[u8; 3]>`, row by row. Each cell's color is the mean of its pixels, and its character comes from that mean's BT.601 brightness.
- `Cells::write_ansi(depth, &mut String)` writes truecolor (`ESC[38;2;R;G;Bm`) or 256-color (`ESC[38;5;Nm`) ANSI. It writes an escape only when the color changes within a row and resets at the end of each row.
- The 256-color fallback, `ansi256`, picks the nearest entry by squared RGB distance from the fixed 6x6x6 cube and gray ramp (indices 16 to 255). It skips 0 to 15 because themes recolor them. The cube is separable per channel and the nearest gray follows from the channel mean, so the mapping is exact and needs only a 256-byte table.
- `Cells::write_plain` writes grayscale text, replacing the old `render(grid, &mut String)`.
- Color is always on. `charaoke` doesn't detect the terminal: callers choose the depth from `COLORTERM` and `TERM`, and fall back to 256-color when unsure.
- The Ratatui widget lives in `acres-tui` (step 4), so `charaoke` keeps no runtime dependencies. The `tui_proxy` bench carries a copy until then.

Palette reduction (option C) stays out. It can come later as a `Cells` post-pass if ANSI size becomes the CLI's bottleneck.

## Consequences

- Benches:
  - `charaoke/render` now includes color.
  - `charaoke/ansi` measures `write_ansi` at both depths, to compare with `baseline/color`.
  - `charaoke/end_to_end` adds truecolor ANSI, to compare with `baseline/end_to_end`.
  - `charaoke/tui_proxy/drag` writes cells straight into the `Buffer`.
  - `charaoke/tui_proxy/drag_ansi` and `baseline/ansi_parse` measure the `ansi-to-tui` round trip that this decision removes.
- A color resize costs less than the old grayscale one, because writing straight into the `Buffer` saves more than color adds. The estimate was 1.5 to 2.5 ms and the gate 4 ms, and on the M1 Air it measures under 1 ms. These are from one `make bench` run per machine, on an 843x632 image at 160 columns (results on PR #70):

  | | M1 Air (target) | Ryzen 9700X (dev) |
  | --- | --- | --- |
  | `charaoke/tui_proxy/drag` (render + direct `Buffer` write, 120-200 col drag) | 0.96 ms | 0.54 ms |
  | `charaoke/tui_proxy/drag_ansi` (render + ANSI + `ansi-to-tui` + `Paragraph`) | 1.66 ms | 1.11 ms |
  | Step 2 grayscale `tui_proxy` (ADR 0009) | 0.98 ms | 0.56 ms |
  | `charaoke/render` (with color) | 0.52 ms | 0.32 ms |
  | `charaoke/ansi` truecolor / 256-color | 0.033 / 0.025 ms | 0.026 / 0.023 ms |
  | `charaoke/load` | 0.037 ms | 0.022 ms |
  | `charaoke/end_to_end` (decode + load + render + truecolor ANSI) | 1.91 ms | 1.18 ms |
  | `baseline/color` | 6.36 ms | 4.01 ms |
  | `baseline/ansi_parse` | 1.10 ms | 1.14 ms |
  | `baseline/end_to_end` | 33.8 ms | 21.9 ms |

- Color costs about 0.17 ms in `render` on the Air (0.35 to 0.52 ms), for the extra two channel sums. ANSI output is about 190x faster than the old color stage, and the TUI path also drops `ansi-to-tui`'s 1.1 ms parse, which no bench measured before this change.
- The x86-64 `load` gap from ADR 0009 is gone now that `load` is a copy (0.022 ms on the Ryzen against 0.037 ms on the Air).
- `render`'s signature changed from `&mut String` to `&mut Cells`. `charaoke` is pre-1.0 and has no users outside the workspace, so this is the cheapest time to change it.
- Memory per loaded image rises from 1 to 3 bytes per pixel, about 1.6 MB for 843x632.
- Brightness now comes from each cell's mean color rather than from each pixel. The two agree within 1, and a unit test pins that down.
- The x86-64 vectorization question moves to `render`'s three-channel sums. That investigation follows in its own change.
- Render still sums every pixel of each cell. If that becomes the bottleneck, for example with large images, zoom or panning, a summed-area table built at load would cut each cell to 4 lookups per channel at about 4x the memory. Not planned for now.
