# acres

Typed, polite access to the [Art Institute of Chicago's public API](https://api.artic.edu/docs/).

You build a request for an endpoint and fetch it with one shared `Catalog`, which

- keeps to the API's 60 requests a minute, shared by every process on the machine,
- retries 429 and 503 responses,
- caches on disk with lifetimes (images for good, records for 7 days, search pages for 24 hours),
- answers from the cache alone when offline, and
- sends the `AIC-User-Agent` header the API asks for.

```rust,no_run
use acres::{Catalog, artworks::{ArtworkSummary, request::search}};

let catalog = Catalog::builder().build()?;
let request = search::Request::builder()
    .base_uri(catalog.base_uri())
    .q(Some("monet".into()))
    .fields(Some(ArtworkSummary::FIELDS.iter().map(|f| f.to_string()).collect()))
    .build()?;
let page = catalog.search(&request).await?;
for artwork in page.value.data {
    println!("{} ({})", artwork.title, artwork.id);
}
```

Responses are typed (`Page<ArtworkSummary>`, `Item<Artwork>`, departments and artwork types), or raw JSON through `Catalog::raw`.
Every error has a stable `kind()`, such as `cache.missing`, and many have a `hint()` on what to do next.

The `ACRES_*` environment variables set the defaults, such as `ACRES_OFFLINE` and `ACRES_CACHE_DIR`; the [workspace README](https://github.com/dylan-stark/acres/blob/main/README.md#configuration) lists them.

`acres` powers [`acres-cli`](../acres-cli) and [`acres-tui`](../acres-tui). The [API docs](https://docs.rs/acres) cover every endpoint and type.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
