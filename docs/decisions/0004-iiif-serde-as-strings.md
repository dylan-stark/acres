# 0004. Serialize IIIF types as strings, behind a feature

- Status: Proposed
- Date: 2026-11-22

## Context

`iiif`'s public types derived `Serialize`/`Deserialize`, so serde saw their internal shape: a `Uri` became `{scheme, server, prefix, identifier}` and a `Percentage` its stored string. Deserializing skipped every check `FromStr` makes, so invalid values (a `Percentage` of `"banana"`, a `Uri` with a `..` identifier) could be built. The derives also forced serde on users who don't need it. No workspace crate serializes these types.

## Decision

Implement `Serialize`/`Deserialize` by hand for each public type, through `Display` and `FromStr`, so the serialized form is the IIIF string (e.g. `"https://example.org/images/12345"`, `"pct:10,20,30,40"`). Put the impls behind an optional, default-off `serde` feature.

## Consequences

- Serialized values look like the URLs and parameters they represent, and deserializing validates exactly as parsing does.
- Internal representations can change (e.g. #47 storing numbers instead of strings) without changing the serialized form.
- Only string-capable formats are supported. Data written by the old derives no longer deserializes. The crate is unreleased, so there's nothing to migrate.
- Users who want serde must enable `iiif/serde`.
