# Zed Linux installed-host evidence

This is an installed-host probe record for Recite Milestone 4. It does not
change the parity contract: the checked-in Zed client remains partial until
the host evidence owners close the remaining capabilities.

## Reproduction

Run this from a Linux session with an installed Zed binary, a working GPU
driver, and the private compositor tools:

```sh
TMPDIR=/path/to/private-temp RECITE_ZED_TIMEOUT=90 \
  tests/editor-hosts/zed/check.sh
```

The probe builds `recite-lsp` and `recite` into a temporary Cargo target unless
`RECITE_LSP_BIN` and `RECITE_CLI_BIN` are supplied. Keep `TMPDIR` on a
writable, private filesystem with enough space for the Cargo target and probe
artifacts. It runs the direct `zed-editor` binary under Cage's headless WLR
backend with a private
`XDG_RUNTIME_DIR`, user-data directory, DBus session, and Wayland socket. It
unsets the caller's `DISPLAY` and `WAYLAND_DISPLAY`; it never uses the live
desktop. The selected extension directory is copied under `TMPDIR`, since the
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
| Recite LSP SHA-256 | `87874eb92054a854d36d3ca0e32fe2c30f57dc593537e8e03d71d05d18949cdf` |
| Recite CLI SHA-256 | `6d8b6e1cbff9c198966d9b759d9e81a6abf79dba6fd2f7f1b0e8b2ce24a308b1` |
| Host compositor | Cage `0.3.1-b7b774a`, WLR `headless`, one private `wayland-0` socket |
| Automation tools | `wtype` 0.4-2.4, `grim` 1.5.0-2.1, `dbus-run-session` 1.16.2 |
| Render path | Zed log recorded `Rendered first frame` and an AMD Radeon 860M Vulkan adapter |

The direct host binary itself does not accept `--version`; the version string
above comes from the installed `zeditor` client while the executable hash is
from the direct binary actually launched by Cage. Zed's own log identified the
running build as `1.18.1+stable, sha unknown`.

## Results

The 2026-09-06 private run used the installed host and prebuilt binaries from
the checked-out worktree. Its private probe state was kept outside `/tmp`;
substitute the paths to the prebuilt binaries and private temporary directory
for a reproduction:

```sh
ZED_EDITOR=/usr/lib/zed/zed-editor \
RECITE_LSP_BIN=/path/to/recite-lsp \
RECITE_CLI_BIN=/path/to/recite \
TMPDIR=/path/to/private-temp RECITE_ZED_TIMEOUT=90 \
scripts/check-zed-host.sh /path/to/recite
```

The installed host rendered and activated the development extension, started
the real `recite-lsp`, exercised the keyboard LSP/task workflow, and left no
private probe process after shutdown. The transport and task assertions
recorded:

```text
extension_wasm_sha256=eff6f486881a0e53b77d29c98fa3a5098113af1d55fd58389850457ebb4b2f9f
installed_extension_index=recite(dev=true),grammar_rev=209ea23195f674a18be0b8f87e037273fb3296bd
recite_lsp_process=observed
lsp_transport=actual_zed_requests_and_recite_responses_asserted
lsp_diagnostics=RECITE_PARSE011/013 severity=1 non-BMP UTF-16 ranges asserted
lsp_utf16=non_BMP_fixture_didOpen_and_post_marker_request_asserted
lsp_features=completion/hover/definition/references/prepareRename asserted
lsp_code_action=non_empty_canonical_quick_fix_response_and_edit_asserted
lsp_rename_edit=work_renamed_two_occurrence_workspace_edit_asserted
lsp_rename_edit=non_empty_workspace_edit_applied_and_saved
lsp_code_action=non_empty_quick_fix_applied_and_saved
lsp_ui_actions=diagnostics,completion,hover,definition,references,rename,code-actions dispatched
task_extract=pid=2757833, exact argv/cwd/status asserted
task_compile=pid=2758074, exact argv/cwd/status asserted
diagnostic_navigation=next_and_previous_keyboard_actions_observed
task_validate=pid=2758674, exact argv/cwd/status asserted
task_watch=structured argv observed, Ctrl-C termination observed
shutdown=Ctrl-Q+zed:quit+Alt-F4 requested; no private probe process remained
PASS: installed Zed Linux source extension, activation/rendering, LSP process, diagnostic fixture, LSP UI actions, applied code action, applied rename, static task invocation/status, watch keyboard termination, and private shutdown exercised
```

The checked-in `tests/editor-hosts/zed/lsp_proxy.py` is copied into the private
run directory. Zed launches that unique probe-local path, which launches the
copied `recite-lsp` binary; the proxy records and forwards each original
`Content-Length` frame without synthesizing messages. `assert_lsp_log.py` then
checks messages from this Zed process: initialize advertised UTF-16,
synchronization, completion, hover, definition, references, prepare-rename,
and code-action capabilities; the canonical malformed fixture produced
`RECITE_PARSE011` and `RECITE_PARSE013` at their exact severity-1 UTF-16
ranges, including a non-BMP marker carried in the real `didOpen` text. Zed
also sent a real completion request at line 2, UTF-16 character 14, after the
marker and separator; this proves the installed client emitted the expected
post-marker wire position. It does not claim any additional rendering or
client-side range conversion beyond that request. The other Zed-triggered
completion, hover, definition, references, and prepare-rename
responses contained the canonical Recite results. Zed also sent a real
missing-ID code-action request for `code-action.recite`, with the `RECITE_ID001`
diagnostic and selected marker range; Recite returned the canonical
`Insert missing stable ID` quick-fix with the deterministic text
` line@56d52d8cd8619971011f` in a versioned document edit. Zed applied that
workspace edit and the probe compared the resulting line to that exact
returned `newText`. The exact request and response remain in the retained
proxy log.

The replacement-name keyboard flow entered `work_renamed` and sent a real
`textDocument/rename` request. Recite returned the exact two-occurrence
workspace edit for `core.recite`; Zed applied it and the probe compared the
complete before/after file, rejecting any change beyond those exact two
returned replacements. Extract and compile were then spawned independently
against the valid fixture; their task wrapper correlated each exact argv/cwd
start record with its own PID-matched exit status, including the compile
output path `project/core.recitec`.

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
  LSP stdio/editor-parity fixtures. The host copy of the malformed fixture
  includes a non-BMP marker; its retained `didOpen` text plus a real
  completion request at line 2, UTF-16 character 14 after that marker prove
  the installed Zed client emitted the expected UTF-16 request position. This
  does not claim rendering or client-side response-range conversion. A
  conforming Zed client increments full-sync document versions for each
  change; this keyboard probe does not fabricate a stale `didChange`, so
  stale-version rejection remains a lower-level ownership/test concern rather
  than a claimed host result.
- The malformed diagnostic fixture is intentionally outside project
  discovery. Opening it as a project sibling previously caused the
  project-wide stable-ID planner to return `Incomplete`, which correctly
  produced `result: []`; the probe now excludes `host-fixtures/**` while
  retaining the fixture for diagnostics. This is a harness isolation fix, not
  a semantic or host limitation.
- The task terminal does not expose structured records through an
  editor-diagnostic API. The probe therefore asserts exact task argv/cwd/status
  for validate, extract, and compile, plus process termination for watch; it
  does not parse rendered task records. Zed exposes no stable host API for a
  native watch-cancellation controller; Ctrl-C is the genuine terminal
  keyboard boundary.
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
