# Recite on Nix

The flake builds `recite` and `recite-writer` from the repository's two locked
Cargo workspaces. It pins nixpkgs and rust-overlay in `flake.lock` and selects
the Rust version from `.mise.toml`. The writer builds the exact Skia source
referenced by `freya-skia-bindings 0.100.0`: rust-skia/skia commit
`41382841f36aa208e7433f3c46d14237a1b15054` (`m152-0.100.0`). Its build
uses Nix-provided GN, Ninja and system image/font libraries, with Skia's offline
source-build path. The Wuffs, Vulkan Memory Allocator and SPIRV-Cross sources
are pinned to that Skia checkout's `DEPS` revisions. The package follows the
[Nixpkgs Neovide recipe](https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/ne/neovide/package.nix)
for source-built Skia and a Clang build environment. Skia m152's system
HarfBuzz rule assumes `/usr/include/harfbuzz`, so the assembled source points
that rule at Nix's HarfBuzz headers. Its GN WebP target also needs
`webpdemux` and `webpmux` at the final Rust link. On macOS the bindings embed
FreeType from the pinned Skia `DEPS` revision and need explicit links for the
selected system image and text libraries. Keep these adaptations aligned with
the locked `freya-skia-bindings` version when updating it.

From a committed checkout:

```sh
nix build .#recite
nix build .#recite-writer
nix run .#recite -- --help
nix run .#recite-writer -- --help
nix flake check
nix develop
```

For an untracked working copy, use `path:$PWD` in place of `.` so Nix includes
new files. The Linux writer output installs a desktop entry and icon for
`recite://` links. Installing or activating that association belongs to the
user's desktop/package manager. On Linux, the package supplies `msginit` and
`xdg-open` as PATH fallbacks; on macOS it supplies `msginit`. Configured editor
and producer commands still resolve from the user's PATH first.

The flake declares x86-64 and AArch64 Linux and macOS outputs. Evaluation alone
does not establish native build or desktop acceptance on every platform; record
actual host results separately. The smoke checks exercise headless CLI entry
points, not GUI, accessibility, URL activation, or installed-host integration.
On Linux distributions outside NixOS, launching a Vulkan or OpenGL GUI may also
require host GPU driver integration (for example, nixGL); a successful package
build or `--help` run does not verify that desktop path.
