# Recite on Nix

The flake builds `recite` and `recite-writer` from the repository's two locked Cargo workspaces. It
pins nixpkgs and rust-overlay in `flake.lock` and selects the Rust version from `.mise.toml`. The
Writer package builds Skia from the source pinned by its Cargo dependency, with Nix-provided build
tools and system libraries. The [package definition](packages.nix) owns source revisions and
platform linker adaptations; keep these aligned when updating `freya-skia-bindings`.

From a committed checkout:

```sh
nix build .#recite
nix build .#recite-writer
nix run .#recite -- --help
nix run .#recite-writer -- --help
nix flake check
nix develop
```

For an untracked working copy, use `path:$PWD` in place of `.` so Nix includes new files. The Linux
writer output installs a desktop entry and icon for `recite://` links. Installing or activating that
association belongs to the user's desktop/package manager. On Linux, the package supplies `msginit`
and `xdg-open` as PATH fallbacks; on macOS it supplies `msginit`. Configured editor and producer
commands still resolve from the user's PATH first.

The flake declares x86-64 and AArch64 Linux and macOS outputs. Evaluation alone does not establish
native build or desktop acceptance on every platform; record actual host results separately. The
smoke checks exercise headless CLI entry points, not GUI, accessibility, URL activation, or
installed-host integration. On Linux distributions outside NixOS, launching a Vulkan or OpenGL GUI
may also require host GPU driver integration (for example, nixGL); a successful package build or
`--help` run does not verify that desktop path.
