# 0021. Error reporting shared by the CLI and TUI

- Status: Accepted
- Date: 2026-11-27

## Context

ADR 0019 gave `acres-cli` a kind, a hint and an exit code for each failure, and wrote them as
text or JSON. That logic lived in `acres-cli/src/report.rs`, so the TUI had none of it. Its
failures were bare strings: a failed image fetch reached only the log drawer (`L`), no failure
showed a hint, the log had no `kind` to search by, and a bad config file at startup printed a
full `color-eyre` report and exited with 1.

We considered three ways to share the logic.

- **The TUI depends on `acres-cli`, or runs it.** The CLI is a binary with its own policy, such
  as clap usage errors and stdout handling. Depending on it would tie the TUI to the CLI's
  release, and spawning it would add a process per request for no gain.
- **Put classification in `acres`.** `acres` already knows each of its errors' kind and hint.
  But which exit code a class of failure gets, and how it is written, is an application's
  choice, and `acres` would also need to know `charaoke` and `image` errors it doesn't depend on.
- **A small internal crate the apps share.** It holds the app policy once, and each app adds
  only what is its own.

## Decision

The current conventions, which may have grown since, are in
[`docs/ERROR-HANDLING.md`](../ERROR-HANDLING.md).

- **`acres-report`** is a crate shared by the apps that depends on `acres`, `iiif`
  and `charaoke`. It holds the exit codes, the `Failure` type and its classification from an
  error chain, and the text and JSON renderers. `acres-cli` and `acres-tui` both depend on it and
  never on each other. `acres` keeps only its own errors' facts: `kind`, `hint` and
  `is_transient`.
- **Apps add their own errors** with `classify_with`: the CLI and TUI for `clap_stdin`, the TUI
  for its config file.
- **TUI failures carry a `Failure`, not a string.** A failure of something the user asked for
  shows in the status bar with its hint until the next key, or in a popup over the art in zen,
  and `!` shows the last one again. Every failure is logged once at WARN
  with its `kind`.
- **The `L` log drawer is removed.** It showed a second, partial copy of the log. With failures
  in the status bar or the zen popup, and `!` to see the last one, the log file is enough.
- **A failure that stops the TUI is written as the CLI writes it**, on stderr once the terminal
  is restored, with the CLI's exit code: 7 for a bad config file or keybinding.

## Consequences

- The CLI's JSON and exit codes are unchanged. A new kind or exit code is added in one place and
  both apps get it.
- A long failure is cut at the edge of the status bar. The zen popup wraps it whole, and the log
  always has it.
- `acres-report` is one more crate to keep in the dependency rules. It is publish-ready, so the
  apps can go on crates.io, but it exists for the apps and its API changes with them.
