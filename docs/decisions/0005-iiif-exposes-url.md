# 0005. Expose `url::Url` from the `iiif` public API

- Status: Proposed
- Date: 2026-11-22

## Context

`iiif` request types could only be turned into a URL by `to_string()`, so callers that need a
URL (e.g. for an HTTP client) re-parsed the string themselves. `iiif` already depends on `url`
to parse base URIs, but only internally. Returning `url::Url` from a public method makes `url` a
public dependency: a semver-breaking `url` release becomes a breaking `iiif` release.

## Decision

`Uri`, `ImageRequest` and `InformationRequest` get `to_url(&self) -> url::Url`, built by parsing
their `Display` output. We don't re-export `url` or wrap `Url` in a newtype.

## Consequences

- Callers get a typed URL without an extra parse step or a direct `url` dependency of their own
  (though matching versions helps if they also use `url`).
- Bumping `url` across a major version is a breaking change for `iiif` and needs a changelog entry.
- `to_url()` must agree with `Display`; tests check this, including property tests for `Uri`.
