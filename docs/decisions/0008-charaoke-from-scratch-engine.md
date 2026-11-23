# 0008. Build `charaoke`, a from-scratch ASCII engine, with an 8 ms frame budget

- Status: Proposed
- Date: 2026-11-23

## Context

`acres-tui` converts artwork images to ASCII through `image-to-ascii-builder`, which wraps the third-party `image-to-ascii` crate (ADR 0001). We want the TUI to re-render the image live as the terminal is resized, and later to pan across large images.

Ratatui draws at 60 FPS by default, a 16.7 ms frame. The baseline benchmarks (`make bench`, `image-to-ascii-builder/benches/baseline.rs`) show the current path is far over that. These numbers come from one run on a 4 vCPU Intel Xeon @ 2.10 GHz cloud VM, so they show orders of magnitude, not exact figures. The image is the fixture artwork upscaled to the 843 px width the TUI fetches, with the default settings (`bitocra-13` font, `alphabet` alphabet, `edge-augmented` algorithm):

| Stage, 843x632 image | 80 cols | 160 cols | 320 cols |
| --- | --- | --- | --- |
| End to end (`Ascii::builder().build()`) | 30 ms | 50 ms | 145 ms |
| Conversion only, `edge-augmented` | 19 ms | 39 ms | 125 ms |
| Conversion only, `base` | 3.3 ms | 13 ms | 75 ms |
| ANSI colouring | 0.4 ms | 1.5 ms | 5.9 ms |

The fixed costs on every call are JPEG decode (4.3 ms), conversion to an `f32` luma image (2.7 ms) and parsing the BDF font (2.0 ms). The cost of converting grows faster than the cell count, because `image-to-ascii` resizes to the full glyph resolution (one font-sized pixel block per cell) and scores every glyph in the alphabet against every cell. The crate also pins `image` to 0.24, and its API leaves no room to reuse buffers or cache work between frames.

## Decision

Build a new workspace crate, `charaoke`, from scratch with no workspace dependencies. Its target is to convert a full TUI pane on resize in 8 ms or less on Dylan's M1 MacBook Air, half the 60 FPS frame, so drawing and app work fit in the rest. Benchmarks exist before any optimisation: `charaoke/benches/stages.rs` measures each stage on the same inputs and widths as the baseline, so the two reports line up.

## Consequences

- We own the hot path and can explore buffer reuse, caching across resizes, tiling for panning, multithreading and SIMD, each backed by benchmark numbers.
- We start without `image-to-ascii`'s glyph-matching algorithms and metrics. Each one comes back only once its cost per cell has been measured against the budget.
- Until the TUI and CLI move over, the workspace carries two ASCII paths and two `image` versions (0.24 via `image-to-ascii`, 0.25 in `charaoke`'s benchmarks).
- Baseline numbers need re-measuring on the M1 Air before they count against the target. Cloud VM numbers are only good for relative comparisons within one run.
