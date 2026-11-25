# 0013. TUI overhaul into a catalog explorer

- Status: Accepted
- Date: 2026-11-25

We turn `acres-tui` from a viewer of a saved JSON file into an explorer of the live Art Institute of Chicago collection: it starts on a live search, browses page by page, shows each artwork's record next to its art, and keeps a `?` help overlay. A static JSON stays available as an offline option, not the default.

## Context

ADR 0011 moved the TUI onto `charaoke`. The art now fits the pane, re-renders on every resize in under 1 ms on the M1 Air, and a timing HUD (`t`) shows each stage. ADR 0014 then added half-block and quadrant modes beside ASCII, cycled with `b`, and ADR 0012 rejected edge-aware glyphs. Everything around the art is still a demo:

- **Input is a file.** `acres-tui <FILE or STDIN>` requires an `Artworks` JSON, usually from `acres-cli artworks > artworks.json`. There is no way to search, page or change what is loaded from inside the TUI.
- **The list knows titles only.** `acres::artworks::Data` keeps `id`, `image_id` and `title`. The list shows `title (id)`; nothing about the artist, date, medium or place is shown anywhere.
- **Two modes, few keys.** `Mode` is `Browse` or `View`. `esc` opens the artworks pop-up, `j`/`k` move, `enter`/`x` select, `a` opens ramps, `b` cycles art modes, `t` the HUD, `l` logs, `q` quits. `Action::Help` and `EnterSearch`/`ExitSearch` exist but do nothing.
- **The library is half-built (ADR 0016 addresses this).** `acres` can build `GET /artworks`, `GET /artworks/search` (`q`, `query`, `sort`, `from`, `size`, `facets`) and single-artwork requests, and `Api::fetch` caches every response on disk forever, keyed by URL. Search results are an untyped `serde_json::Value`.
- **Each fetch is its own task.** `components/iiif.rs` spawns a Tokio task per image, with a new `reqwest::Client` each time and no cancellation, so moving quickly through a list queues every image.

The catalog has over 120,000 artworks. A tool that only shows what someone exported beforehand cannot explore it. This ADR covers the whole overhaul so the data model, screens and keys are designed together; the implementation lands as several PRs (see Rollout).

## Decision: live data by default

`acres-tui` with no arguments queries the API through the `Catalog` client from [ADR 0016](0016-acres-catalog-client.md). That client takes care of typed responses, rate limiting, caching and the `AIC-User-Agent` header. The positional file argument becomes `--from-file <FILE|->`, which loads a saved `Artworks` JSON exactly as today for the demo and tests. `--offline` turns on ADR 0016's offline mode.

What the explorer takes from the API's rules:

- Pages of 50 (the API allows up to 100). The list stops at the API's 10,000-record cap, and the status bar says the rest needs a narrower search.
- While the client waits on the rate limit, the status bar says `rate limited`.
- Images stay 843 px wide, the API's recommended size, with the IIIF base from each response's `config.iiif_url`.
- List requests ask for `thumbnail`, so its tiny `lqip` image can show before the full image arrives (see Decisions on review questions).

## Decision: screens and features

The TUI becomes one explorer screen with three panes, plus overlays. A results list sits on the left, the art in the middle, and the artwork's record on the right. A one-line search bar runs along the top and a status bar along the bottom. Any pane can be hidden, and `z` (zen) shows the art alone at full size.

```text
┌[1] Search──────────────────────────────────────────────────────────────┐
│ / monet                                                                │
└────────────────────────────────────────────────────────────────────────┘
┌[2] Results──────────┐┌[3] Art──────────────────────┐┌[4] Record───────┐
│> Water Lilies       ││                             ││ Claude Monet    │
│  Stacks of Wheat    ││     (art, fit to pane)      ││ French, 1840-   │
│  Cliff Walk at Po…  ││                             ││ Water Lilies,   │
│  …                  ││                             ││ 1906            │
└─────────────────────┘└─────────────────────────────┘└─────────────────┘
 12 / 340 loaded · ? help  / search  m save  enter art · n · next artwork
```

Every pane, the search bar included, has a thin single-line border with its number and name set into the top edge near the left corner, like `─[2] Results──`. Pressing the number focuses that pane from anywhere outside the search bar: `1` search, `2` results, `3` art and `4` record. `tab` still cycles through them. The focused pane's border takes the accent color, and zen mode drops the borders. Help, filters and ramps open as centered overlays over the panes. The borders cost the art 2 columns and 2 rows, which the fit-to-pane size already accounts for.

Features, in the order they land:

1. **Live search.** `/` opens the search bar; `enter` runs a full-text `q` search. With no query the TUI starts on Highlights, a short built-in list of well-known public-domain works (see Start screen under Decisions on review questions), so it is useful with zero arguments.
2. **Paged results.** The list loads the next page when the cursor is within 10 rows of the end, so it reads as one long list. The status bar shows `loaded / total`.
3. **Preview as you move.** Moving the cursor shows that artwork's art and record after a short debounce (150 ms), so holding `j` does not fire a fetch per row. The preview is the record's `lqip` placeholder, and `enter` or `space` fetches the full 843-pixel image (changed after step 2, see below).
4. **Wall label in the record pane.** The pane reads like the short labels next to works in the museum's galleries: the artist with nationality and life dates (`artist_display`), title and date, medium, and the credit line with reference number. With the pane focused, `d` expands it in place to the full record (dimensions, place of origin, department, whether and where it is on view, and the description with HTML stripped), scrolled with `j`/`k`. `enter` opens the same full record in a large overlay over the art, which `esc` closes. `o` opens the artwork's page on artic.edu in the browser; `y` copies its link.
5. **Filters and sort.** `f` opens a filter overlay: has image (on by default, since a record without `image_id` has no art), public domain, on view, department, artwork type, classification, and a date range. `s` cycles sort: relevance, date, title. Filters become an Elasticsearch `query` on the same search endpoint.
6. **Pivots.** From a selected artwork, `A` lists more by the same artist and `D` more from its department. Each pivot is just a new search, so it pages and filters the same way.
7. **History.** Every search and pivot is pushed on a stack. `backspace` goes back, restoring the list, cursor and selection.
8. **Saved artworks.** `m` marks or unmarks the selected artwork. `'` toggles between results and the saved list. Saved artworks live in `ACRES_TUI_DATA` as `saved.json`, a format of the TUI's own: a version number and, per entry, the artwork record as fetched, when it was saved, and an optional one-line note. Keeping the whole record means the saved list opens offline, and `--from-file` accepts `saved.json` as well as an Artworks JSON. `e` edits the selected saved artwork's note in a one-line input like the search bar, and the note shows under the wall label.
9. **What stays.** Fit-to-pane art in color in all three modes (ASCII, half blocks, quadrant blocks), the ramps picker, the timing HUD and the log all stay, some under new keys. The log becomes a drawer over the left of the art, like the HUD on the right, so opening it never resizes the art.

Out of scope here, each its own ADR later: zooming and panning the art through IIIF regions (tiling), and other collections (agents, exhibitions, galleries) as their own browsers. This ADR reserves the zoom and pan keys and gestures (see the keymap), so that ADR adds behavior without changing any binding.

## Decision: keymap and help

The two modes (`Browse`, `View`) become focus contexts: `List`, `Art`, `Record`, `Search` (the text bar), and `Overlay` (help, filters, ramps). A key is looked up in the focused context first, then in `Global`. Bindings stay in `acres-tui/.config/config.json5`, keyed by context, and users can still override them. Every existing binding is up for change; the main breaks are that `esc` no longer opens the list (it only closes and backs out), `ctrl-d` no longer quits (it pages), and ramps move from `a` to `r`.

| Key | Context | Action |
| --- | --- | --- |
| `?` | Global | Toggle the help overlay |
| `/` | Global | Focus the search bar |
| `tab` / `shift-tab` | Global | Focus the next or previous pane (`1` to `4` jump straight to one, outside the search bar) |
| `n` / `p` | Global | Select the next or previous artwork in the list, from any pane |
| `f` | Global | Open filters |
| `s` | Global | Cycle sort: relevance, date, title |
| `A` / `D` | Global | More by this artist / from this department |
| `backspace` | Global | Back to the previous search |
| `m` | Global | Mark or unmark as saved |
| `'` | Global | Switch between results and saved |
| `e` | Global | Edit the note on a saved artwork |
| `o` / `y` | Global | Open on artic.edu / copy its link |
| `z` | Global | Zen: art only, full screen |
| `b` | Global | Cycle art mode: half blocks (the default), quadrant blocks, ASCII (kept from ADR 0014) |
| `r` | Global | Pick a ramp (ASCII mode only) |
| `t` | Global | Toggle the timing HUD |
| `L` | Global | Toggle logs |
| `q` | Global | Quit, after a prompt that `q` or `enter` confirms |
| `ctrl-c` | Global | Quit without asking |
| `ctrl-z` | Global | Suspend |
| `j` / `k`, `down` / `up` | List, Record, Overlay | Move, or scroll the record |
| `g` / `G` | List, Record, Overlay | Top / bottom |
| `ctrl-d` / `ctrl-u` | List, Record | Half a page down / up |
| `enter` | List, Art | Fetch and show the full image |
| `space` | Global | Fetch and show the full image |
| `d` | Record | Expand the wall label to the full record in place, or collapse it |
| `enter` | Record | Open the full record in a large overlay |
| `enter` | Search | Run the search and focus the list |
| `up` / `down` | Search | Recall earlier queries |
| `esc` | Search, Overlay, zen | Close or cancel, keeping what was there |

In the search bar, every printable key is text, so `q`, `?` and `/` type themselves there; only `enter`, `esc`, `up`, `down` and `ctrl-u` (clear) are bound.

Mouse (crossterm mouse capture, on by default):

- Scrolling over the list moves the selection, and over the record it scrolls the label or full record.
- Clicking a list row selects it, and clicking a pane focuses it.
- Clicking outside an overlay closes it, like `esc`.
- Setting `mouse: false` in the config turns capture off, so the terminal's own text selection works. Most terminals also let shift-drag select text while capture is on.

Reserved for zoom and pan, and left unbound until that ADR: in the Art pane, `+` / `-` to zoom, `0` to fit, and `h` `j` `k` `l` or the arrows to pan. Scrolling over the art will zoom and dragging will pan, so until then both do nothing there.

Help:

- `?` opens a centered overlay over whatever is showing, and `?` or `esc` closes it. The art underneath keeps its grid, as with the HUD, so opening help costs no re-render.
- It is built from the loaded keymap, user overrides included, so it cannot drift from the real bindings. Each `Action` carries a one-line description (strum's `EnumMessage`), since JSON5 comments are not readable at runtime.
- The focused context's keys are listed first, then `Global`. `j`/`k` scroll it when it does not fit.
- A one-line hint in the status bar always shows `? help` and the 3 most useful keys for the focused pane.
- The status bar also echoes the last key and what it did, like `n · next artwork`. A key that does nothing in the focused pane shows as `x · not mapped here`, so a wrong key or the wrong pane is obvious while learning the keymap. The echo stays until the next key, and `show_keys: false` in the config turns it off. The help overlay and the HUD's event log use the same action names.

## Performance and architecture

The budget is unchanged: a full-pane convert in 8 ms or less on the M1 Air, and the whole frame should fit in it too. Nothing that waits on the network or disk runs on the draw path.

```text
keys, mouse ─▶ app loop ─▶ catalog task (one Catalog: search, page, record, image)
                  ▲               │ results as actions, stale generations dropped
                  │               ▼
                  └──── decode (spawn_blocking) ─▶ Converter LRU
draw path (inside a frame): render / render_blocks ─▶ CellsWidget / BlocksWidget
```

Keys and results arrive as actions; only the accented draw path runs inside a frame.

- **One catalog task.** A single Tokio task owns the `Catalog` client and takes requests over a channel. Results come back as actions. Each request carries a generation number, and results older than the latest search or selection are dropped, so a fast `j` run shows only the artwork the cursor stopped on. An in-flight image fetch for an artwork that is no longer selected is aborted.
- **Decode off the loop.** Decoding (1.31 ms on the Air) moves to `spawn_blocking`, and the loaded `Converter` comes back as an action. The draw path only runs `render` and `CellsWidget` (ADR 0011), or render\_blocks and BlocksWidget in the block modes (ADR 0014).
- **Recent art in memory.** The last 16 loaded `Converter`s stay in an LRU, so `backspace`, `n`/`p` back over seen artworks and toggling saved need no fetch or decode.
- **Cheaper art pane.** In the three-pane layout the art gets roughly half the columns, so its render is smaller than today's full-pane case. Zen in quadrant mode is the worst case: a quadrant drag costs 1.43 ms on the Air against 0.93 ms for ASCII (ADR 0014), so each frame bench runs in all three modes.
- **Prefetch inside the bucket.** The next results page is fetched early (within 10 rows of the end). Images are not prefetched for neighbors, to stay inside 60 requests a minute and the API's one-at-a-time guidance.

The timing HUD stays on `t` and gains events: `search <query>` and `page <n>` with `wait` (time in the rate limiter), `fetch` and `parse`, plus `hit` when the cache answered; `record <id>` with `fetch` and `parse`. The existing `image`, `resize` (which names the art mode) and `frame` events are unchanged.

New benches, recorded on both machines in this ADR once implemented (Ryzen numbers are not budget checks, the Air is):

| Bench | What it measures | M1 Air (target) | Ryzen 9700X (dev) |
| --- | --- | --- | --- |
| `tui/frame/explorer/{half,quadrant,ascii}` | Whole `terminal.draw` of the three-pane screen into a `TestBackend`, alternating 200x60 and 199x60 so the art re-renders every frame; the five fixture artworks, record and art | 0.37 / 0.37 / 0.37 ms | 0.22 / 0.22 / 0.21 ms |
| `tui/frame/zen/{half,quadrant,ascii}` | Same, art only | 0.42 / 0.42 / 0.42 ms | 0.27 / 0.27 / 0.22 ms |
| `tui/frame/help/ascii` | Explorer in ASCII with the help overlay open | 0.40 ms | 0.23 ms |
| `charaoke/tui_proxy/drag` (existing, ADR 0014 run) | 120 to 200 column drag, ASCII | 0.93 ms | 0.58 ms |
| `charaoke/tui_proxy/drag_half` (existing) | Same drag, half blocks | 1.03 ms | 0.67 ms |
| `charaoke/tui_proxy/drag_quadrant` (existing) | Same drag, quadrant blocks | 1.43 ms | 1.01 ms |

The frame benches live in `acres-tui/benches`, since only the TUI can draw its own screen. The bar for merging each step: `tui/frame/explorer` and `tui/frame/zen` under 8 ms on the Air in every mode, and `make bench-compare` against `main` shows no regression in `charaoke/*`.

Step 1 measured the shell on the file input, with times per mode in the order listed. The worst case, zen in ASCII on the Air, is 0.42 ms, about a twentieth of the budget, so later steps have plenty of room.

## Consequences

- `acres-tui artworks.json` stops working; it becomes `acres-tui --from-file artworks.json`. The project is pre-1.0, so we break it rather than guess whether a bare argument is a file or a query. The README, `demo.tape` and `make demo` switch to the flag.
- Muscle memory breaks: `esc` no longer opens the list, `a` is no longer ramps, `ctrl-d` no longer quits, and `l` becomes `L`. The `?` overlay and the status-bar hint are the answer to that.
- The TUI depends on the network by default, so it now has to handle errors on screen: a failed search keeps the previous results and shows the error in the status bar, and `--offline` covers no-network use.
- This ADR builds on ADR 0016, which is merged.

## Alternatives considered

- **Keep the file as the default and add search as a mode.** Rejected: it leaves the TUI useless without a CLI step first, which is the problem we are solving.
- **Use the nightly data dumps** ([api-data](https://github.com/art-institute-of-chicago/api-data)) as a local index. Fast and unlimited, but a multi-gigabyte download before first use and a sync job to maintain. Worth revisiting if the rate limit hurts in practice.
- **Keep modal `Browse`/`View`.** Rejected: with list, art and record on screen together, focus is the natural model, and modes hid the list behind `esc`.
- **Fetch and decode in one task per image, as today.** Rejected: no cancellation and no shared rate limit.

## Testing

- Network stays mocked: the TUI's catalog-task tests use `wiremock`, as `acres-cli/tests` already does, with `ACRES_USE_CACHE=false` and a fixture search page. They check that stale results are dropped and that an aborted image fetch never reaches the art.
- New `insta` snapshots: the explorer layout, zen, the help overlay (checked against the default keymap), filters, the wall label and full-record overlay, the status bar in `rate limited` and `offline` states, and an empty search.
- Existing snapshots for the artworks pop-up and ramps browser are re-recorded or removed as their screens change.

## Rollout

Each step is its own branch and PR from `main`, with its own CHANGELOG entry:

1. TUI shell: focus contexts, the new keymap, `?` help overlay and status bar, still on the file input. Adds `tui/frame/*` benches.
2. Live search as the default on ADR 0016's `Catalog`, paging, the catalog task with cancellation, `--from-file` and `--offline`.
3. Wall label and full record, `o` and `y`, decode off the loop and the `Converter` LRU.
4. Filters and sort.
5. Pivots, history and saved artworks.

Steps 2 to 5 build on ADR 0016's `Catalog`. Bench results for both machines are added to this ADR as steps 1 to 3 land.

## Decisions on review questions

- **Start screen.** Decided: a built-in Highlights list of about ten public-domain artwork ids, such as Hokusai's The Great Wave (24645), Monet's Water Lilies (16568), Stacks of Wheat (64818) and Cliff Walk at Pourville (14620), Seurat's La Grande Jatte (27992), Van Gogh's The Bedroom (28560) and Caillebotte's Paris Street; Rainy Day (20684). One GET /artworks?ids= request loads them all, and we sort the rows back into our order because the API returns its own. Ids are stable record keys, so they rarely go stale, but a work can be removed or change rights. The TUI drops any id that comes back missing, not public domain or without an image, and if none remain it falls back to public-domain works with images, most recently updated first. A highlights list in the config replaces the built-in one, and the weekly canary workflow checks the built-in ids against the live API so a stale one shows up as a failed run rather than a quietly shorter list.
- **Preview on move or on `enter`?** Decided: preview on move, after a 150 ms debounce, so holding j still fires no fetch per row.
- **Default art mode.** Decided: the TUI starts in half blocks, which ADR 0014 found far more recognizable at the explorer's narrower art pane. b still cycles to quadrant blocks and ASCII.
- **Full record: in place, overlay, or both?** We propose both (`d` expands the pane, `enter` opens an overlay). If one is enough, we would keep the overlay, since the pane is narrow for long descriptions. Decided: both, as proposed.
- **Placeholder art.** Should the tiny `thumbnail.lqip` GIF render as blurry colored art while the full image loads? It needs the `image` crate's GIF decoder (small), and makes moving through the list feel instant. Decided: yes, with a config switch to turn it off.
- **Public domain only by default?** The API reserves its larger image sizes for public-domain works, but 843 px, which we use, is its recommended size for all. We suggest leaving the filter off by default so modern works show, and checking non-public-domain images during step 3. Decided: leave the filter off by default.
- **Keys.** Any bindings in the table you want different, especially `n`/`p` for next and previous, `m` and `'` for saved, and capital `A`/`D` for pivots? Decided: keep them as proposed.
- **A "surprise me" key** for a random artwork: worth it in this overhaul or later? Decided: yes, in this overhaul.
- **Mouse support** (click to select, scroll the list): here, or its own ADR? Decided: yes, in this ADR.
- **Saved list format.** Is reusing the CLI's `Artworks` JSON enough, or do you want notes or tags per saved artwork? Decided: a TUI-owned, versioned saved.json with the record, the time saved and a note per entry (see Saved artworks). Tags wait, since they only pay off with a way to filter by them.
- **Scope of one ADR.** With the library split into ADR 0016, five PRs remain here. Should the keymap and help (step 1) also become their own ADR? Decided: no, one ADR landed through several PRs.

## Changes after using step 2

- **Placeholder first, full image on request.** After the debounce, the art shows the record's `thumbnail.lqip`, a GIF about 8x5 pixels that comes inline with every list, scaled up smoothly into a blur of the artwork's colors. Browsing so downloads no images and spends nothing from the rate limit. `enter` or `space` fetches the 843-pixel image, once per artwork. We tried a 200-pixel thumbnail fetch first, but it still costs a request per artwork and looks little different from the full image. This brings forward the placeholder planned for step 3, without its config switch, since it is now the only preview.
- **Quit asks first.** `q` opens a small prompt, and `q` again or `enter` quits, so a stray `q` never ends a session. `ctrl-c` quits at once.
- **The log is a drawer.** It slides over the left of the art the way the timing HUD slides over the right, instead of narrowing the pane and re-rendering the art.

## Notes from step 3

- **The wall label comes with the list.** `ArtworkSummary` gained `medium_display`, `credit_line` and `main_reference_number`, so every page of results carries the whole label and moving through the list costs no record requests. Only `d` and `enter` fetch the full record, once per artwork, and `d` collapses again when the selection moves, so browsing with the pane expanded never fetches a record per row.
- **Decode off the loop.** The image fetcher decodes on a blocking thread and sends RGB pixels, so the app loop only loads them into the converter (about 0.04 ms). It keeps the last 16 full images decoded rather than 16 `Converter`s: loading is cheap, and one converter keeps the ramp and mode in one place.
- **Copying the link** uses an OSC 52 escape, which most terminals support and which works over SSH, instead of a clipboard library.
- **Frame benches after step 3**, unchanged from step 1 within a few percent, since drawing did not change:

  | Bench | Air | Ryzen |
  | --- | --- | --- |
  | explorer (half / quadrant / ASCII) | 0.37 / 0.39 / 0.37 ms | 0.22 / 0.22 / 0.20 ms |
  | zen (half / quadrant / ASCII) | 0.42 / 0.42 / 0.42 ms | 0.27 / 0.27 / 0.22 ms |
  | help | 0.40 ms | 0.23 ms |

