# 0012. `charaoke` does not add edge-aware glyphs

- Status: Rejected
- Date: 2026-11-24

## Context

Since ADR 0011, `charaoke` picks every character by brightness alone. Outlines and line art lose the strokes that `image-to-ascii`'s default edge-augmented mode drew, so we looked at bringing edges back.

The old mode blurs the image, runs a Laplacian edge filter, then scores all 95 glyphs of a bundled 7x13 bitmap font against each cell. It takes 21.5 ms at 160 columns on the M1 Air (`baseline/convert/edge-augmented/843x632/160`). Its signed per-cell gradient mostly cancels on thin lines, and its glyph shapes come from one bitmap font rather than the terminal's.

We considered three options:

| Option | How a cell is chosen | Font-independent | Cost at 160 columns |
| --- | --- | --- | --- |
| A. Port the old edge mode | Blur and Laplacian, then score 95 glyphs of a bundled font | No | About 21.5 ms on the Air |
| B. Orientation glyphs | A Sobel gradient plane built at load, a structure tensor summed per cell, and `-`, `/`, `\|` or `\` where the edge is strong and coherent | Yes | Estimated 1.0 to 1.3 ms on the Air |
| C. Sub-cell shape vectors | Split each cell into 2x3 regions and pick the glyph with the nearest coverage vector | Partly: vectors come from one reference font | About 8 ms, unoptimized, in the probe below |

The terminal emulator, not the program, chooses the font, and a program cannot reliably ask which one it is. That ruled out A.

## Decision

We add no edge-aware glyphs for now. `charaoke` keeps choosing characters by brightness alone.

A throwaway probe rendered three fixture artworks (The Bedroom, The Great Wave, La Grande Jatte) at 100 columns with B and C:

- B, with strength 80 and coherence 70%, turned 16 to 19% of cells into strokes. They mostly traced frames, borders and the print's edge, and laid runs of `-` over textured paper. The result read as noisier, not sharper.
- C, with 2x3 regions measured from DejaVu Sans Mono and a nearest-glyph match, filled the picture with `$`, `U`, `X` and `@`, and lost most of the shading. A per-cell contrast boost didn't fix that.

Neither justified the code, and an option that ships turned off by default still costs upkeep.

## Consequences

- Line art and outlines stay softer than with `image-to-ascii`'s old default.
- Half blocks (ADR 0014) are next, since they are expected to give a larger visible gain by doubling vertical detail in full color.
- `image-to-ascii-builder` stays only for the `baseline/*` benches. ADR 0011 had planned to remove it once edge-aware characters landed, so that removal now needs its own decision.
- If edges come back, B's structure tensor is the starting point. Its thresholds would need tuning against texture, and a stroke might only be drawn where neighboring cells agree on the direction.
