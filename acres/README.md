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

`Catalog::builder()` starts from these environment variables, and the builder's methods override them:

| Variable | What it does | Default |
| --- | --- | --- |
| `ACRES_OFFLINE` | `true` answers only from the cache, without touching the network | `false` |
| `ACRES_CONTACT` | A contact, such as an email address, added to the `AIC-User-Agent` header | none |
| `ACRES_CACHE_DIR` | Where responses and images are cached | the platform cache directory |
| `ACRES_USE_CACHE` | `false` turns the cache off | `true` |
| `ACRES_STATE_DIR` | Where the rate limit shared between processes is kept | the platform state directory |
| `ACRES_BASE_URI` | The API to talk to | `https://api.artic.edu/api/v1` |

A value that can't be read, such as `ACRES_OFFLINE=maybe`, makes `build()` fail with a `config.invalid` error.

`acres` powers [`acres-cli`](../acres-cli) and [`acres-tui`](../acres-tui). `cargo doc --open -p acres` builds the API docs, which cover every endpoint and type.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
