# 0005. Keep `url` out of the `iiif` public API

- Status: Accepted
- Date: 2026-11-22

## Context

Issue #50 suggested `to_url() -> url::Url` on `Uri`, `ImageRequest` and `InformationRequest`.
`iiif` already uses `url` internally to parse base URIs, but returning `url::Url` would make it a
public dependency: a semver-breaking `url` release would force a breaking `iiif` release, and
callers would need a compatible `url` version to use the result.

## Decision

Don't expose `url` types. Requests leave `iiif` as strings via `Display`. Callers that want a
`url::Url` parse it themselves (`Url::parse(&request.to_string())`).

## Consequences

- `iiif` can upgrade or drop `url` without a breaking release.
- Callers that want a `Url` parse the string a second time. `iiif` isn't on a hot path, so the cost
  doesn't matter.
- If a typed URL is ever needed, an optional `url` feature (like `serde`, ADR 0004) can add it
  without affecting other users.
