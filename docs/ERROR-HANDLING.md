# Error handling

How errors are built, named, worded and reported across the workspace.
The reasoning behind the main choices is in [ADR 0006](decisions/0006-iiif-error-naming.md) (naming and layers) and [ADR 0019](decisions/0019-error-kinds-and-exit-codes.md) (kinds, hints and exit codes).
What gets logged about errors, and at which level, is in [LOGGING.md](LOGGING.md).

## Libraries (`iiif`, `acres`, `charaoke`)

- **Use one enum per layer, built with `thiserror`.** `iiif` has `BaseUriError`, `ParameterError`
  and `RequestError`. `acres` has `ConfigError`, `RequestError`, `ResponseError`, `CacheError` and
  `ParseError`. Each crate also has an umbrella `Error` that wraps them all with
  `#[error(transparent)]`, so callers who don't care about the layer can use one type.
- **Mark every public error enum `#[non_exhaustive]`.**
- **Name variants for what is wrong with the input.** Use `Invalid` (present but malformed),
  `Missing` (required but absent), `Unexpected` (present but not allowed) or `OutOfRange` (parses,
  but the value isn't allowed), or a more exact phrase where one exists (`TrailingSlash`). Don't
  repeat the type's name (`BaseUriError::InvalidChar`, not `InvalidUriChar`).
- **Keep dependency types out of the API.** Hold a third-party cause as
  `Box<dyn Error + Send + Sync>` (`acres::BoxError`) behind `#[source]`.
- **Never return `anyhow`, `eyre` or `String` errors from a library.**

### Messages

- Messages are lowercase, with no trailing punctuation. Name the offending input:
  `invalid region: nope`, `404 from <url>: Not found: …`.
- An error with a `source()` leaves the source out of its own message. Reporters (`eyre`, the CLI)
  print the chain, so repeating it prints the cause twice.
- Say what is wrong, not what the code was doing: `not cached while offline: <url>`, not
  `failed to fetch`.

### Kinds

Every error has a `kind()`: a stable, dotted, snake_case name such as `cache.missing`,
`response.not_found` or `iiif.parameter.invalid_region`.

- The first segment is the layer, or `iiif` or `charaoke` for those crates. The last segment is
  the variant name in snake_case.
- Kinds are part of the API. Never rename a kind or reuse one for something else. A new variant
  gets a new kind.
- A wrapper takes its inner error's kind, so `acres::RequestError::Iiif` reports the `iiif` kind.
- Add a test that pins any new kind.

### Hints

`acres` errors can have a `hint()`: one short sentence on what to do next, for when the message
alone doesn't say it. For example, `cache.missing` says to go online.

- Write hints for the person or agent running the tool, in the imperative: "check the id",
  "wait a minute and try again".
- Leave a hint out (`None`) rather than restating the message.
- Name settings by their environment variable or flag (`ACRES_CACHE_DIR`, `--offline`), so the
  fix can be acted on directly.

### Panics

- Panic only on invariants the code itself guarantees, and use `expect` with a reason stating the
  invariant: `expect("presets are valid")`. Never use a bare `unwrap()` outside tests.
- **User input, config files, environment variables, file contents and API responses never
  panic.** Return an error that names the bad value and where it came from.
- Never panic in `Drop`. A panic during an unwind aborts the process and hides the first error.
  Log it and carry on.

## Binaries (`acres-cli`, `acres-tui`)

- Use `color_eyre::Result` and add context with `wrap_err` / `with_context` where it helps the
  reader: `could not open artworks.json`. Context is lowercase, says what was being done to what,
  and doesn't repeat the cause.
- Prefer a library error with a kind over a new string. If the CLI meets a new class of failure,
  give it a kind in `acres-cli/src/report.rs`.

### CLI reporting

`acres-cli` reports a failure on stderr. stdout only ever carries data.

- **Text** (the default):

  ```text
  error: could not open artworks.json
    caused by: No such file or directory (os error 2)
  hint: check the input path, or pass - to read stdin
  ```

  With `RUST_BACKTRACE=1`, the full `eyre` report with its backtrace comes first.

- **JSON** (`--error-format json` or `ACRES_ERROR_FORMAT=json`): one line, for programs and agents.

  ```json
  {"kind":"cache.missing","message":"not cached while offline: …","causes":[],"hint":"go online …","exit_code":4}
  ```

  `hint` is `null` when there is none. Usage errors are reported the same way, with kind `usage`.

- **Exit codes** are grouped by what the caller can do about the failure (see `acres-cli --help`):

  | Code | Class | Examples |
  | --- | --- | --- |
  | 0 | success | Also when stdout is closed early, as by `\| head` |
  | 1 | other | Anything unclassified, including bugs |
  | 2 | usage | Unknown flag, missing argument |
  | 3 | not found | `response.not_found` |
  | 4 | offline miss | `cache.missing` |
  | 5 | transient, try again later | `response.too_many_requests`, `response.expired`, `response.transport`, 5xx |
  | 6 | invalid input | `request.*`, `iiif.*`, `charaoke.*`, `input.*`, other 4xx |
  | 7 | environment | `config.invalid`, `cache.io`, `io` |
  | 8 | bad response | `parse.invalid_body` from the API |

  Codes are part of the CLI's contract, like kinds. Never reuse a code for another class.

### TUI reporting

- Failures from something the user asked for (a search, a save, an export, an image fetch) show in
  the status bar, as well as in the log. Failures in background work (cache reads while browsing)
  go only to the log (`L`). Image fetch failures still reach only the log for now. Fixing that is
  planned.
- A bad config file or keybinding stops startup with a message naming the key and the file. It is
  not a panic and isn't reported as a bug.
- A real panic restores the terminal, writes the report to the log file, and asks the user to
  report it.

