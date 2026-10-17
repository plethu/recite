# Recite identity

The wordmark sets "recite" in Fraunces with a colon device after it: two rotated squares that echo
the `::` of a block header. The letters are converted to outlines, so the assets need no font
installation. Fraunces is licensed under the SIL Open Font License 1.1, which permits embedding its
outlines; the assets share the repository's MIT OR Apache-2.0 license.

- `recite-wordmark.svg`: the quiet wordmark for app chrome and the docs header. Letters take the
  root `fill` (plum) and the colon uses `currentColor` (teal), so a renderer can recolour each ink.
  The viewBox is tight; its aspect ratio is 3.337 (width over height).
- `recite-wordmark-reversed.svg`: the same on dark surfaces, warm-paper letters and a mint colon.
- `recite-wordmark-print.svg`: the print wordmark for the README, social image and splash. A hard
  teal copy of the letters sits 0.028 em right and down beneath an opaque plum, on grained paper.
  The colon is printed once, at the same size as in the quiet wordmark. There is no translucent
  fringe.
- `recite-wordmark-hero.svg`: the same print wordmark without paper, for surfaces that already carry
  grain, such as Writer's welcome screen. Its aspect ratio is 3.251.
- `recite-mark.svg`: the r and colon on a grained paper tile with the same print recipe, for large
  formats. `../../apps/writer/packaging/icons/recite-writer.svg` is the same mark as clean artwork
  for Writer's packaged icon: no grain, no filters.
- `grain.png`: a 256-pixel tile of paper grain for canvas and sidebar surfaces. Keep it off the
  reading surface and out from behind prose and code. Chrome text at 13 to 14 pixels may sit on
  grained canvas, because the grain peaks at 7% alpha.

## Letterforms

Outlines come from the Fraunces variable font, `Fraunces[SOFT,WONK,opsz,wght].ttf`, "Version 1.000;
[b76b70a41]" as distributed by google/fonts. The instance is `wght` 700, `opsz` 144, `SOFT` 100 and
`WONK` 1, set with −0.04 em letter-spacing and no kerning. Each colon square is 0.11 em on a side;
the upper square is centred 0.06 em below the x-height, the lower 0.06 em above the baseline, and
the pair starts 0.07 em after the e. To regenerate, instance the font with
`fontTools.varLib.instancer` and draw each glyph through `fontTools.pens.svgPathPen`; keep these
values so the assets stay consistent.

## Inks

| ink            | value     | use                                                         |
| -------------- | --------- | ----------------------------------------------------------- |
| Plum           | `#683e62` | letters and the only text accent                            |
| Ink            | `#302431` | Writer text                                                 |
| Paper          | `#f3e9df` | tiles and the reversed wordmark                             |
| Riso teal      | `#00838a` | the colon, plates, rules and the print offset               |
| Text-safe teal | `#0f6b6f` | links and destinations on paper                             |
| Mint           | `#82d8d5` | the second ink on dark surfaces, where it is also text-safe |
| Overprint      | `#002035` | plum multiplied by teal; the status bar                     |

Riso teal is 3.8:1 on paper, so it is a plate ink. The second ink is never text on a paper surface:
canvas, reading surface, inset, floating panels or plates. On overprint navy, mint is 10:1 and may
be text in either mode.

## Print recipe

Grain is SVG `feTurbulence` (`fractalNoise`, base frequency 0.9, two octaves, seed 7) mapped to ink
`#402e33` at 7% alpha. The print offset is a hard, opaque second-ink copy beneath the top ink. Use
it only in identity assets and hero or empty states, never on controls.

`grain.png` is the raster equivalent, generated deterministically with ImageMagick:

```sh
magick -seed 7 -size 256x256 xc:gray50 +noise Random -colorspace Gray -virtual-pixel tile \
  -blur 0x0.6 -auto-level \( -size 256x256 xc:'#402e33' \) +swap -alpha off \
  -compose CopyOpacity -composite -channel A -evaluate multiply 0.07 +channel -strip PNG32:grain.png
```

## Desktop icon

To regenerate desktop formats, render the icon SVG to a 1024-pixel PNG, then use Pillow to export a
512-pixel PNG, a multi-resolution ICO (16, 24, 32, 48, 64, 128, 256), and an ICNS (through 1024).
Resize with Lanczos. The current exports were rendered with headless Chrome over a transparent
background. The SVG is the editable source; do not edit the raster exports independently.
