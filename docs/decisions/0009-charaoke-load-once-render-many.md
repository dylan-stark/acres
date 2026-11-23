# 0009. `charaoke` loads an image once and renders it many times

- Status: Proposed
- Date: 2026-11-23

## Context

ADR 0008 sets an 8 ms budget for re-rendering a TUI pane on resize. The `image-to-ascii` path redoes everything on every call, including decode, brightness, font parsing and conversion, and allocates fresh buffers each time. But a resize changes only the grid. The image and its brightness stay the same.

## Decision

`charaoke::Converter` splits the work in two:

- `set_image` takes raw pixels (`Luma8`, `Rgb8` or `Rgba8`), not encoded bytes. It reduces them once to an 8-bit brightness plane with integer BT.601 weights.
- `render(grid, &mut String)` area-averages that plane onto the grid's cells with integer sums and picks each character from a 256-entry `Ramp` table. It writes into a caller-owned `String` and keeps its scratch buffers, so a resize allocates nothing once the buffers have grown.

Decoding stays with the caller, so `charaoke` keeps no runtime dependencies. The first version has no colour, glyph matching or metrics.

## Consequences

- A resize costs one `render`. Measured on an 843x632 image at 160 columns, one `make bench` run per machine (results on PR #68):

  | | M1 Air (target) | Ryzen 9700X (dev) |
  | --- | --- | --- |
  | `charaoke/render` (per resize) | 0.35 ms | 0.21 ms |
  | `charaoke/tui_proxy` (render + Ratatui draw, 120-200 col drag) | 0.98 ms | 0.56 ms |
  | `charaoke/end_to_end` (decode + load + render) | 1.72 ms | 1.18 ms |
  | `charaoke/load` (brightness plane, once per image) | 0.03 ms | 0.16 ms |
  | `baseline/convert/base` | 6.2 ms | 4.9 ms |
  | `baseline/convert/edge-augmented` | 21.7 ms | 13.8 ms |
  | `baseline/end_to_end` (includes colour) | 34.1 ms | 21.7 ms |

  A resize on the M1 Air uses about 1 ms of the 8 ms budget, so the target is met for grayscale, with about 7 ms left over.
- Decode now dominates the end-to-end cost (1.31 of 1.72 ms on the M1 Air). It's paid once per image, not per frame.
- `load` runs about 5x faster on the M1 Air than on the Ryzen, the same gap as `image`'s `to_luma8` in ADR 0008. The brightness loop is likely vectorised on aarch64 but not on default x86-64. That's for step 3's SIMD work.
- Colour must come back before the TUI switches over (step 4). It will be measured as its own stage, since ANSI colouring takes 6.4 ms on the M1 Air in the baseline, nearly all of the remaining budget.
- Area averaging ignores glyph shape. Edge-aware algorithms come back only as measured, optional stages.
