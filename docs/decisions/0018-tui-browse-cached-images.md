# 0018. Browse cached images instead of placeholders

- Status: Accepted
- Date: 2026-11-26

We drop the `lqip` placeholder that [ADR 0013](0013-tui-catalog-explorer.md) shows while browsing. Moving through a list now shows an artwork's full image only when it is already in the local image cache, and otherwise leaves the art pane empty until `enter` or `space` fetches it. Each list row starts with a small glyph that says whether its image is cached.

## Context

- **The placeholder didn't work out.** ADR 0013 (step 2) shows each record's `thumbnail.lqip`, a GIF about 8x5 pixels that comes inline with every list, scaled up into a soft blur after a 150 ms debounce. In practice the blur doesn't look good, and it reads as a broken image more than a preview.
- **The full image is already kept on disk.** [ADR 0016](0016-acres-catalog-client.md)'s `Catalog` caches every image under `<cache dir>/images/<xxh3 of the URL>`, and images never expire. Once an artwork's 843-pixel image has been fetched, it is a local file read away, with no request and no rate-limit cost. The TUI never asked the cache directly: it only benefited when `enter` happened to hit it.
- **A small in-memory LRU sits on top.** `components/iiif.rs` keeps the last 16 decoded full images (`Recent`), so coming back to one shows it at once. That stays as it is.
- **Nothing in the list said what's local.** You couldn't tell which rows would show art at once and which needed a fetch, which matters most in `--offline` mode.

## Decision

### 1. Remove the placeholder

- `Entry::placeholder`, the placeholder path in `Iiif` and the tiny-image upscale in `Decoded::decode` are gone, and so is the `image` crate's GIF feature in `acres-tui`.
- `acres::artworks::Thumbnail` and `ArtworkSummary::thumbnail` are gone, and list and record requests no longer ask for `thumbnail`. That trims a few hundred bytes of base64 per row from every page. The CLI's raw JSON output is not affected.
- `Action::UpdateImage` loses its `full` flag, since every image shown is now the full image.

### 2. Browsing shows cached images only

When the selection changes:

1. **Decoded recently** (in `Recent`): shown at once, as before.
2. **In the disk cache**: the art pane clears at once, and after the same 150 ms debounce the image is read from the cache and decoded on a blocking thread, then shown. Holding `j` through a list still reads and decodes nothing until the cursor rests.
3. **Not cached**: the art pane clears at once (not after the debounce), so the previous artwork's art never sits next to the new artwork's label. The pane stays blank, with no hint: the row's open circle already says there is no local copy.

`enter` and `space` work as before: they fetch the full image once per artwork, through the cache. On a cached image already shown, they do nothing.

### 3. A cache-only lookup in `acres`

`Catalog` gains two cache-only calls that never touch the network or the rate limit, even online:

- `Catalog::cached_image(&ImageRequest) -> Result<Option<Bytes>, Error>` reads the entry if it exists.
- `Catalog::has_cached_image(&ImageRequest) -> bool` is a single `stat` of the entry's path.

With the cache turned off, both report nothing cached. Only the exact request the TUI makes (`Entry::image_request`: full region, 843 wide, default quality, JPEG) counts. The CLI fetches the same URL, so images it fetched count too.

### 4. A glyph for cached rows

- The explorer keeps the ids known to be cached, and the ids already checked. Whenever a list changes (a page arrives, the saved list shows, a step back), rows not yet checked go to the image fetcher as `Action::CheckCached`, which `stat`s each on a blocking thread and answers with `Action::Cached(ids)`. The draw never touches the disk.
- A finished full-image fetch sends its id too, so the glyph changes as soon as you've fetched it.
- Rows start with a dim `•` when the image is cached and a dim open circle `◦` when it is not, in a fixed column before the title (and before the `*` for saved), in both Results and Saved. A row with no image at all has a blank there. The help overlay for the list explains the marks in one line.

## Not doing

- **Prefetching** neighbors or the whole page. It would spend the rate limit on images nobody asked for.
- **Watching the cache directory.** If another process fills or clears the cache, the glyphs catch up only for rows not yet checked.
- **A cache size limit or eviction.** Images never expire and an 843-pixel JPEG is roughly 100 to 250 KB. If that becomes a problem, it gets its own follow-up.

## Performance

`tui/browse` (`acres-tui/benches/browse.rs`) is new: `check_page` asks about a page of 50 rows, half cached, and `cached` reads and decodes one cached 843-pixel image, debounce excluded. Both run off the app loop. The `tui/frame/*` benches are re-run to show the glyph column costs nothing per frame.

| Bench | M1 Air | Ryzen 9700X |
| --- | --- | --- |
| `tui/browse/check_page` | TBD | TBD |
| `tui/browse/cached` | TBD | TBD |
| `tui/frame/*` worst | TBD | TBD |

For reference, the cloud dev container measured `check_page` at 98 µs (about 2 µs a row, building the URL and one `stat`) and `cached` at 8.3 ms, nearly all JPEG decoding.

## Consequences

- Browsing shows real art or nothing, never a blur, and costs no requests either way.
- In `--offline` mode, everything fetched before shows while browsing, and the glyphs say which rows will.
- List pages get smaller, since `thumbnail` is no longer requested.
- Each list change `stat`s each new row once, off the loop.
- A row not fetched before shows an empty art pane until `enter`.

## Decisions on review questions (2026-11-26)

1. **Glyph and placement.** A dim `•` in a fixed column before the title.
2. **Empty-pane hint.** No hint: the pane stays blank. Rows without a local copy show an open circle `◦` instead.
3. **Drop `thumbnail` from list requests.** Yes.
4. **Debounce.** Keep 150 ms.
