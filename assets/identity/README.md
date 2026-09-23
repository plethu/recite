# Recite identity

The selected identity is audition 06, “Printer’s recite”: original outlined
lettering with a matching lowercase r. These assets share the repository’s
MIT OR Apache-2.0 license and do not require a font installation.

- `recite-wordmark.svg`: the primary plum wordmark, on a transparent background.
- `recite-wordmark-reversed.svg`: the warm-paper wordmark for dark surfaces.
- `recite-mark.svg`: the standalone r.
- `recite-wordmark-print.svg`: optional larger-format treatment with offset rose
  ink, broken edges, and flecks. Use the clean artwork for small icons.
- `../../apps/writer/packaging/icons/recite-writer.svg`: warm-paper r on an ink
  tile, used for Writer’s packaged desktop icon.

Plum is `#683e62`, ink is `#241f27`, and the reversed mark is warm paper
`#f3e9df`. Preserve the letterforms when recolouring for a light or dark surface.
The exploratory sheets remain in `docs/design/identity`.

To regenerate desktop formats, render the icon SVG to a 1024-pixel PNG with
`rsvg-convert`, then use Pillow to export a 512-pixel PNG, a multi-resolution ICO
(16, 24, 32, 48, 64, 128, 256), and an ICNS (through 1024). Resize with Lanczos.
The SVG is the editable source; do not edit the raster exports independently.
