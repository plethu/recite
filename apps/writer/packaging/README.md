# Writer package previews

This directory defines native package candidates for Recite Writer 0.0.0:
Linux `.deb`, macOS `.app` and `.dmg`, and Windows current-user NSIS. The
application ID is `io.github.plethu.recite`. Packages include the repository's
MIT/Apache dual-license notice and both license texts. The icon uses the shared [Recite identity](../../../assets/identity/README.md).
Its source is `icons/recite-writer.svg`; the native icon files are derived from
that source.

For Linux distribution across distros, start with the
[Flatpak source build](flatpak/README.md). The repository also provides
[Nix packages and a development shell](../../../nix/README.md). The native
formats below remain useful for platform-specific installation checks.

The workflow uses [cargo-packager 0.11.8](https://github.com/crabnebula-dev/cargo-packager/releases/tag/cargo-packager-v0.11.8)
and its [published configuration fields](https://docs.crabnebula.dev/packager/configuration/).
From the repository root, after installing the native build dependencies and
`cargo install cargo-packager --version 0.11.8 --locked`:

```sh
python3 tests/writer-packaging/check.py
python3 scripts/package-writer.py --check-config
python3 scripts/package-writer.py
python3 scripts/check-writer-package.py target/writer-packages/linux  # Linux host
```

The command builds the writer with its locked Cargo workspace, packages the
native format, and writes under `target/writer-packages/`. The checker inspects
metadata, payload, license files and installed binary CLI flags, then writes
`SHA256SUMS`. The [package preview workflow](../../../.github/workflows/writer-packages.yml)
runs native builds and uploads short-lived CI artifacts. macOS has separate
Apple Silicon (`macos-15`) and Intel (`macos-15-intel`) jobs; inspection checks
the packaged Mach-O architecture before launching it. The Windows runner also
installs, reinstalls the same version and uninstalls the preview under its
disposable account. It preserves any pre-existing `recite://` association.
Neither a workflow definition nor a checksum is evidence of a signed or accepted
release.

The Debian config declares the writer's direct runtime libraries. Packaging
sets the `libc6` minimum from the actual binary's highest required `GLIBC_*`
symbol. Artifact inspection compares that minimum and the `NEEDED` libraries
against the package control file. It also checks `runtime-abi.json`, which
records the binary hash and its `GLIBC`, `GLIBCXX` and `CXXABI` symbol
requirements. The native Linux CI runner is Ubuntu 24.04; a passing build there,
rather than a local package made on a newer distribution, will establish the preview's
older-system baseline. Other library package versions remain subject to native
install testing.

For a bounded local packager smoke, `--binary /path/to/recite-writer` stages an
already built native binary instead of building a release binary. Such an
artifact is only a packaging fixture; its bytes may be a debug build. The
normal CI invocation builds with Cargo's release profile. Smoke staging uses a
separate directory and never overwrites Cargo's cached release executable.

The Linux desktop entry accepts one URL (`%u`) and declares
`x-scheme-handler/recite`. With a display and `gio` available, run
`python3 scripts/check-writer-desktop-links.py /path/to/extracted-deb` to exercise
cold launch and running-window delivery in a private desktop session. The Linux
CI lane runs it under Xvfb. This does not test package-manager installation,
upgrade association or uninstall ownership; see the [local evidence](../packaging.md).
macOS omits URL scheme registration until the app handles native URL-open events.
Windows also omits scheme registration: cargo-packager 0.11.8's
[NSIS uninstall template](https://github.com/crabnebula-dev/cargo-packager/blob/cargo-packager-v0.11.8/crates/packager/src/package/nsis/installer.nsi#L561-L567)
does not provide a verified runtime ownership check, and the published NSIS
configuration has no small uninstall hook. The Windows installer is an app
preview, not a deep-link-capable install.

The workflow still needs a native run on each configured OS and architecture.
It does not assert GUI, screen-reader, IME/BiDi, physical device, signing,
notarisation or full desktop-link acceptance. Those results belong in the
platform acceptance record before distribution.
