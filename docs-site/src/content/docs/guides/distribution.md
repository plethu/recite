---
title: Preparing and checking packages
description: Find package procedures and release acceptance requirements.
template: splash
---

Use the package's own installation and upgrade instructions. Record the source revision, artifact
hash and tested OS/architecture; a numbered developer preview does not establish serious-v1
acceptance.

| Surface                            | Package procedures                                                                                                                                          |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CLI and LSP                        | [Installation](/getting-started/install/)                                                                                                                   |
| Writer native packages and Flatpak | [Writer packaging](https://github.com/plethu/recite/blob/main/apps/writer/packaging.md)                                                                     |
| Nix                                | [Flake setup](https://github.com/plethu/recite/blob/main/nix/README.md)                                                                                     |
| VS Code/VSCodium                   | [Extension guide](https://github.com/plethu/recite/blob/main/editors/vscode/README.md)                                                                      |
| Neovim                             | [Package guide](https://github.com/plethu/recite/blob/main/editors/recite-neovim/README.md)                                                                 |
| Zed                                | [Extension guide](https://github.com/plethu/recite/blob/main/editors/zed/README.md)                                                                         |
| Bevy, Godot and Unity              | [Engine guides](/adapters/) and [package checks](https://github.com/plethu/recite/blob/main/docs/engine-authoring-workflows.md#packages-before-publication) |

## Signing and release records

The
[release workflow](https://github.com/plethu/recite/blob/main/CONTRIBUTING.md#preparing-and-publishing-releases)
owns candidate validation, signatures, publication and unresolved limits. Package builds and
checksums do not establish signing, notarisation, store acceptance, desktop integration or
accessibility. Those claims need results for the shipped candidate and declared host profile.

For a bug report, include revision, package hash, engine/runtime version when applicable,
OS/architecture, exact command and a small source/schema fixture. Remove private content before
sharing it through the [issue tracker](https://github.com/plethu/recite/issues).
