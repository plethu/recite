# Recite for VS Code and VSCodium

This package is the shared VSIX entry point for Recite's VS Code and VSCodium
clients. It registers `.recite` files and connects the editor to a local
`recite-lsp` process over standard input and output.

The package also contributes `syntaxes/recite.tmLanguage.json`, a tolerant
TextMate grammar shared by VS Code and VSCodium. It provides lexical scopes for
Recite markers, names, anchors, references, metadata, values, calls, prose,
markup, and placeholders. It never validates IDs, references, schemas,
conditions, effects, markup balance, or match exhaustiveness; those remain
parser/compiler/LSP responsibilities.

The grammar supplies scopes only; the active VS Code or VSCodium theme controls
their colour, font, and contrast. The line and choice anchor scopes are separate
so a theme may de-emphasise them, but the grammar cannot require that visual
treatment. Scope appearance is never the sole semantic signal: marker, label,
and anchor text remains present, and non-colour/high-contrast themes remain
valid. The pinned Node tokenizer snapshots provide evidence for scope identity,
not installed-host rendering or accessibility behaviour.

The package's CommonJS entry shim obtains the VS Code host API through
`require`, keeping the VS Code 1.89 extension host boundary loadable, then
delegates activation to the ESM implementation.

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
pnpm --filter recite-vscode run messages:update  # after changing the canonical Fluent resources
```

Build and verification never rewrite the checked-in message projections. Use
the explicit update command when the canonical English Fluent resources
change, then run the checks to review the resulting bytes.

The VS Code and VSCodium projections, including their argument lists and
output paths, are declared in `crates/recite-ui/resources/inventory.toml`.
The extension reports transport, protocol, and lifecycle failures through the
same localised catalog. Server `showMessage` notifications use the matching
editor severity, while server logs remain in the output channel and genuine
process details are retained alongside their localised failure category.
Transient restart attempts stay in the output channel; exhausted recovery is
shown as an error notification.

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

The server process is started when a trusted `.recite` document activates the
package, receives full-document open/change/save/close notifications, and is
shut down when the extension deactivates or its configuration changes. The
client also honours the LSP server's `client/registerCapability` request for
project file watching and forwards deterministic create/change/delete events.
The command palette also adapts the local structured CLI protocol for
validation, compilation, extraction, fixture runs, traces, and a one-process
watch loop. Commands use the saved active `.recite` document where applicable;
they never save or execute an untitled/dirty document. Output is consumed as
version-1 NDJSON, while diagnostics are kept in a command-owned collection.
The watch stop command sends the versioned stdin cancellation record and waits
for the matching stopped record before using bounded process recovery.

Set `recite.cli.path` to an absolute or project-root-relative `recite` binary;
a bare executable name is resolved through `PATH`. It is restricted in
untrusted workspaces, and commands never invoke a shell or a hosted service.

Code-action edits are returned as extension-owned commands. The command keeps
the LSP document versions, including zero-edit sibling preconditions, and
checks them again immediately before applying the edit. Native rename is not
registered yet: VS Code's native `WorkspaceEdit` path cannot preserve those
LSP versions at its eventual apply boundary. A version-safe rename adapter is
remaining closure work for REC-51.

Relative paths and process spawning use Node's platform-neutral path and
process APIs. Linux, macOS, and Windows are intended hosts, but this scaffold
contains Linux-only executable evidence; platform packaging and publication
smoke remain release work. A real installed VS Code and VSCodium activation
smoke is also remaining REC-51 closure evidence; this foundation intentionally
does not add the unpinned `@vscode/test-electron` dependency or its browser
download. The same VSIX can be submitted to the VS Code
Marketplace or Open VSX when those distribution decisions are made.
