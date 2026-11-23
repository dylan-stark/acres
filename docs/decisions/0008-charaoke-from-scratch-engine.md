# 0008. Build `charaoke`, a from-scratch ASCII engine, with an 8 ms frame budget

- Status: Proposed
- Date: 2026-11-23

## Context

`acres-tui` converts artwork images to ASCII through `image-to-ascii-builder`, which wraps the third-party `image-to-ascii` crate (ADR 0001). We want the TUI to re-render the image live as the terminal is resized, and later to pan across large images.

Ratatui draws at 60 FPS by default, a 16.7 ms frame. The baseline benchmarks (`make bench`, `image-to-ascii-builder/benches/baseline.rs`) show the current path is far over that on every machine we measured. The input is the fixture artwork upscaled to the 843 px width the TUI fetches (843x632), with the default settings (`bitocra-13` font, `alphabet` alphabet, `edge-augmented` algorithm). Times are criterion's mean estimate from one `make bench` run per machine (results on PR #67):

| End to end (`Ascii::builder().build()`) | 80 cols | 160 cols | 320 cols |
| --- | --- | --- | --- |
| M1 MacBook Air, 16 GB (target machine) | 24.2 ms | 36.8 ms | 83.8 ms |
| AMD Ryzen 9700X @ 5.58 GHz (main development machine) | 12.7 ms | 21.7 ms | 61.5 ms |
| 4 vCPU Intel Xeon @ 2.10 GHz cloud VM (noisy, for reference) | 30 ms | 50 ms | 145 ms |

Stage by stage, at 160 cols on the same image:

| Stage | M1 Air | Ryzen 9700X |
| --- | --- | --- |
| JPEG decode (`image` 0.24) | 3.35 ms | 1.35 ms |
| To `f32` luma image | 1.53 ms | 1.18 ms |
| Parse BDF font | 1.23 ms | 0.87 ms |
| Conversion, `edge-augmented` | 22.6 ms | 14.0 ms |
| Conversion, `base` | 6.7 ms | 4.9 ms |
| ANSI coloring | 6.8 ms | 3.9 ms |

Decode, luma and font are fixed costs paid again on every call, 6.1 ms on the M1 Air before any conversion. Conversion grows faster than the cell count, because `image-to-ascii` resizes to the full glyph resolution (one font-sized pixel block per cell) and scores every glyph in the alphabet against every cell. ANSI coloring alone nearly fills the budget on the M1 Air, so it is a stage to optimize, not an afterthought. The crate also pins `image` to 0.24, and its API leaves no room to reuse buffers or cache work between frames.

`charaoke/benches/stages.rs` measures the stages `charaoke` will own using `image` 0.25. On the 843x632 image, decode takes 1.36 ms on the M1 Air and 0.79 ms on the Ryzen, and `to_luma8` takes 0.24 ms and 0.90 ms. Moving to `image` 0.25 alone cuts decode by 40-60%. The luma gap between machines (the M1 is 3.8x faster) suggests the x86 build is not vectorizing that loop, which step 3's SIMD work should look at.

## Decision

Build a new workspace crate, `charaoke`, from scratch with no workspace dependencies. Its target is to convert a full TUI pane on resize in 8 ms or less on Dylan's M1 MacBook Air, half the 60 FPS frame, so drawing and app work fit in the rest. Benchmarks exist before any optimization: `charaoke/benches/stages.rs` measures each stage on the same inputs and widths as the baseline, so the two reports line up.

## Consequences

- We own the hot path and can explore buffer reuse, caching across resizes, tiling for panning, multithreading and SIMD, each backed by benchmark numbers.
- We start without `image-to-ascii`'s glyph-matching algorithms and metrics. Each one comes back only once its cost per cell has been measured against the budget.
- Until the TUI and CLI move over, the workspace carries two ASCII paths and two `image` versions (0.24 via `image-to-ascii`, 0.25 in `charaoke`'s benchmarks).
- The target is judged on the M1 Air. Day-to-day regressions are tracked on the Ryzen with `make bench-save` and `make bench-compare`, and the cloud VM is too noisy for either.
