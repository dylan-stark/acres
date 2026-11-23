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

- A resize costs one `render`. On the cloud VM that is 0.52 ms for an 843x632 image at 160 columns, against 13 ms for the cheapest `image-to-ascii` conversion. A headless Ratatui resize (`charaoke/tui_proxy`) takes 1.5 ms. M1 Air and Ryzen numbers still to come.
- Decode now dominates the end-to-end cost (about 2.2 of 3.3 ms on the VM). It's paid once per image, not per frame.
- Colour must come back before the TUI switches over (step 4). It will be measured as its own stage, since ANSI colouring cost 6.8 ms on the M1 Air in the baseline.
- Area averaging ignores glyph shape. Edge-aware algorithms come back only as measured, optional stages.
