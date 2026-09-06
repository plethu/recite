# Zed Linux installed-host evidence

This is an installed-host probe record for Recite Milestone 4. It does not
change the parity contract: the checked-in Zed client remains partial until
the host evidence owners close the remaining capabilities.

## Reproduction

Run this from a Linux session with an installed Zed binary, a working GPU
driver, and the private compositor tools:

```sh
RECITE_ZED_TIMEOUT=90 tests/editor-hosts/zed/check.sh
```

The probe builds `recite-lsp` and `recite` into a temporary Cargo target unless
`RECITE_LSP_BIN` and `RECITE_CLI_BIN` are supplied. It runs the direct
`zed-editor` binary under Cage's headless WLR backend with a private
`XDG_RUNTIME_DIR`, user-data directory, DBus session, and Wayland socket. It
unsets the caller's `DISPLAY` and `WAYLAND_DISPLAY`; it never uses the live
desktop. The selected extension directory is copied under `/tmp`, since the
Zed development installer writes `extension.wasm` and generated grammar files
beside the selected source directory. Every probe artifact is removed on
success or failure unless `RECITE_ZED_KEEP=1` is set for diagnosis.

The DBus service shadow in the probe maps credential and portal services to
`/bin/false`. This prevents a keyring or portal dialog taking the private
keyboard focus; it is a harness adaptation, not a Recite service and not a
production-host claim.

## Observed host

Observed 2026-09-06 on Linux x86_64:

| Item | Evidence |
| --- | --- |
| Installed package | Arch `zed` `1.18.1-1`, repository `extra` |
| Direct host | `/usr/lib/zed/zed-editor` |
| Direct host SHA-256 | `6329d6a67e3114d834c3d50b08babf8328de68001250c37cf43a890daeecd8fe` |
| Version query | `zeditor --version` → `Zed 1.18.1 – /usr/lib/zed/zed-editor` |
| Architecture/kernel | `x86_64`, Linux `7.2.3-1-cachyos` |
| Recite LSP SHA-256 | `de5b70e31b2dce4489a9637d489a0be8448c789a8af311e2280696385f037954` |
| Recite CLI SHA-256 | `4720ca9cc17e67eb05f3ef5843872afb96dbb831ee09e6c56bc9096583e44819` |
| Host compositor | Cage `0.3.1-b7b774a`, WLR `headless`, one private `wayland-0` socket |
| Automation tools | `wtype` 0.4-2.4, `grim` 1.5.0-2.1, `dbus-run-session` 1.16.2 |
| Render path | Zed log recorded `Rendered first frame` and an AMD Radeon 860M Vulkan adapter |

The direct host binary itself does not accept `--version`; the version string
above comes from the installed `zeditor` client while the executable hash is
from the direct binary actually launched by Cage. Zed's own log identified the
running build as `1.18.1+stable, sha unknown`.

## Results

The final private run used the installed host and the prebuilt binaries from
the checked-out worktree:

```sh
ZED_EDITOR=/usr/lib/zed/zed-editor \
RECITE_LSP_BIN=/tmp/recite-zed-final-target/debug/recite-lsp \
RECITE_CLI_BIN=/tmp/recite-zed-final-target/debug/recite \
RECITE_ZED_TIMEOUT=45 \
scripts/check-zed-host.sh /tmp/recite-m4-zed
```

It passed the following assertions:

```text
extension_wasm_sha256=eff6f486881a0e53b77d29c98fa3a5098113af1d55fd58389850457ebb4b2f9f
installed_extension_index=recite(dev=true),grammar_rev=209ea23195f674a18be0b8f87e037273fb3296bd
recite_lsp_process=observed
lsp_transport=actual_zed_requests_and_recite_responses_asserted
lsp_diagnostics=RECITE_PARSE011/013 severity=1 UTF-16 ranges asserted
lsp_features=completion/hover/definition/references/prepareRename/codeAction asserted
lsp_rename_edit=unsupported_in_this_key_sequence(rename requires host text-entry confirmation)
diagnostic_navigation=next_and_previous_keyboard_actions_observed
task_validate=structured argv observed, status=1 observed
task_watch=structured argv observed, Ctrl-C termination observed
shutdown=Ctrl-Q+zed:quit+Alt-F4 requested; no private probe process remained
PASS: installed Zed Linux source extension, activation/rendering, LSP process, diagnostic fixture, LSP UI actions, static task failure, watch keyboard termination, and private shutdown exercised
```

The checked-in `tests/editor-hosts/zed/lsp_proxy.py` is copied into the private
run directory. Zed launches that unique probe-local path, which launches the
copied `recite-lsp` binary; the proxy records and forwards each original
`Content-Length` frame without synthesizing messages. `assert_lsp_log.py` then
checks messages from this Zed process: initialize advertised UTF-16,
synchronization, completion, hover, definition, references, prepare-rename,
and code-action capabilities; the canonical malformed fixture produced
`RECITE_PARSE011` and `RECITE_PARSE013` at their exact severity-1 UTF-16
ranges; and Zed-triggered completion, hover, definition, references,
prepare-rename, and code-action responses contained the canonical Recite
results.

The non-empty Wayland screenshots record extension installation, authoring,
each action stage, diagnostic navigation in both directions, and task stages.
Keyboard events crossed the private compositor through `wtype`: command
palette, development-extension path selection, file picker, task picker,
task-terminal Ctrl-C, F8/Shift-F8 diagnostic navigation, and shutdown. The
temporary `recite` wrapper recorded exact task argv and status: the malformed
canonical fixture returned status 1, and the watch task was stopped by
Ctrl-C. The final process check tracks the private Cage/DBus process tree and
probe path, and found no remaining private Zed, Cage, LSP, CLI, proxy, or task
process.

## Boundaries and residuals

- The extension was installed as a local development extension, not from the
  Zed gallery. Gallery publication, signing, and gallery-install behavior are
  not claimed.
- The LSP transport assertions prove requests and responses crossed this
  installed Zed process, but they are not a replacement for the shared Recite
  LSP stdio/editor-parity fixtures. This host run does not add a non-BMP
  fixture or independently prove stale-document/version rejection; those
  remain covered by the canonical lower-level tests.
- The rename action reached `textDocument/prepareRename` and its canonical
  response. This keyboard sequence did not enter a replacement name and did
  not capture a `textDocument/rename` edit, so rename edit application remains
  unsupported in this host probe.
- Zed's task terminal displays the CLI's structured records but does not
  expose them through an editor-diagnostic API. The probe therefore asserts
  exact task argv/status and process termination, not parsing of rendered task
  records. Zed exposes no stable host API for a native watch-cancellation
  controller; Ctrl-C is the genuine terminal keyboard boundary.
- Any process still carrying this probe's private path or descended from its
  private Cage root causes the lane to fail. Cleanup uses bounded TERM/KILL
  recovery only to avoid leaking processes and reports recovery as evidence
  failure.
- No macOS/Windows host, screen-reader, high-contrast, gallery, live desktop,
  or network service behavior is claimed.

## Official host references

- [Developing extensions](https://zed.dev/docs/extensions/developing-extensions)
  documents `zed: install dev extension`, Rust/Wasm extension builds, and
  grammar build prerequisites.
- [Installing extensions](https://zed.dev/docs/extensions/installing-extensions)
  distinguishes development extensions from normal installed extensions.
- [Linux development](https://zed.dev/docs/development/linux) documents Zed's
  Wayland/X11 host modes and Linux requirements.
- [Tasks](https://zed.dev/docs/tasks) documents `.zed/tasks.json`, task
  spawning, `$ZED_FILE`, `$ZED_WORKTREE_ROOT`, and terminal task behavior.
- [Configuring languages](https://zed.dev/docs/configuring-languages) documents
  Zed's completion, hover, navigation, rename, code-action, and diagnostic
  editor commands used by the probe.
- [Diagnostics](https://zed.dev/docs/diagnostics) documents language-server
  diagnostics and the diagnostic deployment command.
- [Finding and navigating](https://zed.dev/docs/finding-navigating) documents
  the Ctrl-P file picker used to move between canonical fixtures.
- [All actions](https://zed.dev/docs/all-actions) documents the diagnostic
  actions; the [Linux default keymap](https://github.com/zed-industries/zed/blob/main/assets/keymaps/default-linux.json)
  binds F8 and Shift-F8 to next/previous diagnostic navigation.
- [Worktree trust](https://zed.dev/docs/worktree-trust) documents restricted
  worktrees and the `session.trust_all_worktrees` setting used only in this
  ephemeral probe profile.
