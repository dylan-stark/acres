# Architecture decision records

Short records of non-obvious decisions, so a later reader (human or agent) knows why the code is shaped the way it is.

- Copy [`0000-template.md`](0000-template.md) to `NNNN-short-title.md` using the next number.
- Keep it to one page. Never edit an accepted ADR's decision; add a new ADR that supersedes it and update the old one's status.

| ADR | Title | Status |
| --- | --- | --- |
| [0001](0001-builder-pattern-for-ascii.md) | Builder pattern for ASCII conversion | Accepted |
| [0002](0002-cli-json-to-stdout.md) | CLI writes JSON to stdout | Accepted |
| [0003](0003-split-iiif-crate.md) | Split IIIF into its own crate | Accepted |
| [0004](0004-iiif-serde-as-strings.md) | Serialize IIIF types as strings, behind a feature | Proposed |
| [0005](0005-iiif-no-public-url.md) | Keep `url` out of the `iiif` public API | Accepted |
| [0006](0006-iiif-error-naming.md) | Name and layer `iiif` errors by what is wrong with the input | Accepted |
