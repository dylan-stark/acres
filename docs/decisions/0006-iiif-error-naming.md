# 0006. Name and layer `iiif` errors by what is wrong with the input

- Status: Accepted
- Date: 2026-11-22

## Context

`IiifError` was one flat enum with mixed naming (`InvalidX`, `MissingX`, `HasQuery`,
`BadSlashes`, `InvalidUriWithUserInfo`), two variants for the prefix, a `MissingIdentifier` for a
request with too few segments, and `url::ParseError` in its public API (#52). Request parsing
passed base-URI errors through unchanged, so callers couldn't tell which part of a request
failed (#43). Consumers report errors with `eyre`/`color-eyre`, which print the whole `source()`
chain, so messages that also interpolated their source printed it twice.

## Decision

- **Names say what is wrong with the input,** as std does (`io::ErrorKind::UnexpectedEof`,
  `IntErrorKind::InvalidDigit`): a qualifier plus the part of the input it applies to. Qualifiers
  are `Invalid` (present but malformed), `Missing` (required but absent), `Unexpected` (present
  but not allowed) and `OutOfRange` (parses, but the value isn't allowed). A more exact word wins
  where one exists (`TrailingSlash`, `BadSlashes`). Names don't repeat what the type already says
  (`UriError::InvalidChar`, not `InvalidUriChar`), and the umbrella type is `iiif::Error`.
- **One error type per layer:** `UriError` (base URI), `ParameterError` (region, size, rotation,
  quality, format, percentage, degree), `RequestError` (image and information requests, wrapping
  the lower errors with `#[from]`). `iiif::Error` wraps all three transparently.
- **Messages:** lowercase, no trailing punctuation. Each leaf error holds the offending input.
  An error with a `source()` leaves it out of its own message.
- **No dependency types:** a third-party cause is held as `Box<dyn Error + Send + Sync>`
  (consistent with ADR 0005).

## Consequences

- Callers can match on which layer failed (`RequestError::InvalidUri(UriError::InvalidChar(_))`)
  or use `iiif::Error` with `?` and ignore the layering.
- `to_string()` on a wrapping error shows only the outer message; callers who want the cause
  walk `source()` or use an error reporter. Serde only keeps messages, so the `serde` impls join
  the chain into one message.
- New variants follow these rules; review against this ADR. `acres`'s own error type gets the
  same treatment in #62.
