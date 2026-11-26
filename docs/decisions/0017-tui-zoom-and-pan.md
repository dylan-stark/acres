# 0017. Zoom and pan in the art pane

- Status: Accepted
- Date: 2026-11-25

## Context

The art always fits its pane (`Grid::fit_within`), so a landscape painting in a tall zen screen, or a portrait in a wide one, leaves bands of empty cells. ADR 0013 deferred zoom and pan to their own ADR. What we build on:

- **One image per artwork.** `enter` or `space` fetches the full image at 843 pixels wide (`FULL_WIDTH`), decoded off the loop and kept in a 16-image LRU. Browsing shows the free `lqip` placeholder.
- **A shared rate limit.** The catalog allows 60 requests a minute across the API and images, so every extra image request has a cost.
- **Headroom in the frame budget.** Zen is the worst frame today at 0.42 ms on the M1 Air and 0.27 ms on the Ryzen 9700X, against our 8 ms budget.

We probed the AIC image server on 2026-11-25 (The Great Wave, 24645, public domain, master 6679 x 4577):

| Request | Result |
| --- | --- |
| `full/843,` (today) | 200, 233 KB |
| `full/1686,` | 200, 852 KB |
| `full/3000,` | 200, 2.5 MB |
| `full/3340,` or larger | 403, although `info.json` lists 3340 and 6679 |
| A region on the 256-pixel tile grid, such as `0,0,512,512/256,` | 200, about 20 KB |
| Any other region, such as `1000,500,1000,500/843,` or `pct:25,25,50,50/843,` | 403 |

So AIC serves whole images up to 3000 pixels wide and tiles, but not arbitrary crops, and `info.json` lists sizes it then refuses.

## Decision

The art pane shows a **view** of the 843-pixel image we already fetch: a rectangle kept as a center and a zoom, relative to the image, drawn on a **frame** of cells. We fetch nothing new, and up close the image turns into flat blocks of color on purpose, for a retro look.

### The view

- **Fit** is today's behavior: the whole image, letterboxed.
- **Fill** scales the image until it covers the frame and crops the overflow, centered.
- **Zoom** multiplies the scale by 1.25 a step, about the view's center (keys) or the cell under the pointer (scroll). Zoom out stops at fit. Zoom in keeps going past one image pixel per output pixel, so each image pixel grows into a flat block, and stops when the view's longer side is 16 image pixels. Past 1:1 the steps are whole numbers of output pixels per image pixel (1, 2, 3, 4 and so on) rather than 1.25x, so every block is the same size: at a scale such as 2.4x, image pixels would alternate between 2 and 3 output pixels. The mode sets what an output pixel is (a cell in ASCII, half a cell in half blocks, a quarter in quadrants), so changing mode re-snaps the zoom. Output pixels in ASCII and quadrants are twice as tall as they are wide, so there the first whole step is an image pixel one output pixel tall and two wide (two cells by one in ASCII), and the steps go up in those. `View` stays in fractions of the image; only the TUI's choice of zoom levels snaps.
- **Pan** moves the center, clamped so the view never shows past the image's edge, except along an axis where the whole image already fits.
- **Zen matches the pane.** If the pane is in fit, zen opens in fit, and a fill or zoom carries over both ways.
- **Nothing is remembered.** Moving to another artwork goes back to fit. Zooming is play; saved views (below) are the way back to a crop, and saving one never changes the default.
- The status bar shows the view while it differs from the default, such as `fill 2.4x`.
- **Minimap.** While a zoomed view pans or zooms, a minimap of about 16 x 6 cells in half blocks appears in the bottom-right corner with the view outlined. It fades one second after the last pan or zoom and never shows at fit. It is rendered once when the image loads.

### The frame

The frame is the size, in cells, of the canvas the art is drawn on. Fit, fill, zoom and pan all work inside it.

| Frame | Size | When |
| --- | --- | --- |
| Pane | The art pane, as today | The default in the three-pane layout |
| Screen | The whole terminal | The default in zen |
| Custom | A fixed size, at most the terminal's | Chosen with `S` |

- `S` opens an overlay with Pane, Screen, a custom size typed as `cols x rows`, and two presets: **Stamp**, 20 x 12 cells (a portrait postage stamp), and **Postcard**, 60 x 20 cells (landscape 3:2).
- A custom frame is centered with a thin border that is not part of the art; in zen it floats on an empty screen. No perforated border.
- A custom frame larger than the art pane is clipped to the pane, and the status bar hints that `z` shows all of it. If the terminal shrinks below it, it is clipped the same way.
- The frame resets with the view on a new artwork. A saved view keeps its frame.

### Keys and mouse

In the `Art` context, which zen also uses:

| Key | Action |
| --- | --- |
| `+` or `=` | Zoom in |
| `-` | Zoom out |
| `h` `j` `k` `l`, arrows | Pan by one cell |
| `H` `J` `K` `L`, shift-arrows | Pan by a tenth of the view |
| `0` | Fit |
| `F` | Fill |
| `S` | Pick the frame size |
| `v` | Save this view under a name |
| `V` | List the saved views |
| `X` | Export the rendered art to a file |

`L` pans in the `Art` context, so in the art pane and in zen it no longer toggles logs, which stays on `L` from the other panes.
| Scroll | Zoom about the cell under the pointer |
| Drag | Pan |

### Saved views and export

- `v` saves the current view under a name (a one-line prompt, default `view N`), saving the artwork first if needed. An artwork keeps several named views. `V` lists them, newest first: `enter` jumps to one, `e` renames it, `x` deletes it. The status bar names the view showing, such as `view: Fuji`.
- Each saved entry in `saved.json` gains an optional `views` list, so the file stays at version 1:

  ```json
  {"record": {"id": 24645, "…": "…"}, "saved_at": "2026-11-25T20:40:00Z",
   "views": [{"name": "Fuji", "center": [0.62, 0.41], "zoom": 3.2, "base": "fill",
              "frame": [20, 12], "saved_at": "2026-11-25T20:41:12Z"}]}
  ```

  `center` is in fractions of the image and `zoom` is relative to `base` (fit or fill), so a view comes back the same on any terminal size, as closely as the new frame's aspect allows. `frame` is `[cols, rows]` for a custom frame and absent for Pane or Screen.
- `X` writes exactly the frame's cells, in the current mode and colors, to `exports/<id>-<YYYYMMDDTHHMMSS>.ans` in the data directory with `Cells::write_ansi` or `Blocks::write_ansi`, plus a plain `.txt` in ASCII mode. A Stamp export is a 20 x 12 vignette you can `cat` anywhere. PNG export would need a font rasterizer and is left for later.

### In `charaoke`

`charaoke` gains `View`, a rectangle of the image in fractions of its size, and `Converter::render_view` and `render_blocks_view`, which stretch a view over a grid. `View::fill` gives the centered view that covers a grid, and `View::centered` places a view of a given size, clamped to the image. The box filter in `mean_rows` takes the view as a window in image pixels, which may fall between pixels, so a pan of one cell moves one cell even when zoomed in past one pixel per cell. `render` and `render_blocks` are the full view and render byte for byte as before. Nothing else is optimized: a zoomed view reads fewer image pixels than today's fit.

## Consequences

- Zen can fill the screen, and small custom frames make vignettes that export as text.
- Zoom costs no requests and works offline, but detail stops at the 843-pixel image. Larger sizes (up to 3000 pixels) and tiles stay possible in a later ADR.
- `charaoke::Error` holds the rejected view's floats, so it is no longer `Eq`.

### Benches

Each is recorded on the M1 Air and the Ryzen 9700X.

| Bench | What it measures | Bar on the Air |
| --- | --- | --- |
| `charaoke/view/{fit,fill,zoom4}/{ascii,half,quadrant}` | One render of a view on a 200 x 60 grid from the 843-pixel input: the whole image fitted, the fill view, and that view zoomed 4x | Under 1.5 ms (today's quadrant drag is 1.43 ms) |
| `tui/frame/zen_pan/{half,quadrant,ascii}` (step 2) | Whole zen frame at 200 x 60, zoomed in six steps (about 3.8x, since steps are 1.25x), panning one cell every frame | Under 8 ms |
| `tui/frame/zen/*`, `tui/frame/explorer/*` (existing) | Today's frames | No regression in `make bench-compare` against `main` |

Step 1 results, median per render (ascii / half / quadrant):

| Bench | M1 Air | Ryzen 9700X |
| --- | --- | --- |
| `charaoke/view/fit` | 0.53 / 0.56 / 0.97 ms | 0.34 / 0.46 / 0.77 ms |
| `charaoke/view/fill` | 0.46 / 0.50 / 0.96 ms | 0.32 / 0.42 / 0.82 ms |
| `charaoke/view/zoom4` | 0.12 / 0.15 / 0.44 ms | 0.10 / 0.14 / 0.42 ms |

Against `main`, every `charaoke/*` bench on the Air moved by under 1.5%. The Ryzen moved both ways: `render/843x632` got 6 to 10% faster, while `render_blocks/843x632` got 4 to 31% slower (half at 80 columns is worst, 0.27 to 0.35 ms) and `ansi/*/truecolor` got up to 9% slower. We read the Ryzen swings as code layout, not more work: the `ansi` code did not change at all, and counting instructions with Cachegrind on x86-64 shows the same work per render as `main` (half at 80 columns 0.6% fewer instructions, quadrant 0.7% fewer, ASCII 0.8% more). Every render stays far inside the 8 ms budget.

Step 2 results, median per frame (half / quadrant / ascii):

| Bench | M1 Air | Ryzen 9700X |
| --- | --- | --- |
| `tui/frame/zen_pan` | 0.62 / 0.94 / 0.54 ms | 0.44 / 0.76 / 0.34 ms |
| `tui/frame/zen` | 0.41 / 0.41 / 0.41 ms | 0.27 / 0.27 / 0.22 ms |
| `tui/frame/explorer` | 0.36 / 0.36 / 0.36 ms | 0.22 / 0.22 / 0.21 ms |

Panning a zoomed zen frame costs more than a fit because the fill-sized art covers all 200 x 60 cells, and it stays under a ninth of the budget. Against `main`, `zen` is flat or faster on both machines. `explorer` is 1 to 2% slower on the Ryzen and 1 to 6% faster on the Air. `help` is 4 to 6% slower on both, about 13 µs on the Ryzen, which we read as the extra layout work each frame now does (the pane's size for the frames, and centering the art on its frame), not anything help draws.

### Rollout

Three steps, each its own branch and PR from `main`, squash-merged:

1. **Views in `charaoke`.** `View`, `render_view`, `render_blocks_view` and the `charaoke/view` benches. No TUI change; today's output stays byte for byte the same.
2. **Fill, zoom and pan in the TUI.** The view model, keys, scroll and drag, zen sharing the pane's view, the status bar, the minimap, frame sizes and the `S` overlay, help, and insta snapshots. Adds `tui/frame/zen_pan/*`.
3. **Saved views and export.** `v`, the `V` overlay and the `views` list in `saved.json`, then `X` and the `exports/` folder.

### Alternatives considered

| Option | Why not, for now |
| --- | --- |
| Ask IIIF for exactly the view on every zoom or pan | AIC returns 403 for regions off its tile grid, and each pan would spend a request. |
| Stream 256-pixel tiles, like a map | Works and reaches the full master, but a pan across a big work needs dozens of tiles against the shared limit, plus a tile cache and stitching. A later deep-zoom ADR. |
| Fetch larger sizes, up to 3000 pixels, on zoom, from a size picker, or always | Costs requests and up to ten times the bytes, `info.json` lists sizes AIC refuses, and we prefer the retro look for now. |
| A pyramid of halved copies in `charaoke` | Only needed for images larger than 843 pixels. |
| Fill the letterbox bands with a blurred copy of the edges | Shows the whole work but doesn't fill the screen with the artwork itself. |
| Sixel or kitty graphics | Not ASCII art, and not every terminal supports them. |
