---
title: Preparing and checking packages
description: Build local artifacts and distinguish package checks from release acceptance.
---

The current packages are prepared from source. Publication is reserved for v1;
local installation and upgrade checks can run before that release. Keep the
source revision, toolchain, artifact hashes and host profile with each result.

| Surface | Preparation and smoke check | Upgrade and remaining acceptance |
| --- | --- | --- |
| CLI and LSP | Build or install the locked workspace from the [installation guide](/getting-started/install/). Run `recite --version`, `recite --help` and the complete workflow check. | Rebuild compiled assets with the selected revision. Validate before replacing the toolchain in a project; keep source and its previous working toolchain available. |
| Writer native packages | Use the [native package instructions](https://github.com/plethu/recite/blob/main/apps/writer/packaging/README.md) for Linux `.deb`, macOS `.app`/`.dmg` and Windows NSIS. | Package checks inspect metadata, licenses, ABI/architecture and headless entry points. Windows CI also exercises install/reinstall/uninstall. Desktop integration and accessibility need their named platform checks. |
| Writer Flatpak | Use the [isolated Flatpak build](https://github.com/plethu/recite/blob/main/apps/writer/packaging/flatpak/README.md). The builder writes a local bundle and checks it without installing it into the user's normal profile. | Check installed activation and upgrade/uninstall association ownership separately. The manifest's host filesystem and subprocess permissions must remain documented. |
| Nix | From a committed checkout, run `nix build .#recite`, `nix build .#recite-writer` and `nix flake check`. See [Nix setup](https://github.com/plethu/recite/blob/main/nix/README.md). | Pin the flake revision in the consuming configuration. The flake exposes CLI and Writer, not an LSP package. Build/headless checks do not establish GPU or desktop acceptance. |
| VS Code/VSCodium | `scripts/check-vscode.sh` builds/checks the extension and produces its VSIX through `pnpm editor:package`. | Follow the [extension setup](https://github.com/plethu/recite/blob/main/editors/vscode/README.md) with matching CLI/LSP binaries. Marketplace publication is a later release action. |
| Neovim and Zed | Use the [Neovim](https://github.com/plethu/recite/blob/main/editors/recite-neovim/README.md) and [Zed](https://github.com/plethu/recite/blob/main/editors/zed/README.md) instructions and their existing check scripts. | Pin the integration revision and check the configured CLI/LSP paths after replacement. Interactive behavior is separate from package structure checks. |
| Bevy | `scripts/check-bevy-package.sh` assembles Cargo archives and builds an external consumer from them. | The check uses temporary dependency patches while crates remain unpublished. Keep the engine/version profile from the [adapter guide](/adapters/bevy/). |
| Godot | `scripts/package-godot-addon.sh` and `mise exec -- just engines godot` build and exercise an extracted addon. | The host test replaces an old addon and preserves authored files. Check each claimed native platform with its own library and Godot host. |
| Unity | `scripts/unity/build-upm.sh` prepares a local UPM archive; `scripts/unity/run-unity-tests.sh` runs with a licensed `UNITY_EDITOR`. | Use the [Unity guide](/adapters/unity/) for native plugin placement and host setup. Unity 6.7+ is primary, the pinned Mono profile is best effort, and CoreCLR remains experimental. |

## Signing and release records

Checksums identify artifacts; they do not establish a trusted signature. A
successful package build is not evidence of macOS notarisation, Windows code
signing, a signed Flatpak repository or store acceptance. Record those outcomes
for the actual release candidates instead of inferring them from CI definitions.
No new signing keys, accounts or publication infrastructure are required by the
local checks above.

Before release, [#79](https://github.com/plethu/recite/issues/79) owns final
packaging/install evidence and [#81](https://github.com/plethu/recite/issues/81)
owns known limits and support records. Include the artifact hash, platform and
architecture, install/upgrade result, signing status and unresolved limitations.
The [engine acceptance matrix](https://github.com/plethu/recite/blob/main/docs/adapter-acceptance-matrix.md)
and Writer's platform records remain the detailed evidence owners.

For a reproducible bug report, include the source revision, package hash,
engine/runtime version when applicable, OS/architecture, exact command and a
small source/schema fixture. Remove private project content before sharing it.
The [GitHub issue tracker](https://github.com/plethu/recite/issues) tracks follow-up
work; no response-time or platform-support guarantee is implied by a package
being buildable.
