# 0001. Builder pattern for ASCII conversion

- Status: Accepted
- Date: retroactive (recorded 2026-10-14)

## Context

The `image-to-ascii` crate exposes many knobs (alphabet, font, metric, conversion algorithm, brightness offset, character width) with awkward constructors. Both `acres-cli` and `acres-tui` need to set subsets of them from user input.

## Decision

Wrap it in the `image-to-ascii-builder` crate, which offers `Ascii::builder()` with chainable setters, typed enums for each option, and sensible defaults. Callers set only what they care about.

## Consequences

- CLI and TUI share one vocabulary of options and parsing (`Alphabet`, `Font`, ...).
- Adding an option is a builder method plus an enum variant, not a signature change.
- The crate has no workspace dependencies, so it can be tested and published on its own.
