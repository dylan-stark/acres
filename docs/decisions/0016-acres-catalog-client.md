# 0016. `acres` catalog client

- Status: Accepted
- Date: 2026-11-24

## Context

The TUI is becoming an explorer of the live collection (ADR 0013), and it needs a client it can call many times a second. What `acres` had:

- **Untyped responses.** `Api::fetch::<T>` returned anything `TryFrom<Bytes>`. Search results were a `serde_json::Value` (`Search`, `Cached`), and `Artworks`' `Data` kept only `id`, `image_id` and `title`.
- **A new client per call.** The free function `fetch` built a `reqwest::Client` on every request, so no connection was reused.
- **No rate limiting or retry.** Every call went straight out, and a 429 was treated like any other failure.
- **A cache that never expired.** Every response was written to `<data dir>/<xxh3 of URL>` and read back forever.
- **The wrong header.** `fetch` sent an `ACRES-User-Agent` header with a personal address hard-coded in the source.
- **One flat error type.** `AcresError` mixed layers and wrapped `anyhow` and `url` errors (#62).

The API's rules ([AIC API docs](https://api.artic.edu/docs/), as of 2026-11-24):

| Rule | In short |
| --- | --- |
| Rate limit | Anonymous users are throttled to 60 requests a minute, per IP |
| Page size | `limit` cannot exceed 100; default 12 |
| Search depth | No more than 10,000 records from a single search query |
| Identification | Send an `AIC-User-Agent` header with a project name and contact, as a courtesy |
| Images | Take the IIIF base from `config.iiif_url`; 843 px wide is the most likely cached size; fetch one at a time |

## Decision

`acres` gets a typed model, one long-lived `Catalog` client, and errors layered as in ADR 0006. The CLI and the TUI's image fetch move onto it in the same change set.

**Typed responses.** `Page<T>` holds `pagination`, `config` and `data: Vec<T>`; `Item<T>` holds one record. `ArtworkSummary` (id, title, artist, date, image, thumbnail) is a list row and `Artwork` the full record, each with a `FIELDS` constant to pass as `fields`. Optional fields are `Option`s and unknown fields are ignored. Each `Catalog` call also has a `raw` form returning the body unparsed, so the CLI keeps printing the API's JSON exactly as returned (ADR 0002).

**Search requests.** The search builder gains `fields`, `limit`, `page` and typed `Filters` (`has_image`, `is_public_domain`, `is_on_view`, `department_id`, `artwork_type_id`, `artist_id`, a range of years), sent as an Elasticsearch `bool` filter in the API's bracket form. It refuses a `limit` over 100 or a page past 10,000 records before sending anything, and percent-encodes `q`.

**One `Catalog`.** Cheap to clone (an `Arc` inside), it owns one `reqwest::Client`, the `AIC-User-Agent: acres/<version>` header (with ` (<contact>)` only when `ACRES_CONTACT` or `contact` is set), the rate limiter, the cache and the config, read once.

**Queueing.** A token bucket of 60 a minute (bursts of 5) covers every request, but a strict first-in, first-out queue would build a backlog nobody wants after a new search. So:

- Dropping a request's future removes it from the queue without taking a token.
- Each request has a maximum wait, 10 seconds by default and settable per handle (`with_max_wait`); past it, it fails with `ResponseError::Expired`, unsent.
- Metadata (searches, listings, records) is served before images, oldest first. Images are served newest first, one in flight at a time.

**Retries.** A 429 or 503 is retried up to 3 times, honoring `Retry-After` in seconds, otherwise backing off 1, 2, then 4 seconds. Each retry goes back through the queue and counts against the same maximum wait.

**Cache lifetimes.** Images are kept forever, records for 7 days and search or listing pages for 24 hours. Age is the file's modified time, so the on-disk layout and existing caches are unchanged. `ACRES_OFFLINE=true` answers only from the cache, ignoring lifetimes, and a miss is `CacheError::Missing`.

**Errors.** `acres::Error` wraps `RequestError` (`InvalidQuery`, `OutOfRange`, `InvalidImageUrl`, `InvalidContact`, `Iiif`), `ResponseError` (`Status`, `Expired`, `TooManyRequests`, `Transport`), `CacheError` (`Missing`, `Io`) and `ParseError` (`InvalidBody`). Messages are lowercase, leave out their source, and third-party causes are boxed, so `anyhow` leaves `acres`.

**Consumers.** `acres-cli` builds one `Catalog` and every fetching command goes through it, with unchanged output and a new `--offline`. `acres-tui` shares one `Catalog` from `App` for images. `Api`, `Search` and `Cached` are deprecated, pointing to `Catalog`, and removed in a later release. `Artworks` stays as the TUI's file format.

`acres` gains `tokio` with only the `time` and `sync` features, and still depends only on `iiif` in the workspace.

## Consequences

- The TUI explorer (ADR 0013) gets a client it can share across tasks and a model it can render without touching JSON.
- Requests no longer carry a personal address. Anyone who wants the API to be able to reach them sets `ACRES_CONTACT`.
- Cached search pages and records can go stale and be refetched, so working fully offline now takes `ACRES_OFFLINE`.
- At 60 requests a minute, a burst can wait a few hundred milliseconds. The wait is visible in `Fetched::wait` instead of turning into failures.
- Code matching on `AcresError` breaks, and `Api`, `Search` and `Cached` warn. The project is pre-1.0 and the binaries are the only users.
- Alternatives we passed on: a limiter hidden inside `Api::fetch` (still a client per call, hard to test); the `governor` crate (a 60-a-minute bucket is a few lines over `tokio::time`, and our own is easier to test on a paused clock); an HTTP cache that obeys cache headers (a new on-disk format and dependency tree for three fixed lifetimes); and types generated from a schema (the API publishes none, and we use about a dozen fields).
- Tests use `wiremock` and a paused Tokio clock, so none sleeps or reaches the API. The one bench, `acres/parse/search_page`, parses a 50-row page fixture:

  | | M1 Air (target) | Ryzen 9700X (dev) |
  | --- | --- | --- |
  | `acres/parse/search_page` | to measure | to measure |
