# 0002. CLI writes JSON to stdout

- Status: Accepted
- Date: retroactive (recorded 2026-10-14)

## Context

`acres-cli` retrieves data from the AIC API that other tools (`jq`, `curl`, `acres-tui`, shell scripts) want to consume.

## Decision

Subcommands print the raw API response as JSON (or a bare URL/bytes for `iiif`) to stdout and nothing else. Diagnostics and logs go to stderr so stdout stays clean. Subcommands that take input accept a file or `-` for stdin.

## Consequences

- Commands compose in pipelines, e.g. `acres-cli artworks > artworks.json; acres-tui artworks.json`.
- No pretty-printing or formatting is built in; users reshape with `jq`.
- Changing output shape is a user-visible change and belongs in the CHANGELOG.
