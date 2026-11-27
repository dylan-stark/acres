# 0020. Log levels, fields and destinations

- Status: Accepted
- Date: 2026-11-27

## Context

Logging was noisy where it didn't matter and quiet where it did. The `acres` search and
collection builders logged every setter at INFO, so each TUI search wrote a burst of
"Setting q" lines. Meanwhile no request was ever logged: `Catalog` measured each request's wait
and fetch time but emitted nothing. `acres-cli` used `RUST_LOG` with no default, which means
ERROR only, so warnings such as "shared rate limit unavailable" never reached anyone. It had no
`-v` flag either. The TUI overwrote its log on every start, which lost the log of a run that had
crashed, and it logged a missing config file, the normal case, at ERROR. No crate had a span.

## Decision

The current conventions, which may have grown since, are in [`docs/LOGGING.md`](../LOGGING.md).

- **Levels.**
  - ERROR is for failures the app itself can't report any other way.
  - WARN is for degraded but continuing work the caller can't otherwise see, such as a shared
    rate limit that can't be used or a response that can't be cached.
  - INFO is for notable state changes: retries, startup.
  - DEBUG is for one line per unit of work: a request's outcome, a cache hit.
  - TRACE is for detail inside a unit, such as each HTTP response in a retry loop.
- **Libraries return errors and don't log them above DEBUG.** The caller reports each failure
  once (ADR 0019).
- **One span and one closing event per request.** `Catalog::raw` runs in a DEBUG `request`
  span with `endpoint` and `resource`. It ends with a DEBUG `fetched` event carrying
  `from_cache`, `wait_ms`, `fetch_ms` and `bytes`, or with `request failed` carrying the error's
  `kind`. Durations are whole milliseconds in fields named `*_ms`.
- **CLI.** Logs go to stderr at WARN by default.
  - `-v` adds INFO and each request's closing event.
  - `-vv` turns on DEBUG everywhere, and `-vvv` turns on TRACE.
  - `-q` turns logging off. Errors are still reported, since they don't go through the log.
  - `RUST_LOG` overrides the flags when set.
  - Colors are used only when stderr is a terminal and `NO_COLOR` is unset.
- **TUI.** `acres-tui.log` in the data directory is appended to. It is moved aside to
  `acres-tui.log.1` once it passes 5 MB, and each run starts with an `acres-tui started` line
  giving the version and pid. INFO is the default level, set by `ACRES_TUI_LOGLEVEL` or
  `RUST_LOG`. Image fetches and cache reads run in `image` and `cached_image` spans keyed by
  artwork id.

- **Span timings at TRACE.** When the filter allows TRACE, each span logs a `close` line with
  `time.busy` and `time.idle`.

## Consequences

- `acres-cli -v` shows what was fetched, from where and how long it took, which helps both
  debugging and agents.
- The TUI log keeps about two runs of history, and its default INFO level is quiet now that the
  setter logs are gone.
- Field names (`endpoint`, `kind`, `*_ms`) are worth keeping stable for anyone reading logs with
  a program, though they are not a contract like the CLI's exit codes.
