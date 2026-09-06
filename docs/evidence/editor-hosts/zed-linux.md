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

The final run used:

```sh
ZED_EDITOR=/usr/lib/zed/zed-editor \
RECITE_LSP_BIN=/tmp/recite-zed-host-target.zPfWt2/debug/recite-lsp \
RECITE_CLI_BIN=/tmp/recite-zed-host-target.zPfWt2/debug/recite \
RECITE_ZED_TIMEOUT=45 \
scripts/check-zed-host.sh /tmp/recite-m4-zed
```

It reported:

```text
extension_wasm_sha256=eff6f486881a0e53b77d29c98fa3a5098113af1d55fd58389850457ebb4b2f9f
installed_extension_index=recite(dev=true),grammar_rev=209ea23195f674a18be0b8f87e037273fb3296bd
recite_lsp_process=observed
task_validate=structured argv observed, status=1 observed
task_watch=structured argv observed, Ctrl-C termination observed
lsp_ui_actions=diagnostics,completion,hover,definition,references,rename,code-actions dispatched
shutdown=Ctrl-Q requested; no private probe process remained
PASS: installed Zed Linux source extension, activation/rendering, LSP process, diagnostic fixture, LSP UI actions, static task failure, watch keyboard termination, and private shutdown exercised
```

The install path and each authoring/task stage also produced a non-empty
Wayland screenshot. The keyboard events were sent through `wtype` to the
private compositor: command palette, development-extension path selection,
file open, task picker, task selection, task-terminal Ctrl-C, and the final
Ctrl-Q request. The temporary `recite` PATH wrapper recorded the exact
structured task argv and the malformed canonical fixture returned status 1.
The watch task was stopped through the task terminal; the exact CLI watch
process was absent afterward. Ctrl-Q did not close this Linux host within the
bounded wait, so the harness terminated only processes whose command lines
contained its unique temporary probe path. No Zed, Cage, LSP, or CLI process
with that path remained.

The host action sequence dispatched `diagnostics: deploy`, `editor: show
completions`, `editor: hover`, `editor: go to definition`, `editor: find all
references`, `editor: rename`, and `editor: toggle code actions`. The
non-empty screenshots for the latter six were identical in this headless run,
so that sequence demonstrates keyboard reachability only; it does not claim
that Zed rendered a successful completion, hover, navigation, rename, or code
action result.

For the final run, the screenshot digests were
`diagnostics-panel=9a23166d380a5b276d1e763c5e89d1de87e91b0d4197f3d6406c3bcce17b505e`,
`lsp-completion=lsp-hover=lsp-definition=lsp-references=lsp-rename=lsp-code-actions=b1b7348c0a7cc46966f4d021a502afa53719a0dbb86e469ef59b7ae29fc54ea9`,
and the install/authoring/task screenshots were non-empty as reported by the
probe. These are ephemeral screenshots; the probe does not check them in.

The LSP server was made available through the same temporary task PATH and
Zed logged the extension's normal fallback launcher. The malformed canonical
fixture was opened in the installed extension and the LSP process remained
alive. This proves host launch/attachment reachability and the diagnostic
fixture path; Zed 1.18.1 provides no stable machine-readable API for reading
the rendered diagnostic panel or task terminal, so screenshot evidence is not
promoted to a structured diagnostic assertion.

## Boundaries and residuals

- The extension was installed as a local development extension, not from the
  Zed gallery. Gallery publication, signing, and gallery-install behavior are
  not claimed.
- The host's LSP UI actions were dispatched through a real `recite-lsp`
  process, but the headless run did not produce distinct result captures for
  completion, hover, definition, references, rename, or code actions. This
  probe therefore does not claim those result payloads. The shared Recite LSP
  stdio/editor-parity tests remain the authority for structured responses; Zed
  exposes them only through rendered UI in this lane.
- Zed's task terminal displays the CLI's structured records but does not
  expose them through an editor-diagnostic API. The probe therefore asserts
  exact task argv/status and process termination, not task-record parsing.
- The task surface has no native machine-readable watch-cancellation
  controller in this host. Ctrl-C is the genuine terminal keyboard boundary;
  exact private-process cleanup is the bounded fallback.
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
- [Worktree trust](https://zed.dev/docs/worktree-trust) documents restricted
  worktrees and the `session.trust_all_worktrees` setting used only in this
  ephemeral probe profile.
