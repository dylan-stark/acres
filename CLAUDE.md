# acres

Rust workspace of tools for the [Art Institute of Chicago API](https://api.artic.edu/docs/). See `README.md` for usage.

## Commands

- `make check` — the single "done" command: reformat check, clippy (`-D warnings`), tests, docs. Run it before finishing any change.
- The Rust toolchain is pinned in `rust-toolchain.toml` (rustup required) and CI uses the same version. To bump Rust, change both `rust-toolchain.toml` and the `dtolnay/rust-toolchain@<version>` refs in `.github/workflows/ci.yml` and `snapshots.yml`. A weekly canary workflow runs clippy on stable/beta to flag upcoming breakage.
- `make demo` — records `demo.gif` offline (needs `vhs`, `ttyd` and Chromium; `make docker-demo` runs it in the image): serves `acres-tui/tests/fixtures/` images on `127.0.0.1:8099` and runs `demo.tape` against `acres-tui/tests/fixtures/artworks.json`.
- `make coverage` — report-only `cargo llvm-cov` summary (needs `cargo install cargo-llvm-cov`).
- TUI render tests are `insta` snapshots (`acres-tui/src/components/snapshots/`). After an intended UI change run `cargo insta review` (or `INSTA_UPDATE=always cargo test -p acres-tui`) and commit the `.snap` changes.
- CI also runs `cargo insta test --check` (`snapshots.yml`) and an optional Claude review job (`claude-review.yml`, enabled by repo variable `CLAUDE_REVIEW=true` plus the `ANTHROPIC_API_KEY` secret). The `.claude/agents/reviewer.md` subagent checks a diff against these conventions.
- Individual steps: `make reformat-check` (`cargo fmt --check`), `make reformat` (applies formatting), `make lint`, `make test`, `make docs`.

## Crates and dependency direction

- `iiif` — IIIF image URL construction (no workspace deps)
- `image-to-ascii-builder` — builder wrapper around `image-to-ascii` (no workspace deps)
- `acres` — AIC API library; depends on `iiif`
- `acres-cli` — Clap CLI; depends on `acres`, `iiif`, `image-to-ascii-builder`
- `acres-tui` — Ratatui TUI; depends on `acres`, `iiif`, `image-to-ascii-builder`

Library crates never depend on their consumers: `iiif` and `image-to-ascii-builder` depend on nothing in the workspace, `acres` may depend only on `iiif`, and `acres-cli`/`acres-tui` may depend on `acres`, `iiif` and `image-to-ascii-builder`. Never add a dependency in the other direction.

## Conventions

- Commit messages: one terse past-tense sentence ending in a period, e.g. `Fixes filenames.`
- Workflow: never commit directly to `agent-harness` or `main`. Do each piece of work on its own branch off the integration branch (`agent-harness` for harness issues), push it, and open a PR against that branch with `gh pr create` (body says `Closes #N`). `make check` must pass first. Don't merge your own PR.
- Record non-obvious decisions as an ADR in `docs/decisions/` (copy `0000-template.md`, add it to the index in that folder's `README.md`).
- Update `CHANGELOG.md` (under `[Unreleased]`, Keep a Changelog format) in the same commit as any user-visible change.
- Never read, modify, or commit `scratch/` or `target/`.
- Tests must mock the network. Never hit the live AIC API from tests.
- TUI keybindings live in `acres-tui/.config/config.json5`.

## Environment

- `.envrc` (direnv) sets `ACRES_TUI_CONFIG`, `ACRES_TUI_DATA` and `ACRES_TUI_LOGLEVEL`, the variables `acres-tui` reads.
- `Dockerfile` provides the pinned toolchain plus `jq`, `vhs`, `ttyd` and `ffmpeg`: `make docker-check` builds it and runs `make check` inside, using `target-docker/` so it does not clash with host `target/`.
- Parallel work: `git worktree add ../acres-<branch> -b <branch> agent-harness`. Each worktree gets its own `target/` (isolated, slower first build). To share build artifacts, set `CARGO_TARGET_DIR=~/.cache/acres-target`; cargo locks the directory, so concurrent builds serialize.
