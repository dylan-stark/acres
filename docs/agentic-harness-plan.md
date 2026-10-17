# Agentic development harness for acres

## Context
acres is a single-author Rust workspace (~7k LOC, 5 crates: `acres`, `iiif`, `image-to-ascii-builder`, `acres-cli`, `acres-tui`) for the Art Institute of Chicago API. Branch `agent-harness` is identical to `main`, so no harness work exists yet. Goal: rate the repo for agentic development across five subsystems, then build the harness in small steps.
Ratings come from a read-only survey of the repo. I did not run the test suite.

## Ratings (1 = absent, 5 = agent can work unattended)

| Subsystem | Score | Evidence |
|---|---|---|
| Instruction | **1/5** | No CLAUDE.md/AGENTS.md, no `.claude/`, no architecture or conventions doc. Only README plus good rustdoc in `acres` and `iiif`. README drifts from the code (`image` vs `iiif`, `artworks-manifest` vs `artwork-manifest`, links to nonexistent crate READMEs). |
| Tooling | **3/5** | Makefile gates (`reformat`, `lint`, `test`, `docs`) match CI exactly, which gives a clear definition of done. Missing: one `make check` target, cargo-nextest, MCP config, permission allowlist. `vhs` is not installed locally. |
| Environment | **2/5** | Lockfile committed and tests run `--locked`. No `rust-toolchain.toml`, devcontainer or Dockerfile. `.envrc` uses stale `AIC_TUI_*` names and nonexistent paths. `cd.yml` ships a stale `aic-tui` binary name. No worktree or offline story. |
| State | **1/5** | Only git history and two tags. No CHANGELOG, roadmap, ADRs or issue templates. Stale local branch `take-tui`. Commit style (terse past-tense sentence) is undocumented. |
| Feedback | **2/5** | ~36 tests: CLI integration tests with wiremock, 14 TUI config tests. TUI components/app/render have zero tests. `iiif/image_request.rs` (1172 lines) has 1 test, `ascii.rs` (702 lines) has almost none. No snapshots or coverage. `demo.tape` has no assertions and needs network + vhs. Strict CI is a strength: clippy `-D warnings`, rustdoc `-D warnings`. |

Overall: **~1.8/5**. The build gates are solid. The harness is missing: nothing tells an agent how the project works, the environment isn't pinned, nothing records state, and the TUI can't be verified automatically.

## Incremental plan
Each step is one small PR/commit on `agent-harness`, ordered by leverage per effort. Every step keeps `make check` green.

### Step 1 — Instruction baseline (lowest cost, highest value)
- Add `CLAUDE.md` (and a one-line `AGENTS.md` pointing to it):
  - the commands: `make reformat lint test docs`
  - the crate map and dependency direction (`iiif` and `image-to-ascii-builder` → `acres` → `acres-cli`/`acres-tui`)
  - the commit style
  - never touch `scratch/` or `target/`
  - tests mock the network, so don't hit the live AIC API in tests
  - TUI keybindings live in `acres-tui/.config/config.json5`
- Fix README drift (naming, dead links). Add the `.gitignore` fix for the bitocra `.pdf` vs `.bdf` entry.
- Add a `make check` target (reformat + lint + test + docs) as the single "done" command.

### Step 2 — Tooling and permissions
- `.claude/settings.json`: allow `make *`, `cargo build/test/clippy/fmt/doc/check`, `git status/diff/log`. Deny network tools against the live API and writes to `scratch/`.
- Hook: a PostToolUse hook on `.rs` edits running `cargo fmt` (and optionally `cargo check -q`).
- `.claude/commands/`: `/check` (runs `make check` and summarizes failures) and `/new-crate-test` if useful.
- Optional: install `cargo-nextest` and swap it into the `test` target.

### Step 3 — Environment reproducibility
- Add `rust-toolchain.toml` (pin the stable version, include rustfmt/clippy) and mirror it in CI.
- Fix `.envrc` to use the `ACRES_*` names and real paths. Check what `acres-tui/src/config.rs` actually reads.
- Fix `cd.yml` binary names (`acres-cli`, `acres-tui`) and replace deprecated `actions-rs`.
- Add a devcontainer or Dockerfile with toolchain, `jq`, `vhs`, `ttyd` and `ffmpeg`, for sandboxed unattended runs. Document `git worktree` usage (per-worktree `target/`, or a shared `CARGO_TARGET_DIR`).

### Step 4 — Feedback: make the TUI verifiable
- Add ratatui `TestBackend` render tests for `acres-tui` components plus `insta` snapshots.
- Add unit tests for `iiif/image_request.rs` URL building and `image-to-ascii-builder`. Use tests with existing wiremock fixtures.
- Make `demo.tape` offline: feed it a fixture `artworks.json` checked in under `acres-tui/tests/fixtures/` instead of fetching from the live API. Add a `make demo` target.
- Add `cargo llvm-cov` as an optional `make coverage`, reporting only (no gate yet).

### Step 5 — State and memory
- `CHANGELOG.md` (Keep a Changelog), seeded from tags v0.1.0 and v0.2.0.
- `docs/decisions/` with a short ADR template and 2–3 retroactive ADRs (e.g. builder pattern, JSON-to-stdout CLI design, IIIF split).
- `ROADMAP.md` or GitHub issues for backlog. Prune the stale `take-tui` branch.
- Convention for agents: record non-obvious decisions in an ADR, and update CHANGELOG in the same commit as user-visible changes.

### Step 6 — Closing the loop
- CI additions: a snapshot-review check (`cargo insta test --check`), and optionally a Claude Code review job on PRs.
- Review subagents or skills in `.claude/` (e.g. `reviewer` that runs `make check` and diffs against CLAUDE.md conventions).
- Re-rate the five subsystems. Target ≥3 on every row.

## Critical files
- New: `CLAUDE.md`, `AGENTS.md`, `.claude/settings.json`, `.claude/commands/*`, `rust-toolchain.toml`, `CHANGELOG.md`, `docs/decisions/*`, `acres-tui/tests/fixtures/*`
- Edit: `Makefile` (add `check`, `demo`, `coverage`), `.envrc`, `.gitignore`, `README.md`, `.github/workflows/{ci,cd}.yml`, `demo.tape`
- Reuse: existing wiremock setups in `acres-cli/tests/*.rs` and `acres/src/**` tests for fixtures; `acres-tui/src/config.rs` tests as the pattern for TUI unit tests.

## Verification
- After each step: `make check` passes locally and CI is green on the PR.
- Step 1: start a fresh Claude Code session and ask for a trivial change. It should find and run `make check` unprompted.
- Step 2: confirm no permission prompts for routine cargo/make commands and that the fmt hook fires on an `.rs` edit.
- Step 3: `rustc --version` matches the pin, and a clean devcontainer build runs `make check`.
- Step 4: `cargo test` runs the new snapshot tests offline, and `make demo` produces a GIF without network.
- Step 6: re-rate against the table above.

## Re-rating after Step 6

Re-rated from the repo state after Harness 1–6 (read-only survey plus the CI/agent files added in step 6; I did not observe a green CI run on the new workflows yet).

| Subsystem | Before | After | Evidence |
|---|---|---|---|
| Instruction | 1 | **4** | `CLAUDE.md` (+ `AGENTS.md`) covers commands, crate map, dependency direction, commit/branch/PR workflow and test rules; README drift fixed. |
| Tooling | 3 | **4** | `make check` as single done command, `.claude/settings.json` allowlist and fmt hook, `/check` command, `reviewer` subagent. No nextest or MCP config. |
| Environment | 2 | **4** | `rust-toolchain.toml` mirrored in CI with a `check-toolchain` guard, fixed `.envrc`, Dockerfile with `vhs`/`ttyd`/`ffmpeg`, documented worktree use. |
| State | 1 | **3** | `CHANGELOG.md`, ADRs 0001–0003 with a template, recorded conventions for both. No roadmap beyond GitHub issues. |
| Feedback | 2 | **3** | `insta` render snapshots for TUI components, enforced in CI by `cargo insta test --check`; offline `make demo`; report-only `make coverage`. Coverage is ungated and `ascii.rs` is still thinly tested. |

Overall: **~3.6/5**, every row ≥3.
