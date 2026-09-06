# Installed VS Code and VSCodium evidence on Linux

This evidence was collected on 2026-09-06 for Linux x86_64. Run:

```text
scripts/check-vscode-host.sh
```

The check downloads each pinned official host archive into `/tmp`, verifies
its SHA-256 digest, and removes the temporary extraction and profiles on exit.
It creates a deterministic local `recite-vscode-0.1.0.vsix`, installs it via
the host extension API, and runs a second host process through the real
extension-test API. Both phases run in a nested headless Wayland compositor
with `DISPLAY` and `WAYLAND_DISPLAY` removed. The harness never uses a live
desktop/session. Existing extracted official hosts can be supplied with
`RECITE_VSCODE_HOST_BIN` and `RECITE_VSCODIUM_HOST_BIN`.

## Pinned host artifacts

| Host | Product version | Runtime API version | Commit | Official archive | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| VS Code | `1.136.1` | `1.136.1` | `a44adf7f53e00964ab890f9f8758a334f1fc15bc` | Microsoft stable Linux x64 | `9b4a54f0d49beaa413eda137d00c6541a639300d479efcac566ad13419409218` |
| VSCodium | `1.126.04524` | `1.126.0` | `4c0b0c6cc561d2d3636d1ec250935431876ce4dc` | VSCodium GitHub release Linux x64 | `adf3548df055d18e476cdee887488ba7486b879ad99a31a546c6b5c5ff296c24` |

The deterministic VSIX produced by the check currently hashes to
`5a5f411406cf3e706d420499a6036923941616c356b7a6d212f2abc8e54db20b`.
The VSIX is a local test artifact; this evidence makes no Marketplace or Open
VSX publication claim.

## Automated observations

For each host, the check asserts:

- the VSIX is installed into a new temporary profile and discoverable by the
  following host process;
- opening `.recite` selects the `recite` language and activates the extension;
- the actual `recite-lsp` emits stable diagnostics (`RECITE_PARSE011` and
  `RECITE_PARSE013`) and the host receives a diagnostic-change event;
- the host's completion and hover APIs return structured results, and
  definition/navigation resolves both same-file and canonical sibling targets;
- references return the two canonical locations in the pinned host's
  deterministic order (the VS Code host sorts them, so the server's source
  order is not observable through this native API); the explicit Recite rename
  command exercises its guarded missing-active-document precondition. The
  installed extension-test API cannot answer its input prompt without opening
  a real UI, so the versioned edit/apply path remains covered by controller
  tests; native F2 rename is explicitly unsupported by this client;
- the UTF-16/non-BMP/CRLF diagnostic range survives host projection, malformed
  and incomplete overlays recover to a valid document, and the stable-ID
  quick-fix is applied through the controller-owned command;
- `recite.validate` reports structured `success` for valid content and
  `content_diagnostics` plus `RECITE_PARSE011` for invalid content;
- `recite.compile` and `recite.extract` preserve the structured failure
  status; `recite.run` and `recite.trace` return matching structured runtime
  traces; and `recite.watch.start` returns an invocation identifier;
- `recite.watch.stop` completes the supported cancellation path with exit code
  zero; the host test completes normally; and no `recite`, `recite-lsp`, or
  host-owned Cage/host descendant remains in the captured process group after
  the host exits (the host then owns its normal extension deactivation
  sequence). Any descendant observed on a phase exit fails that lane, even if
  bounded cleanup recovers it.
- a second isolated host run drives actual Wayland keyboard events through
  private Cage: `Ctrl+P`, `scratch/invalid.recite`, and `Return` open the
  workspace-relative source. Before diagnostics navigation continues, the
  probe asserts the real host's active editor URI, `recite` language, and
  Recite extension activation. Then `Ctrl+Shift+M` opens Problems, `Escape`
  returns focus, `Ctrl+1` focuses the primary editor, and `F8` activates the
  next diagnostic. The probe asserts that the selection is within the
  `RECITE_PARSE011` range; a disposable probe
  keybinding records the live code, error severity, and source location without
  relying on colour. Real key events invoke the supported explicit rename command and `recite.watch.start`;
  a disposable `Ctrl+Alt+Shift+Q` binding invokes the supported
  `recite.watch.stop` command because the product intentionally has no default
  shortcut. The
  resulting document edit and exact captured-group CLI shutdown are observed
  through marker files and process inspection. The disposable bindings belong
  only to this probe profile and are not product compatibility claims.
- the keyboard runs use distinct one-character `v` and `c` profile roots to
  stay within Linux's Unix-socket path limit. Each install phase must start
  without its marker, and the host writes a host-product and lane-specific
  activation marker before the keyboard phase continues; this prevents
  sequential profile/session state from being mistaken for independent host
  evidence.

The assertions live in
[`tests/editor-hosts/vscode/host-probe.cjs`](../../../tests/editor-hosts/vscode/host-probe.cjs).
The script prints each host version, architecture, archive digest, VSIX
digest, and each evidence category after a passing run. A failed assertion or
non-zero host exit fails the check, so this is repeatable automated host proof
rather than a package-only claim.

## Residual manual/platform evidence

The nested headless boundary proves the scripted keyboard path above, including
host command dispatch and inspectable semantic results. It does not prove
arbitrary keyboard-only focus traversal, every command-palette or Problems
panel layout, screen-reader output, high-contrast rendering, or platform-
specific desktop integration. Those remain manual follow-up on supported
desktop platforms. The marker is a test boundary, not a visual or accessibility
oracle.

The test also does not cover non-Linux host builds, remote workspaces,
Marketplace/Open VSX distribution, or an installed user configuration. Those
are separate concerns from this ephemeral installed-host slice.
