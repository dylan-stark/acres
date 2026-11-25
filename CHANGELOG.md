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
- `iiif` types can be read back: `Percentage::value()` and `Degrees::value()`; `ImageRequest::{base_uri, region, size, rotation, quality, format}()`; `InformationRequest::base_uri()`; and `BaseUri::{scheme, host, port, prefix, identifier}()` (prefix and identifier percent-decoded).
- `charaoke` crate, the start of a from-scratch image-to-ASCII engine aimed at live TUI resizing (see [ADR 0008](docs/decisions/0008-charaoke-from-scratch-engine.md)). `Grid::fit` sizes the character grid.
- `make bench` runs criterion benchmarks for `charaoke` and for the existing `image-to-ascii` path as a baseline. `make bench-save` and `make bench-compare` save a named baseline and report regressions against it.
- `charaoke::Converter` turns RGB, RGBA or grayscale pixels into ASCII: it loads an image once, then renders it at any grid size into a reused `String`. `charaoke::Ramp` sets the characters (see [ADR 0009](docs/decisions/0009-charaoke-load-once-render-many.md)).
- `charaoke` benches for loading, rendering and end-to-end conversion, plus a headless Ratatui resize bench (`charaoke/tui_proxy`).
- `charaoke` renders in color: `Converter::render` fills `charaoke::Cells` with a character and a mean color per cell, and `Cells::write_ansi` writes truecolor or 256-color ANSI (`ColorDepth`, `charaoke::ansi256`), while `Cells::write_plain` writes plain text (see [ADR 0010](docs/decisions/0010-charaoke-color.md)).
- Benches for `charaoke` ANSI output (`charaoke/ansi`), a direct-to-`Buffer` color resize and its `ansi-to-tui` alternative (`charaoke/tui_proxy/drag`, `drag_ansi`), and today's `ansi-to-tui` parse (`baseline/ansi_parse`). `charaoke/end_to_end` now includes ANSI output.
- `acres-tui` fits the art to its pane and redraws it on every resize, in truecolor or 256-color depending on `COLORTERM` (see [ADR 0011](docs/decisions/0011-switch-to-charaoke.md)).
- `acres-tui` has a timing HUD, toggled with `t`, that slides out over the art with fetch, decode, load, render, draw and frame times.
- `acres-cli ascii-art` takes `--ramp` (a preset name or custom characters), `--color auto|truecolor|256|never` and `--timings`, which writes stage times to stderr after the art.
- `charaoke::Ramp::PRESETS`, `Ramp::preset`, `Ramp::with_brightness_offset`, `Grid::fit_within` and `Converter::set_ramp`.
- `charaoke/tui_proxy/drag_fit` bench, a resize drag that changes both width and height through `Grid::fit_within`.
- Block modes, for more detail in full color: half blocks (`▀`, two pixels per cell) and quadrant blocks (four pixels per cell). `acres-cli ascii-art` takes `--mode ascii|half|quadrant`, and `b` cycles the modes in `acres-tui` (see [ADR 0014](docs/decisions/0014-charaoke-block-modes.md)).
- `charaoke::Converter::render_blocks`, `BlockMode`, `Blocks` (with `Blocks::write_ansi`) and `QUADRANTS`, plus the `charaoke/render_blocks`, `charaoke/ansi_blocks` and `charaoke/tui_proxy/drag_half` and `drag_quadrant` benches.
- `acres::Catalog`, a shared client for the AIC API that rate-limits to 60 requests a minute, retries 429 and 503 responses, caches with lifetimes (images forever, records 7 days, search pages 24 hours) and sends `AIC-User-Agent`. Waiting requests can be cancelled by dropping them, expire after 10 seconds by default, and metadata goes before images (see [ADR 0016](docs/decisions/0016-acres-catalog-client.md)).
- Typed responses in `acres`: `Page<T>`, `Item<T>`, `artworks::ArtworkSummary` and `artworks::Artwork`, each with a `FIELDS` list to request.
- The `acres` search builder takes `fields`, `limit`, `page` and typed `Filters` (image, public domain, on view, department, type, artist, years), and refuses a `limit` over 100 or a page past 10,000 records.
- `ACRES_OFFLINE` answers only from the cache, and `ACRES_CONTACT` adds a contact to `AIC-User-Agent`.
- `acres/parse/search_page` bench, run by `make bench` with the others.
- `acres::DEFAULT_BASE_URI`, the public API's base URI.
- The `acres` rate limit is shared by every process on the machine, so separate `acres-cli` runs, and the TUI running beside them, keep to 60 requests a minute together. The state lives in `rate-limit` in the app's state directory (`$XDG_STATE_HOME/acres` on Linux, the local data directory elsewhere), which `ACRES_STATE_DIR` overrides. `CatalogBuilder::rate_limit_file` and `no_shared_rate_limit` choose another file or keep the limit in memory.
- `acres-tui` is an explorer (ADR 0013, step 1): a search bar, results, art and record panes with numbered borders, a status bar, and overlays. `?` opens help built from the loaded keymap, `1` to `4`, `tab` and `shift-tab` move focus, `n`/`p` change artwork from any pane, `z` is zen mode, and the search bar narrows the list by title. The status bar suggests keys for the focused pane and echoes the last key (`show_keys: false` turns it off). The mouse scrolls and clicks in the panes (`mouse: false` turns capture off) (see [ADR 0013](docs/decisions/0013-tui-catalog-explorer.md)).
- `tui/frame/explorer`, `tui/frame/zen` and `tui/frame/help` benches of whole TUI frames in each art mode, run by `make bench`.

### Changed

- `acres-tui` keys are bound per focus context (`Global`, `List`, `Art`, `Record`, `Search`, `Overlay`) instead of per mode. `esc` no longer opens the list, `a` (ramps) is now `r`, `l` (logs) is now `L`, and `ctrl-d` pages instead of quitting. Single-character keys keep their case, so `G` and `g` are different keys.
- `acres-tui` starts in half blocks, and previews the selected artwork as the cursor moves, 150 ms after it stops, instead of waiting for `enter`.
- `acres-cli` fetches through one `acres::Catalog`, so its commands share the rate limit, retries and cache lifetimes, and it takes `--offline` (or `ACRES_OFFLINE=true`) to answer only from the cache. JSON commands print the body exactly as the API sent it, instead of re-serializing it with sorted keys.
- `acres-tui` fetches images through one shared `Catalog`, and a newer image request cancels the one still waiting.
- The `acres` request constructors and builders take `impl Into<String>` for the base URI, so a `&str` such as `acres::DEFAULT_BASE_URI` or `Catalog::base_uri()` works directly.
- `acres-cli` and `acres-tui` convert images with `charaoke` instead of `image-to-ascii`. Characters are chosen by brightness, so line art looks softer than with the old edge-matching default (see [ADR 0011](docs/decisions/0011-switch-to-charaoke.md)).
- `acres-cli ascii-art --width` defaults to the terminal width, or 80 when stdout isn't a terminal, instead of the image width. With `--color auto`, the default, output is plain text when stdout isn't a terminal or `NO_COLOR` is set.
- The `acres-tui` alphabets list (`a`) is now a ramps list.
- Renamed the project from `aic-tui` to `acres`.
- Renamed `iiif::Uri` to `iiif::BaseUri`, the IIIF spec's term, since it only accepts IIIF base URIs and no longer clashes with `http::Uri`. The `uri()` methods on `ImageRequest`, `InformationRequest` and the image request builder are now `base_uri()`.
- Renamed the matching errors: `UriError` is `BaseUriError`, `iiif::Error::InvalidUri` and `RequestError::InvalidUri` are `InvalidBaseUri`, and `RequestError::MissingUri` is `MissingBaseUri`.
- Renamed `iiif::Degree` to `Degrees`, since it is a quantity (`Rotation::Degrees(Degrees)`). Its errors follow, with messages "invalid degrees" and "degrees out of range".
- Renamed `iiif::Size::Exactly` to `Size::Exact` and `iiif::Region::Absolute` to `Region::Pixels`, so variants are nouns that name their units.
- `iiif::Region::{Pixels, Percentage}` and `iiif::Size::{Exact, BestFit}` have named fields (`{ x, y, w, h }` and `{ w, h }`) instead of positional ones, so callers no longer have to remember the order.
- Renamed `iiif::Format::WebP` to `Format::Webp`, treating the name as one word like `Jpg` and `Jp2`.
- `iiif` errors are split by layer (see [ADR 0006](docs/decisions/0006-iiif-error-naming.md)): parsing a `BaseUri` returns `BaseUriError`, parsing a `Region`, `Size`, `Rotation`, `Quality`, `Format`, `Percentage` or `Degrees` returns `ParameterError`, and parsing an `ImageRequest` or `InformationRequest` returns `RequestError`, which wraps the failing part's error as its `source()`. `IiifError` is replaced by `iiif::Error`, which wraps any of the three. All are `#[non_exhaustive]`, so matching on them needs a wildcard arm.
- `iiif` error variants are renamed to describe what is wrong with the input: `HasQuery`/`HasFragment`/`InvalidUriWithUserInfo` are `UnexpectedQuery`/`UnexpectedFragment`/`UnexpectedUserInfo`; `InvalidUriChar`/`InvalidUriEscape` are `InvalidChar`/`InvalidEscape`; `InvalidUriPrefix` is merged into `InvalidPrefix`; out-of-range `InvalidPercentage`/`InvalidDegree` are `PercentageOutOfRange`/`DegreesOutOfRange`, and the non-numeric `InvalidPercentageString`/`InvalidDegreeString` are `InvalidPercentage`/`InvalidDegrees`; `MissingInfoPart` is `MissingInfoJson`; and an `ImageRequest` with too few segments reports `RequestError::MissingBaseUri` instead of `MissingIdentifier`. `InvalidPrefix` and `InvalidIdentifier` now live on `BaseUriError`. The unused `MissingParams` is removed.
- `BaseUriError::Malformed` (formerly `InvalidUri`) no longer exposes `url::ParseError`, so `url` stays out of the `iiif` public API (see [ADR 0005](docs/decisions/0005-iiif-no-public-url.md)).
- `iiif` and `acres` error messages no longer repeat their source error, so `eyre`/`anyhow` reports print each cause once. Deserializing an `iiif` type still includes the whole cause chain in its message. Several `iiif` messages are reworded to match the variant names (e.g. "unexpected query in URI", "malformed URI", "missing region in image request").
- `iiif::BaseUri` parsing follows IIIF Image API §2/§3/§9 and RFC 3986 strictly. It now rejects user info, missing or extra slashes after the scheme, queries, fragments, trailing slashes, empty path segments, unencoded `@ [ ]`, raw non-ASCII or other characters outside RFC 3986, malformed `%` escapes, and `.`/`..` identifiers (encoded or not). Valid but non-canonical input is still accepted and displayed in canonical form.
- `BaseUriError` has variants for these cases: `UnexpectedUserInfo`, `InvalidChar`, `InvalidEscape`, `InvalidPrefix`, `InvalidIdentifier`, `BadSlashes`, `UnexpectedQuery`, `UnexpectedFragment` and `TrailingSlash`. `MissingServer` now carries the offending URI.
- `iiif` types serialize as their IIIF string form (e.g. `"https://example.org/images/12345"`) instead of their internal structure, and serde support is now behind the optional `serde` feature (see [ADR 0004](docs/decisions/0004-iiif-serde-as-strings.md)).
- The `iiif` image request builder is now the public, documented `iiif::ImageRequestBuilder`, so it can be named (e.g. `ImageRequestBuilder<NoBaseUri>`). Only the base URI must be set before `build()`; the other parts default to `full/full/0/default.jpg`. `build()` takes `self` instead of `&self` (see [ADR 0007](docs/decisions/0007-iiif-builder-typestate-only-uri.md)).
- `iiif` errors no longer implement `From<ParseFloatError>`; `ParameterError::InvalidPercentage` is used only for percentages.
- `acres` errors are layered as in [ADR 0006](docs/decisions/0006-iiif-error-naming.md): `acres::Error` replaces `AcresError` and wraps `RequestError`, `ResponseError`, `CacheError` and `ParseError`. An artwork image URL that can't be built is `RequestError::InvalidImageUrl`, and `ArtworkInfo::load` returns a `Result`. No `anyhow` or `url` type is part of the API any more.
- `acres` sends `AIC-User-Agent: acres/<version>` instead of an `ACRES-User-Agent` header with a hard-coded address.
- The `acres` cache keeps each kind of response in its own folder (`pages`, `records`, `images`) under the cache directory, names entries by the hex hash of the URL, and writes them atomically. The cache moved from the local data directory to the platform's cache directory (`$XDG_CACHE_HOME/acres` on Linux, `~/Library/Caches` on macOS; `ACRES_CACHE_DIR` still overrides it). Caches from earlier versions are not read; delete the old files in the local data directory (`~/.local/share/acres` on Linux) to reclaim the space.
- Search `q`, `query` and `sort` values are percent-encoded, so a search with spaces or `&` reaches the API intact.

### Removed

- `acres::Api`, `ApiBuilder`, `Cached` and the free `acres::fetch()`: use `acres::Catalog`. The untyped `artworks::Search` and `artworks::Manifest` responses are removed too: use `Page<ArtworkSummary>`, or `Catalog::raw` for the JSON as sent.
- `acres-cli ascii-art` no longer takes `--alphabet` (use `--ramp`), `--font`, `--metric` or `--conversion-algorithm`, which only applied to glyph matching.
- `acres-tui` no longer has the font (`f`), metric (`m`) or conversion algorithm (`c`) lists.
- Removed the hidden `iiif::builder::{Set, Unset}` typestate markers; `iiif::NoBaseUri` replaces them.
- Removed `iiif::Percentage::new()` and `iiif::Degrees::new()`, which took no value and returned 0. Use `Default` for 0, or `TryFrom<f32>`/`FromStr` for a value.

### Fixed

- `acres-tui` builds in release mode again: the panic hook no longer uses `human-panic`, which was never a dependency.
- `iiif::Quality` and `iiif::Format` are now `Copy`.
- `iiif` no longer depends on `anyhow`, `bytes`, `strum` or `tracing`, and its crates.io keywords and categories describe the library instead of the CLI/TUI. Broken links in its docs are fixed.
- `iiif::Percentage` and `iiif::Degrees` compare numerically (so `9 < 10`) instead of as text, which also fixes the ordering of `Region`, `Size`, `Rotation` and `ImageRequest`. They are now `Copy`.
- Deserializing `iiif` types now validates input the same way parsing does, so invalid values (e.g. a `Percentage` of `"banana"`) are rejected.
- Parsing an `iiif::Degrees` from a non-numeric string reports `ParameterError::InvalidDegrees` ("invalid degrees") instead of a percentage error.
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
