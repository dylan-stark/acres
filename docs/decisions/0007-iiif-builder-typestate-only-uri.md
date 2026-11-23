# 0007. Typestate only the required part of the `iiif` image request builder

- Status: Accepted
- Date: 2026-11-22

## Context

`ImageRequest::builder()` returned `Builder<A, B, C, D, E, F>`, which carried a `Set`/`Unset`
marker for each of the six request parts. `build()` needed all six to be `Set`, but only the
base URI has no default (#49). Callers had to spell out defaults by hand, the compiler errors
named six opaque parameters, and the type lived in a private module, so it couldn't be named or
documented.

## Decision

Only required parts get a typestate. The builder is `ImageRequestBuilder<U = NoBaseUri>`, and
its type parameter is the stored base URI itself: `NoBaseUri` until `base_uri()` is called, then
`BaseUri`. `build(self)` exists only on `ImageRequestBuilder<BaseUri>`. Optional parts start at
their defaults and their setters return `Self`. Both types are public and re-exported from the
crate root.

## Consequences

- `ImageRequest::builder().base_uri(uri).build()` is enough for a `full/full/0/default.jpg`
  request, and building without a URI fails with "no method named `build` found for
  `ImageRequestBuilder<NoBaseUri>`".
- Using the URI as the state means no marker generics, no `PhantomData` and no `Option` to
  `expect` in `build()`.
- If a part that has no default is ever added, it needs its own type parameter. Keep that to the
  parts the IIIF spec actually requires.
