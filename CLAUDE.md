# acres

Rust workspace of tools for the [Art Institute of Chicago API](https://api.artic.edu/docs/). See `README.md` for usage.

## Commands

- `make check` — the single "done" command: reformat check, clippy (`-D warnings`), tests, docs. Run it before finishing any change.
- Individual steps: `make reformat` (`cargo fmt --check`; run `cargo fmt --all` to fix), `make lint`, `make test`, `make docs`.

## Crates and dependency direction

- `iiif` — IIIF image URL construction (no workspace deps)
- `image-to-ascii-builder` — builder wrapper around `image-to-ascii` (no workspace deps)
- `acres` — AIC API library; depends on `iiif`
- `acres-cli` — Clap CLI; depends on `acres`, `iiif`, `image-to-ascii-builder`
- `acres-tui` — Ratatui TUI; depends on `acres`, `iiif`, `image-to-ascii-builder`

Dependencies only point downward: `iiif`/`image-to-ascii-builder` → `acres` → `acres-cli`/`acres-tui`. Never add an upward dependency.

## Conventions

- Commit messages: one terse past-tense sentence ending in a period, e.g. `Fixes filenames.`
- Never read, modify, or commit `scratch/` or `target/`.
- Tests must mock the network. Never hit the live AIC API from tests.
- TUI keybindings live in `acres-tui/.config/config.json5`.
