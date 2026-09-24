# Writer Flatpak preview

This manifest builds Recite Writer from the current source checkout with the
GNOME 50 SDK and exports `io.github.plethu.recite.flatpak`. It builds the locked
Cargo workspace offline from committed, generated sources and builds the
`freya-skia-bindings` Skia revision from source. The result is a preview, not a
published or installation-accepted release.

Install the build prerequisites in an isolated Flatpak installation or a
disposable CI runner: `flatpak-builder`, `dbus-run-session`, `org.gnome.Platform//50`,
`org.gnome.Sdk//50`, and
`org.freedesktop.Sdk.Extension.rust-stable//25.08`, plus
`org.freedesktop.Sdk.Extension.llvm22//25.08` for Skia's Clang and bindgen.
The GNOME SDK supplies `ninja`, Python, C/C++ libraries and headers; the runtime supplies
`flatpak-spawn`, `msginit`, and portal-backed `xdg-open`. The Rust extension
must provide Rust 1.96 or newer (the September 2026 x86_64 branch has 1.98.1).
On an existing workstation, do not install these into the normal user profile
solely for this preview. One isolated setup is:

```sh
export XDG_DATA_HOME="$PWD/target/writer-flatpak-profile"
flatpak remote-add --user --if-not-exists flathub \
  https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 \
  org.freedesktop.Sdk.Extension.llvm22//25.08
```

From the repository root:

```sh
python3 scripts/check-writer-flatpak.py  # static metadata and Cargo lock, no SDK needed
python3 scripts/check-writer-flatpak.py --builder
scripts/package-writer-flatpak.sh /path/to/build-output
```

The build command writes `build/`, `state/`, `repo/`, the `.flatpak` bundle,
and `SHA256SUMS` with a relative bundle filename
under the output directory. It does not install the application, add a remote,
or alter desktop associations. The script runs artifact, metadata, license,
desktop-link declaration, and headless `--help`/`--version` checks against the
GNOME runtime after export. The host bridge smoke uses a private D-Bus session.
The manifest uses a `dir` source to capture the current checkout, including
uncommitted Writer changes. For a reproducible source release, replace that
source with a pinned archive and keep the same locked Cargo sources.

Refresh `cargo-sources.json` when `apps/writer/Cargo.lock` changes using
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/41c20aa10819cdb2a4f3ca171758a96d1955c018/cargo):

```sh
python3 flatpak-cargo-generator.py apps/writer/Cargo.lock \
  -o apps/writer/packaging/flatpak/cargo-sources.json
```

GN is pinned to the source revision used by the
[Neovide Flatpak](https://github.com/flathub/dev.neovide.neovide/tree/a0cba7888dbd06eecfefc27e493f92ec8bb72585/modules/gn).
Skia is pinned to `rust-skia/skia` tag `m152-0.100.0`, the revision declared by
`freya-skia-bindings 0.100.0`. `SKIA_SOURCE_DIR` prevents its build script from
fetching a different source during compilation. Native x86_64 and aarch64
builds are intended; each architecture needs its own SDK and build runner.

The manifest grants host filesystem access because Writer projects may live on
mounted drives, and grants `org.freedesktop.Flatpak` D-Bus access so explicitly
configured producers and editors can be launched through `flatpak-spawn --host`.
This is a broad permission and should be visible to reviewers. Installed link
activation, upgrade/uninstall association ownership, device input, native
accessibility, and both architecture builds still require separate acceptance.
