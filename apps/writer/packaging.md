# Writer packaging and desktop integration

The writer has preview package definitions and repeatable artifact checks. Installation acceptance
remains open. Release smoke evidence is tracked in
[#79](https://github.com/plethu/recite/issues/79); platform acceptance also remains part of the GUI
milestone.

| Platform | Preview artifact                                                               | Desktop activation                                                              |
| -------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| Linux    | Flatpak for distribution across distros; `.deb` for the native Ubuntu baseline | `recite://` desktop entry; one project-writer window per user configuration     |
| macOS    | Separate Apple Silicon and Intel `.app` and `.dmg`, native CI configured       | Deferred; the host needs an incoming URL-event bridge                           |
| Windows  | Current-user NSIS, native CI configured                                        | Deferred; protocol registration needs verified ownership-safe uninstall         |
| Nix      | Flake packages and apps for the CLI and Writer, on Linux and macOS             | Linux desktop entry; desktop activation depends on how the package is installed |

The [Flatpak instructions](#flatpak) cover source builds, offline Cargo dependencies and bundle
checks. The [Nix instructions](../../nix/README.md) cover `nix build`, `nix run`, and the
development shell. Both follow the source packaging approach used by Neovide for Rust and Skia;
Flatpak uses the standard Cargo source generator. The package workflow declares x86-64 and ARM64
Linux builds for both formats, and both Mac architectures for Nix and native bundles.

Flatpak uses a shared runtime instead of depending on each distro's library versions. Its manifest
grants host filesystem access for projects and host-command access for explicitly configured schema
producers and source editors. Those commands retain separate arguments and the validated project
working directory; cancelling generation terminates the host-command bridge and its child. The app's
gettext and preferred-application launcher come from the runtime.

The [package build instructions](#native-packages) name the pinned tool, artifact checks and current
limits. `--help` and `--version` work without a display. The default launch opens project controls;
`--examples` opens temporary examples. A project link selects its project before resolving its
location.

Linux has one project-writer window per user configuration. Subsequent launches forward their
project and route over a private local socket. Opening another project uses the same asynchronous
loader and unsaved-work guards as the GUI; the welcome screen can receive project links too.
Examples and the component specimen remain independent windows. An unconfirmed or refused request
does not start a competing writer. macOS and Windows still use the standalone launcher and existing
file-recovery locks; their package candidates deliberately omit URL registration.

Unsaved work blocks a project switch. With a clean workspace, the linked project can open before an
unavailable scene or catalogue is discovered; that error names the opened project and the failed
location. A timeout leaves the outcome unconfirmed: cancellation prevents a pending load from
replacing the workspace, but cannot undo an application step that has already begun.

The native CI matrix is configured, not evidence that those hosted runs passed. Local Linux
artifacts built on a newer distribution are host-specific; use the declared CI build baseline before
offering them to other distributions. Package extraction and CLI launch do not establish
package-manager upgrade/uninstall, signing, notarisation, native accessibility or usability
acceptance.

The
[September 2026 preview evidence](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#writer-packaging)
is historical; rerun the maintained checks for a new candidate.

## Native packages

The packaging directory defines native package candidates for Recite Writer 0.0.0: Linux `.deb`,
macOS `.app` and `.dmg`, and Windows current-user NSIS. The application ID is
`io.github.plethu.recite`. Packages include the repository's MIT/Apache dual-license notice and both
license texts. The icon uses the shared [Recite identity](../../assets/identity/README.md). Its
source is `packaging/icons/recite-writer.svg`; the native icon files are derived from that source.

For Linux distribution across distros, start with the [Flatpak source build](#flatpak). The
repository also provides [Nix packages and a development shell](../../nix/README.md). The native
formats below remain useful for platform-specific installation checks.

The workflow uses
[cargo-packager 0.11.8](https://github.com/crabnebula-dev/cargo-packager/releases/tag/cargo-packager-v0.11.8)
and its [published configuration fields](https://docs.crabnebula.dev/packager/configuration/). From
the repository root, after installing the native build dependencies:

```sh
mise -E packaging install cargo:cargo-packager
```

Run packaging commands with that tool environment:

```sh
mise -E packaging exec -- python tests/writer-packaging/check.py
mise -E packaging exec -- python scripts/package-writer.py --check-config
mise -E packaging exec -- python scripts/package-writer.py
mise -E packaging exec -- python scripts/check-writer-package.py target/writer-packages/linux  # Linux host
```

The command builds the writer with its locked Cargo workspace, packages the native format, and
writes under `target/writer-packages/`. The checker inspects metadata, payload, license files and
installed binary CLI flags, then writes `SHA256SUMS`. The
[package preview workflow](../../.github/workflows/writer-packages.yml) runs native builds and
uploads short-lived CI artifacts. macOS has separate Apple Silicon (`macos-15`) and Intel
(`macos-15-intel`) jobs; inspection checks the packaged Mach-O architecture before launching it. The
Windows runner also installs, reinstalls the same version and uninstalls the preview under its
disposable account. It preserves any pre-existing `recite://` association. Neither a workflow
definition nor a checksum is evidence of a signed or accepted release.

The Debian config declares the writer's direct runtime libraries. Packaging sets the `libc6` minimum
from the actual binary's highest required `GLIBC_*` symbol. Artifact inspection compares that
minimum and the `NEEDED` libraries against the package control file. It also checks
`runtime-abi.json`, which records the binary hash and its `GLIBC`, `GLIBCXX` and `CXXABI` symbol
requirements. The native Linux CI runner is Ubuntu 24.04; a passing build there, rather than a local
package made on a newer distribution, will establish the preview's older-system baseline. Other
library package versions remain subject to native install testing.

For a bounded local packager smoke, `--binary /path/to/recite-writer` stages an already built native
binary instead of building a release binary. Such an artifact is only a packaging fixture; its bytes
may be a debug build. The normal CI invocation builds with Cargo's release profile. Smoke staging
uses a separate directory and never overwrites Cargo's cached release executable.

The Linux desktop entry accepts one URL (`%u`) and declares `x-scheme-handler/recite`. With a
display and `gio` available, run `mise -E packaging exec -- python
scripts/check-writer-desktop-links.py /path/to/extracted-deb` to exercise cold launch and
running-window delivery in a private desktop session. The Linux CI lane runs it under Xvfb. This
does not test package-manager installation, upgrade association or uninstall ownership; see the
[local evidence](#required-deep-link-handling). macOS omits URL scheme registration until the app
handles native URL-open events. Windows also omits scheme registration: cargo-packager 0.11.8's
[NSIS uninstall template](https://github.com/crabnebula-dev/cargo-packager/blob/cargo-packager-v0.11.8/crates/packager/src/package/nsis/installer.nsi#L561-L567)
does not provide a verified runtime ownership check, and the published NSIS configuration has no
small uninstall hook. The Windows installer is an app preview, not a deep-link-capable install.

The workflow still needs a native run on each configured OS and architecture. It does not assert
GUI, screen-reader, IME/BiDi, physical device, signing, notarisation or full desktop-link
acceptance. Those results belong in the platform acceptance record before distribution.

## Flatpak

This manifest builds Recite Writer from the current source checkout with the GNOME 50 SDK and
exports `io.github.plethu.recite.flatpak`. It builds the locked Cargo workspace offline from
committed, generated sources and builds the `freya-skia-bindings` Skia revision from source. The
result is a preview, not a published or installation-accepted release.

Install the build prerequisites in an isolated Flatpak installation or a disposable CI runner:
`flatpak-builder`, `dbus-run-session`, `org.gnome.Platform//50`, `org.gnome.Sdk//50`, and
`org.freedesktop.Sdk.Extension.rust-stable//25.08`, plus
`org.freedesktop.Sdk.Extension.llvm22//25.08` for Skia's Clang and bindgen. The GNOME SDK supplies
`ninja`, Python, C/C++ libraries and headers; the runtime supplies `flatpak-spawn`, `msginit`, and
portal-backed `xdg-open`. The Rust extension must provide Rust 1.96 or newer (the September 2026
x86_64 branch has 1.98.1). On an existing workstation, do not install these into the normal user
profile solely for this preview. One isolated setup is:

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

The build command writes `build/`, `state/`, `repo/`, the `.flatpak` bundle, and `SHA256SUMS` with a
relative bundle filename under the output directory. It does not install the application, add a
remote, or alter desktop associations. The script runs artifact, metadata, license, desktop-link
declaration, and headless `--help`/`--version` checks against the GNOME runtime after export. The
host bridge smoke uses a private D-Bus session. The manifest uses a `dir` source to capture the
current checkout, including uncommitted Writer changes. For a reproducible source release, replace
that source with a pinned archive and keep the same locked Cargo sources.

Refresh `cargo-sources.json` when `apps/writer/Cargo.lock` changes using
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/41c20aa10819cdb2a4f3ca171758a96d1955c018/cargo):

```sh
python3 flatpak-cargo-generator.py apps/writer/Cargo.lock \
  -o apps/writer/packaging/flatpak/cargo-sources.json
```

GN is pinned to the source revision used by the
[Neovide Flatpak](https://github.com/flathub/dev.neovide.neovide/tree/a0cba7888dbd06eecfefc27e493f92ec8bb72585/modules/gn).
Skia is pinned to `rust-skia/skia` tag `m152-0.100.0`, the revision declared by `freya-skia-bindings
0.100.0`. `SKIA_SOURCE_DIR` prevents its build script from fetching a different source during
compilation. Native x86_64 and aarch64 builds are intended; each architecture needs its own SDK and
build runner.

The manifest grants host filesystem access because Writer projects may live on mounted drives, and
grants `org.freedesktop.Flatpak` D-Bus access so explicitly configured producers and editors can be
launched through `flatpak-spawn --host`. This is a broad permission and should be visible to
reviewers. Installed link activation, upgrade/uninstall association ownership, device input, native
accessibility, and both architecture builds still require separate acceptance.

## Required deep-link handling

For each supported desktop platform, the installed package must:

- Register `recite://` with the desktop: a Linux desktop entry and scheme association, macOS bundle
  URL types and URL-open event handling, or Windows protocol registration and activation handling.
- Open the project identified by a link before resolving its scene, passage, catalogue or
  translation-queue location. Report missing/moved projects and unavailable targets without silently
  selecting a different project.
- Deliver links to an already-running writer and focus the appropriate window. Running-window
  delivery must pass through the same route validation and draft guards as Back/Forward. Neither
  startup nor activation may discard source or translation edits or create competing writers for the
  same files.
- Preserve encoded paths, Unicode, spaces and queue queries when handing off the URL. Parse it as
  data; never interpolate it into a shell command. Reject malformed or unsupported routes without
  modifying project content.
- Keep registration usable after an upgrade and remove registrations owned by the package on
  uninstall, without removing another application's association.

The installation smoke matrix must exercise cold launch and running-window activation from an actual
desktop link, correct project/scene/catalogue selection, queue search/filter/page restoration,
unsaved-edit refusal, malformed links, upgrade and uninstall. Record results and limits separately
for each OS.

The [route format, Copy link control and startup arguments](guide.md#navigation-and-links) share one
route validator. Record desktop smoke results separately from headless route tests and package
inspection, and keep each platform's remaining requirements open until its installed-host evidence
exists.
