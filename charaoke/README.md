# charaoke

Fast image-to-ASCII conversion, built for live terminal rendering.

An image is loaded once into a `Converter`. After that, each render fits it to a grid of terminal cells and reuses every buffer, so re-rendering at a new size, as a terminal is resized or the view is zoomed and panned, fits well inside a 60 FPS frame.

## Modes

- **ASCII**: each cell's mean color picks a character from a `Ramp` by brightness. Ramps include presets and custom character sets.
- **Half blocks** (`▀`): two pixels per cell, in two colors.
- **Quadrant blocks**: four pixels per cell, split two ways in two colors, for the most detail.

Output is `Cells` or `Blocks`, written as plain text, truecolor ANSI or 256-color ANSI. A `View` renders a crop of the image, to zoom and pan.

```rust
use std::num::NonZeroU32;
use charaoke::{Cells, Converter, Grid, PixelFormat};

let nz = |n| NonZeroU32::new(n).unwrap();
let (width, height) = (nz(8), nz(8));
// A vertical gradient, dark at the top.
let pixels: Vec<u8> = (0..64).map(|i| (i / 8 * 36) as u8).collect();

let mut converter = Converter::default();
converter.set_image(&pixels, width, height, PixelFormat::Luma8).unwrap();

let grid = Grid::fit(width, height, nz(4));
let mut cells = Cells::default();
converter.render(grid, &mut cells);

let mut ascii = String::new();
cells.write_plain(&mut ascii);
assert_eq!(ascii, "::::\n####");
```

## Speed

A resize drag from 120 to 200 columns, re-rendering an 843 x 632 image into a Ratatui buffer on each step (`make bench FILTER=charaoke/tui_proxy` in the workspace):

| Mode | M1 MacBook Air | Ryzen 7 9700X |
| --- | --- | --- |
| ASCII | 0.93 ms | 0.58 ms |
| Half blocks | 1.03 ms | 0.67 ms |
| Quadrant blocks | 1.43 ms | 1.01 ms |

That is against a budget of 8 ms a frame. The workspace's ADRs 0008 to 0014 record how it got there.

`charaoke` has no dependency on the rest of the [acres](https://github.com/dylan-stark/acres) workspace. The [API docs](https://docs.rs/charaoke) cover every type.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
