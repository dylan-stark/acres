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

### Changed

- Renamed the project from `aic-tui` to `acres`.

### Fixed

- Parsing an `iiif::ImageRequest` from a string no longer drops a non-default port from the base URI.

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
