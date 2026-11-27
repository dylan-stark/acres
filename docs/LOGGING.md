# Logging

What gets logged, at which level and with which fields, and where each binary writes its log.
The reasoning behind the main choices is in [ADR 0020](decisions/0020-log-levels-and-fields.md).
Errors themselves are covered in [ERROR-HANDLING.md](ERROR-HANDLING.md).

## Levels

| Level | Use it for | Examples |
| --- | --- | --- |
| ERROR | A failure the app can't report any other way | The TUI can't restore the terminal while shutting down |
| WARN | Degraded but continuing work the caller can't otherwise see | The shared rate limit can't be used, so it is per process. A response can't be cached |
| INFO | Notable state changes | A retry after a 429, the TUI starting |
| DEBUG | One line per unit of work | A request's `fetched` / `request failed` event, a cache hit |
| TRACE | Detail inside a unit of work | Each HTTP response in a retry loop, a built image URI |

- **Libraries return errors. They don't log them above DEBUG.** The caller reports each failure
  once: the CLI on stderr, the TUI in its status bar or log. A library logging and returning the
  same error makes it show up twice.
- **Don't log setters or arguments as they arrive.** Log the unit of work once, when it ends, with
  what it was and how it went.
- WARN must be rare enough that the CLI can show it by default without being noisy.

## Spans and events

- **Wrap a unit of work in a span named for it,** with the fields that identify it:
  `request` (`endpoint`, `resource`), `image` and `cached_image` (`id`). Spans are DEBUG, so
  they cost nothing at the default levels.
- **End a unit of work with exactly one event:** `fetched` on success, `request failed` on
  failure. Inside a span, the event doesn't need to repeat the span's fields.
- **The message is a short lowercase phrase,** with no trailing punctuation and no interpolated
  values: `retrying`, not `Retrying in 2s after 429`. Values go in fields.
- In async code, attach spans with `.instrument(span)`. Don't hold a `span.enter()` guard across
  an `.await`.
- At TRACE, both binaries also log a `close` line for each span with its `time.busy` and
  `time.idle`, so `acres-cli -vvv` or `ACRES_TUI_LOGLEVEL=trace` shows how long each request or
  image load took end to end.

## Fields

- Use snake_case names, and keep them stable. Programs and agents read logs too, though fields
  aren't a contract the way the CLI's exit codes are.
- `endpoint` is the URL requested.
- `kind` is the error's `kind()` on any failure event (see ERROR-HANDLING.md).
- Durations are whole milliseconds in `*_ms` fields: `wait_ms`, `fetch_ms`, `back_off_ms`.
- Sizes are `bytes`. Counts and ids are plain integers (`attempt`, `id`, `status`).
- Use `%` (Display) for URLs, paths and errors, and `?` (Debug) only for enums and small structs.
- Never log secrets, or whole response bodies above TRACE.

## Where logs go

### `acres-cli`

- Logs go to stderr. stdout only ever carries data.
- The default level is WARN.
  - `-v` adds INFO and each request's closing event.
  - `-vv` turns on DEBUG, and `-vvv` turns on TRACE.
  - `-q` turns logging off. Errors are still reported, since they don't go through the log.
- `RUST_LOG` overrides the flags when set.
- Colors are used only when stderr is a terminal and `NO_COLOR` is unset.

### `acres-tui`

- Logs go to `acres-tui.log` in the data directory. `acres-tui --version` prints where that is.
- The file is appended to across runs. It is moved to `acres-tui.log.1` once it passes 5 MB.
- Each run starts with an `acres-tui started` line giving the version and pid.
- The default level is INFO. `ACRES_TUI_LOGLEVEL` or `RUST_LOG` changes it.
- `L` shows the in-app log drawer. That is a separate list of messages for the user, not the
  log file.

## Tests

- Test a new closing event or field when programs depend on it. `acres/tests/logging.rs` collects
  events with a small `tracing_subscriber::Layer`.
- Put such tests in their own test binary. `tracing` caches which callsites are enabled per
  process, so other tests that log with no subscriber can switch the events off.
