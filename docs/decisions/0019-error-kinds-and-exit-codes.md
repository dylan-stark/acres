# 0019. Error kinds, hints and CLI exit codes for people and agents

- Status: Accepted
- Date: 2026-11-27

## Context

Errors were readable by people but not by programs. `acres-cli` printed a `color-eyre` report
with a source location and a backtrace hint, and every failure exited with 1. A caller had to
parse the English to tell "not cached while offline" from "no such artwork" from "rate limited,
try again". Agents running `acres-cli` are a target user, and they need a stable way to branch on
the failure and a pointer to the fix. Some failures also had no message at all. A bad `ACRES_*`
value was silently ignored, and a typo in a TUI keybinding panicked.

## Decision

- **Kinds.** Every error in `iiif`, `acres` and `charaoke` has `kind() -> &'static str`: a dotted,
  snake_case name, layer first (`cache.missing`, `iiif.parameter.invalid_region`). Kinds are API:
  never renamed or reused. A 404 is its own kind, `response.not_found`, since callers branch on it.
- **Hints.** `acres` errors have `hint() -> Option<&'static str>`: one imperative sentence on what
  to do next, naming the variable or flag to change. `ResponseError::is_transient` says whether
  trying again may work.
- **Exit codes by class,** listed in `acres-cli --help`: 1 other, 2 usage, 3 not found, 4 offline
  miss, 5 transient, 6 invalid input, 7 environment, 8 bad response. A closed stdout (`| head`)
  exits 0 quietly.
- **Error format.** Text by default (`error:`, `caused by:`, `hint:`), or with
  `--error-format json` / `ACRES_ERROR_FORMAT=json` one JSON line on stderr:
  `{"kind", "message", "causes", "hint", "exit_code"}`. The flag is read before clap parses, so
  usage errors come out in the chosen format too. The full `eyre` report shows only with
  `RUST_BACKTRACE`.
- **Config errors surface.** `CatalogBuilder::build` returns `ConfigError` (`config.invalid`) for
  an unreadable `ACRES_*` environment instead of quietly using defaults.

Conventions for writing them are in [`docs/ERROR-HANDLING.md`](../ERROR-HANDLING.md).

## Consequences

- Programs and agents can match on `kind` and the exit code without parsing messages, and messages
  stay free to improve.
- Kinds and codes are now a compatibility surface. Adding is cheap, but renaming means a new
  kind and a changelog entry.
- Every new error variant needs a kind (the compiler enforces this through exhaustive matches) and
  a test that pins it.
- The CLI keeps `color-eyre` for panics and `RUST_BACKTRACE` debugging, but no longer uses its
  report for ordinary failures.
