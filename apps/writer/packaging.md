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

The [Flatpak instructions](packaging/flatpak/README.md) cover source builds, offline Cargo
dependencies and bundle checks. The [Nix instructions](../../nix/README.md) cover `nix build`, `nix
run`, and the development shell. Both follow the source packaging approach used by Neovide for Rust
and Skia; Flatpak uses the standard Cargo source generator. The package workflow declares x86-64 and
ARM64 Linux builds for both formats, and both Mac architectures for Nix and native bundles.

Flatpak uses a shared runtime instead of depending on each distro's library versions. Its manifest
grants host filesystem access for projects and host-command access for explicitly configured schema
producers and source editors. Those commands retain separate arguments and the validated project
working directory; cancelling generation terminates the host-command bridge and its child. The app's
gettext and preferred-application launcher come from the runtime.

The [package build instructions](packaging/README.md) name the pinned tool, artifact checks and
current limits. `--help` and `--version` work without a display. The default launch opens project
controls; `--examples` opens temporary examples. A project link selects its project before resolving
its location.

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

The [September 2026 preview evidence](../../docs/archive/delivery-evidence.md#writer-packaging) is
historical; rerun the maintained checks for a new candidate.

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

The [route format, Copy link control and startup arguments](navigation.md) share one route
validator. Record desktop smoke results separately from headless route tests and package inspection,
and keep each platform's remaining requirements open until its installed-host evidence exists.
