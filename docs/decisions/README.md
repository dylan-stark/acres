# Architecture decision records

Short records of non-obvious decisions, so a later reader (human or agent) knows why the code is shaped the way it is.

- Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md` using the next number.
- Keep it to one page. Never edit an accepted ADR's decision; add a new ADR that supersedes it and update the old one's status.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-builder-pattern-for-ascii.md) | Builder pattern for ASCII conversion | Accepted |
| [0002](0002-cli-json-to-stdout.md) | CLI writes JSON to stdout | Accepted |
| [0003](0003-split-iiif-crate.md) | Split IIIF into its own crate | Accepted |
| [0004](0004-iiif-serde-as-strings.md) | Serialize IIIF types as strings, behind a feature | Proposed |
| [0005](0005-iiif-no-public-url.md) | Keep `url` out of the `iiif` public API | Accepted |
| [0006](0006-iiif-error-naming.md) | Name and layer `iiif` errors by what is wrong with the input | Accepted |
| [0007](0007-iiif-builder-typestate-only-uri.md) | Typestate only the required part of the `iiif` image request builder | Accepted |
| [0008](0008-charaoke-from-scratch-engine.md) | Build `charaoke`, a from-scratch ASCII engine, with an 8 ms frame budget | Proposed |
| [0009](0009-charaoke-load-once-render-many.md) | `charaoke` loads an image once and renders it many times | Proposed |
| [0010](0010-charaoke-color.md) | `charaoke` renders colored cells and writes ANSI separately | Accepted |
| [0011](0011-switch-to-charaoke.md) | Switch the TUI and CLI to `charaoke` | Accepted |
| [0012](0012-charaoke-edge-glyphs.md) | `charaoke` does not add edge-aware glyphs | Rejected |
| [0013](0013-tui-catalog-explorer.md) | TUI overhaul into a catalog explorer | Accepted |
| [0014](0014-charaoke-block-modes.md) | `charaoke` adds half-block and quadrant modes | Accepted |
| [0016](0016-acres-catalog-client.md) | `acres` catalog client | Accepted |
| [0017](0017-tui-zoom-and-pan.md) | Zoom and pan in the art pane | Accepted |
| [0018](0018-tui-browse-cached-images.md) | Browse cached images instead of placeholders | Accepted |
