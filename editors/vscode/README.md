# Recite for VS Code and VSCodium

This package is the shared VSIX entry point for Recite's VS Code and VSCodium
clients. It registers `.recite` files and connects the editor to a local
`recite-lsp` process over standard input and output.

The language server and shared authoring kernel own parsing, validation,
diagnostics, completion, navigation, edits, and stable IDs. The extension only
adapts those LSP values to editor APIs. It does not parse Recite source, run a
game, or require a hosted service. Its small JSON-RPC transport uses Node's
standard library so the extension does not pull in a second client framework or
an embedded browser runtime.

## Local development

From the repository root:

```text
pnpm --filter recite-vscode run check
pnpm --filter recite-vscode run package:check
```

Build the extension and produce a deterministic VSIX with:

```text
pnpm --filter recite-vscode run package
```

The package uses the repository's pinned Node and pnpm versions and creates the
VSIX archive with Node's standard library; no system archive utility, Electron
runtime, or browser binary is required.

## Language-server configuration

The default server command is `recite-lsp`, resolved through `PATH`. Set
`recite.lsp.path` to an absolute path or to a project-root-relative executable
when the binary is not on `PATH`. `recite.lsp.args` passes an explicit argument
array without invoking a shell. `recite.lsp.projectRoot` optionally selects a
project root; otherwise the first workspace folder is used.

The server process is started when a `.recite` document activates the package,
receives full-document open/change/save notifications, and is shut down when
the extension deactivates or its configuration changes. The current server
does not implement cancellation, command/watch envelopes, or remote projects;
the extension does not claim those capabilities.

Relative paths and process spawning use Node's platform-neutral path and
process APIs. Linux, macOS, and Windows are intended hosts, but this scaffold
contains Linux-only executable evidence; platform packaging and publication
smoke remain release work. The same VSIX can be submitted to the VS Code
Marketplace or Open VSX when those distribution decisions are made.
