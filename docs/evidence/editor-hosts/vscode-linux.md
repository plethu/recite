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
`ecfc491f311e975c2080f4fe0d0fef7abf36ac4c4444970fdf1b710a490e6bdd`.
The VSIX is a local test artifact; this evidence makes no Marketplace or Open
VSX publication claim.

## Automated observations

For each host, the check asserts:

- the VSIX is installed into a new temporary profile and discoverable by the
  following host process;
- opening `.recite` selects the `recite` language and activates the extension;
- the actual `recite-lsp` emits stable diagnostics (`RECITE_PARSE011` and
  `RECITE_PARSE013`) and the host receives a diagnostic-change event;
- the host's completion API returns structured completion items;
- `recite.validate` reports structured `success` for valid content and
  `content_diagnostics` plus `RECITE_PARSE011` for invalid content;
- `recite.compile` and `recite.extract` preserve the structured failure
  status, and `recite.watch.start` returns an invocation identifier;
- `recite.watch.stop` completes the supported cancellation path with exit code
  zero; the host test completes normally; and no `recite`, `recite-lsp`, or
  host-owned child process remains after the host exits (the host then owns its
  normal extension deactivation sequence).

The assertions live in
[`tests/editor-hosts/vscode/host-probe.cjs`](../../../tests/editor-hosts/vscode/host-probe.cjs).
The script prints each host version, architecture, archive digest, VSIX
digest, and each evidence category after a passing run. A failed assertion or
non-zero host exit fails the check, so this is repeatable automated host proof
rather than a package-only claim.

## Residual manual/platform evidence

The nested headless boundary proves extension and language-server behaviour
through host APIs. It does not prove keyboard-only reachability of the command
palette, Problems panel, status UI, focus order, screen-reader output,
high-contrast rendering, colour-independent diagnostics, or platform-specific
desktop integration. Those remain manual follow-up on supported desktop
platforms. In particular, no keyboard workflow is claimed by this check.

The test also does not cover non-Linux host builds, remote workspaces,
Marketplace/Open VSX distribution, or an installed user configuration. Those
are separate concerns from this ephemeral installed-host slice.
