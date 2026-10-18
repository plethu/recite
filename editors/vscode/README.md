# Recite for VS Code and VSCodium

This package is the shared VSIX entry point for Recite's VS Code and VSCodium clients. It registers
`.recite` files and connects the editor to a local `recite-lsp` process over standard input and
output.

TextMate provides tolerant syntax highlighting; the active theme controls appearance. Parser,
compiler and LSP remain the authority for validation and source edits. Highlight snapshots establish
scope identity, not rendering or accessibility. The packaged `vscode-languageclient` owns LSP
transport and ships with production dependencies for offline use.

## Local development

From the repository root:

```text
bun run --filter recite-vscode check
bun run --filter recite-vscode package:check
bun run --filter recite-vscode messages:update  # after changing the canonical Fluent resources
```

The extension implementation is strict TypeScript. tsdown erases its types into JavaScript for the
editor's Node extension host; the CommonJS shim acquires `vscode` and injects it into the ESM
implementation. Node or Bun used for development cannot replace the installed editor's runtime.
Vitest runs the adapter tests, including a fresh Node process that loads the built entry point.
Generated projections, packaging scripts and host probes retain their native JavaScript formats.

CLI/watch/trace and guarded LSP inputs are checked against Valibot schemas before use. Types are
inferred from those schemas; watch sequencing and document authority remain in their owners. The
wire schemas reject unknown fields and preserve lossless integers, without coercion or defaults.
Valibot is bundled from a development dependency, with its MIT notice in `dist/valibot.LICENSE`.

The 14 October 2026 selection compared Valibot 1.5.0, Zod Mini 4.6.5 and ArkType 2.2.8 on strict
watch records with 0, 10 and 100 diagnostics, malformed fields, unknown keys, integer bounds and
cross-field constraints. Candidate bundles were 10.5, 25.5 and 157.6 KB respectively; these sizes
cover the representative subset, excluding shared generated vocabularies. ArkType was faster for
large records but imported much more slowly and introduced three transitive packages. Valibot and
Zod had no runtime dependencies. Valibot met this adapter's small startup/bundle needs without
paying for transforms, defaults or rich validation reports we do not consume.

The complete schema bundle, including finite commands and guarded LSP inputs, is about 31 KB (6.7 KB
gzip). In the final local comparison, valid records with 0/10/100 diagnostics took 2.6/15.0/124.5
microseconds versus 2.7/19.4/154.3 for the old checks. V8 allocation sampling, including collected
objects, estimated about 65% less churn across the same prepared workloads; process RSS peaked
higher during the long stress run, so this is not a claim of lower peak memory. Raw timings and
profiles are local investigation artifacts under `target/toolchain-migration/`. Reopen the choice if
schema import cost or frequent large records become visible in editor latency.

Build and verification never rewrite the checked-in message projections. Use the explicit update
command when the canonical English Fluent resources change, then run the checks to review the
resulting bytes.

The VS Code and VSCodium projections, including their argument lists and output paths, are declared
in `crates/recite-ui/resources/inventory.toml`. The extension reports transport, protocol, and
lifecycle failures through the same localised catalog. Server `showMessage` notifications use the
matching editor severity, while server logs remain in the output channel and genuine process details
are retained alongside their localised failure category. Transient restart attempts stay in the
output channel; exhausted recovery is shown as an error notification.

Build the extension and produce a deterministic VSIX with:

```text
bun run --filter recite-vscode package
```

The package uses the repository's pinned Bun version; Node checks the installed extension boundary
and Bun creates the VSIX archive with Node-compatible APIs; no system archive utility, Electron
runtime, or browser binary is required.

## Language-server configuration

The default server command is `recite-lsp`, resolved through `PATH`. Set `recite.lsp.path` to an
absolute path or to a project-root-relative executable when the binary is not on `PATH`.
`recite.lsp.args` passes an explicit argument array without invoking a shell.
`recite.lsp.projectRoot` optionally selects a project root; otherwise the first workspace folder is
used.

The server process is started when a trusted `.recite` document activates the package, receives
editor document synchronization, and is shut down when the extension deactivates or its
configuration changes. The language client also handles the server's dynamic file-watching
registration. The command palette also adapts the local structured CLI protocol for validation,
compilation, extraction, fixture runs, traces, and a one-process watch loop. Commands use the saved
active `.recite` document where applicable; they never save or execute an untitled/dirty document.
Output is consumed as version-1 NDJSON, while diagnostics are kept in a command-owned collection.
The watch stop command sends the versioned stdin cancellation record and waits for the matching
stopped record before using bounded process recovery.

Set `recite.cli.path` to an absolute or project-root-relative `recite` binary; a bare executable
name is resolved through `PATH`. It is restricted in untrusted workspaces, and commands never invoke
a shell or a hosted service.

Code-action and rename edits are returned through extension-owned commands. The commands keep the
LSP document versions, including zero-edit sibling preconditions, and check them again immediately
before applying the edit. Use `Recite: Rename block` (the `recite.renameBlock` command) for
version-safe block rename. Native F2 rename is disabled by the Recite adapter: VS Code's native
`WorkspaceEdit` path cannot preserve those LSP versions at its eventual apply boundary. The explicit
command is covered by the installed-host activation evidence; native F2 remains intentionally
unsupported by this client.

Relative paths and process spawning use Node's platform-neutral path and process APIs. Linux, macOS,
and Windows are intended hosts, but installed-host evidence is currently Linux-only; platform
packaging and publication smoke remain release work. Run `scripts/check-vscode-host.sh` for
installed VS Code and VSCodium activation checks; it prints the observed host versions and
assertions. The same VSIX can be submitted to the VS Code Marketplace or Open VSX when those
distribution decisions are made.
