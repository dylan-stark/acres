# Contributing

## Setup

Install Rust with [rustup](https://rustup.rs/). The toolchain is pinned in `rust-toolchain.toml`, so rustup picks the right version on the first build, and CI uses the same one.

```sh
make check
```

`make check` is the one command a change must pass: it checks formatting, runs clippy with warnings as errors, runs the tests and builds the docs. Its steps also run on their own: `make reformat`, `make lint`, `make test` and `make docs`.

[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) has a tip for re-running tests on every save.

## Tests

- Tests never call the live API. Mock it with `wiremock`, as the existing tests do.
- TUI rendering is tested with [`insta`](https://insta.rs/) snapshots. After an intended UI change, run `cargo insta review` (or `INSTA_UPDATE=always cargo test -p acres-tui`) and commit the changed `.snap` files. CI also rejects snapshots that no test uses.
- `make bench` runs the criterion benchmarks, and `make bench-save` and `make bench-compare` compare a change against a saved baseline. They aren't part of `make check`, but run them for any change to `charaoke` or the TUI's drawing.

## Workflow

- Work on a branch off `main` and open a pull request against `main`. CI runs the same checks as `make check`.
- Write commit messages as one short past-tense sentence ending in a period, such as `Fixed the search cursor.`
- Add a line to `CHANGELOG.md` under `[Unreleased]` in the same commit as any change a user would notice.
- Record a decision that isn't obvious from the code as an ADR in [`docs/decisions/`](docs/decisions/).
- Ideas that come up mid-change but are out of its scope go in a GitHub issue labeled `follow-up`.
- Spell in American English.

## Crates and conventions

The libraries never depend on the apps:

- `iiif` and `charaoke` depend on nothing in the workspace, and `acres` only on `iiif`.
- `acres-report` depends on `acres`, `iiif` and `charaoke`.
- `acres-cli` and `acres-tui` depend on the libraries and `acres-report`, never on each other.

Each cross-cutting topic has a guide with its current conventions:

- [`docs/ERROR-HANDLING.md`](docs/ERROR-HANDLING.md) for building, naming and reporting errors.
- [`docs/LOGGING.md`](docs/LOGGING.md) for log levels, spans and fields.

TUI keys are bound in [`acres-tui/.config/config.json5`](acres-tui/.config/config.json5).

## Releases

[`docs/RELEASING.md`](docs/RELEASING.md) lists the steps for cutting a release.
