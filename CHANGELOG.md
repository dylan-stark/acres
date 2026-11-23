# Changelog

All notable user-visible changes are recorded here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Workspace split into `iiif`, `image-to-ascii-builder`, `acres`, `acres-cli` and `acres-tui` crates (see [ADR 0003](docs/decisions/0003-split-iiif-crate.md)).
- `acres-cli` for fetching artworks, manifests and IIIF URLs/bytes and for rendering ASCII art, with JSON written to stdout (see [ADR 0002](docs/decisions/0002-cli-json-to-stdout.md)).
- `acres-tui` now reads artworks from a JSON file produced by `acres-cli`.
- Crate-level documentation, runnable examples and an expanded README.
- CI snapshot check (`cargo insta test --check`), an optional Claude Code PR review workflow and a `reviewer` subagent.
- Offline demo recording (`make demo`) and snapshot tests for TUI components.
- `iiif` types can be read back: `Percentage::value()` and `Degree::value()`; `ImageRequest::{uri, region, size, rotation, quality, format}()`; `InformationRequest::uri()`; and `BaseUri::{scheme, host, port, prefix, identifier}()` (prefix and identifier percent-decoded).

### Changed

- Renamed the project from `aic-tui` to `acres`.
- Renamed `iiif::Uri` to `iiif::BaseUri` (and `UriError` to `BaseUriError`), the IIIF spec's term, since it only accepts IIIF base URIs and no longer clashes with `http::Uri`.
- `iiif` errors are split by layer (see [ADR 0006](docs/decisions/0006-iiif-error-naming.md)): parsing a `BaseUri` returns `BaseUriError`, parsing a `Region`, `Size`, `Rotation`, `Quality`, `Format`, `Percentage` or `Degree` returns `ParameterError`, and parsing an `ImageRequest` or `InformationRequest` returns `RequestError`, which wraps the failing part's error as its `source()`. `IiifError` is replaced by `iiif::Error`, which wraps any of the three. All are `#[non_exhaustive]`, so matching on them needs a wildcard arm.
- `iiif` error variants are renamed to describe what is wrong with the input: `HasQuery`/`HasFragment`/`InvalidUriWithUserInfo` are `UnexpectedQuery`/`UnexpectedFragment`/`UnexpectedUserInfo`; `InvalidUriChar`/`InvalidUriEscape` are `InvalidChar`/`InvalidEscape`; `InvalidUriPrefix` is merged into `InvalidPrefix`; out-of-range `InvalidPercentage`/`InvalidDegree` are `PercentageOutOfRange`/`DegreeOutOfRange`, and the non-numeric `InvalidPercentageString`/`InvalidDegreeString` take over the `InvalidPercentage`/`InvalidDegree` names; `MissingInfoPart` is `MissingInfoJson`; and an `ImageRequest` with too few segments reports `RequestError::MissingUri` instead of `MissingIdentifier`. `InvalidPrefix` and `InvalidIdentifier` now live on `BaseUriError`. The unused `MissingParams` is removed.
- `BaseUriError::Malformed` (formerly `InvalidUri`) no longer exposes `url::ParseError`, so `url` stays out of the `iiif` public API (see [ADR 0005](docs/decisions/0005-iiif-no-public-url.md)).
- `iiif` and `acres` error messages no longer repeat their source error, so `eyre`/`anyhow` reports print each cause once. Deserializing an `iiif` type still includes the whole cause chain in its message. Several `iiif` messages are reworded to match the variant names (e.g. "unexpected query in URI", "malformed URI", "missing region in image request").
- `acres::AcresError` has a new `InvalidImageUrl` variant for an artwork image URL that can't be built, instead of reusing an `iiif` error. `AcresError` isn't `#[non_exhaustive]`, so exhaustive matches on it need the new arm.
- `iiif::BaseUri` parsing follows IIIF Image API §2/§3/§9 and RFC 3986 strictly. It now rejects user info, missing or extra slashes after the scheme, queries, fragments, trailing slashes, empty path segments, unencoded `@ [ ]`, raw non-ASCII or other characters outside RFC 3986, malformed `%` escapes, and `.`/`..` identifiers (encoded or not). Valid but non-canonical input is still accepted and displayed in canonical form.
- `BaseUriError` has variants for these cases: `UnexpectedUserInfo`, `InvalidChar`, `InvalidEscape`, `InvalidPrefix`, `InvalidIdentifier`, `BadSlashes`, `UnexpectedQuery`, `UnexpectedFragment` and `TrailingSlash`. `MissingServer` now carries the offending URI.
- `iiif` types serialize as their IIIF string form (e.g. `"https://example.org/images/12345"`) instead of their internal structure, and serde support is now behind the optional `serde` feature (see [ADR 0004](docs/decisions/0004-iiif-serde-as-strings.md)).
- `iiif` errors no longer implement `From<ParseFloatError>`; `ParameterError::InvalidPercentage` is used only for percentages.

### Removed

- Removed `iiif::Percentage::new()` and `iiif::Degree::new()`, which took no value and returned 0. Use `Default` for 0, or `TryFrom<f32>`/`FromStr` for a value.

### Fixed

- `iiif::Quality` and `iiif::Format` are now `Copy`.
- `iiif` no longer depends on `anyhow`, `bytes`, `strum` or `tracing`, and its crates.io keywords and categories describe the library instead of the CLI/TUI. Broken links in its docs are fixed.
- `iiif::Percentage` and `iiif::Degree` compare numerically (so `9 < 10`) instead of as text, which also fixes the ordering of `Region`, `Size`, `Rotation` and `ImageRequest`. They are now `Copy`.
- Deserializing `iiif` types now validates input the same way parsing does, so invalid values (e.g. a `Percentage` of `"banana"`) are rejected.
- Parsing an `iiif::Degree` from a non-numeric string reports `ParameterError::InvalidDegree` ("invalid degree") instead of a percentage error.
- Parsing an `iiif::ImageRequest` from a string no longer drops a non-default port from the base URI.
- Malformed input no longer accepted during region/size/rotation parsing.
- Displaying an `iiif::BaseUri` re-encodes characters such as space, `"` and `{` in the prefix and identifier instead of writing them raw.
- An `iiif::BaseUri` identifier with a needlessly encoded `.` (e.g. `a%2Eb`) is now accepted rather than rejected.

## [0.2.0] - 2024-10-13

### Added

- `AIC-User-Agent` header on API requests.
- Resized-image support in the TUI.

### Changed

- CI uses the stable toolchain.
- Cleaned up the `User-Agent` header.

### Removed

- The `fps` component.

## [0.1.0] - 2024-10-02

### Added

- First pass at basic functionality: browse Art Institute of Chicago public-domain artworks as ASCII art in a TUI (`aic-tui`).

[Unreleased]: https://github.com/dylan-stark/acres/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/dylan-stark/acres/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dylan-stark/acres/releases/tag/v0.1.0
