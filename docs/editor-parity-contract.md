# Editor parity contract

This is the shared contract for Recite's first-class text authoring surfaces.
It describes what an editor client can rely on and where the client stops being
the authority. A partial client is not an installed or published client, and a
packaged artifact is not a published artifact; the Neovim source integration is
checked in but remains Linux-only partial support.

The machine-readable companion is
[`fixtures/editor-parity/contract.json`](../fixtures/editor-parity/contract.json).
The checker and the stdio tests are part of the repository gate:

```text
scripts/check-editor-parity.sh
scripts/check-vscode.sh
cargo test --locked -p recite-lsp --test editor_parity
```

## Ownership

Recite's parser, compiler, schema and localisation resolvers, authoring kernel,
CLI contracts, and runtime own meaning. They own source spans, stable IDs,
structured diagnostics, completion candidates, navigation symbols, source
preserving edits, project discovery, and deterministic preview or command
records. An editor must project these values; it must not parse Recite or grow a
second validator in TypeScript, Lua, a grammar, or an extension.

The LSP owns the protocol boundary only:

- URI and document-version transport;
- JSON-RPC and LSP request/notification shapes;
- UTF-16 position projection for the initial negotiated encoding;
- full-document synchronisation, open/close/save handling, and protocol-level
  stale-result mapping;
- cancellation when the server has an explicit cancellation contract.

The clients own activation, file associations, syntax-only highlighting,
editor configuration, command presentation, task/problem integration,
packaging, and host accessibility integration. TextMate and Tree-sitter
grammars are tolerant lexical projections. They do not decide whether an ID,
reference, metadata value, condition, effect, or markup construct is valid.

## Document and result rules

The initial LSP position encoding is UTF-16. Positions are measured in UTF-16
code units on a logical line: CRLF's carriage return is not part of the line,
and a non-BMP scalar consumes two code units. Clients must send the encoding
advertised by `initialize`; the conformance fixture includes CRLF and a non-BMP
scalar rather than relying on ASCII-only tests.

Open documents are overlays. An accepted full-document `didChange` replaces
the overlay and produces diagnostics for that version. A change whose version
is not greater than the current open version is stale and is refused without
replacing text or publishing a result. A malformed or ranged change is refused
under the current full-sync contract. A partial or incomplete buffer is still
an editor input: the server may publish parser diagnostics and the client keeps
editing; it must not turn a temporary parse failure into a different language.

Reference results are declaration-first when the declaration is requested, then
source-ordered by URI and range. Clients must preserve that order rather than
sorting or deduplicating semantic locations locally.

Every result that can be applied to source must retain the document URI,
version, stable IDs, source ranges, and diagnostic codes supplied by the shared
contract. Clients must not apply an edit against a document version they no
longer have. A result that is stale, unavailable, or cancelled is not silently
presented as current success.

The current server handles stale versions synchronously and does not yet
implement `$/cancelRequest` or an asynchronous request scheduler. Cancellation
is therefore an explicit unsupported/planned capability in this contract;
clients must not claim cancellation support or infer it from a request timeout.
Issue #53 owns the command/watch lifecycle and the future cancellation contract.

## Structured commands and watch

The intended command boundary is structured: compile, validate, extract, run,
trace, and watch consumers should eventually use typed/versioned records rather
than localised human CLI output. The shared authoring kernel now exposes a
protocol-neutral `BuildStatusProjection` with phase, generation, diagnostics,
freshness, publication, recovery, and cancellation state. Its executable
coverage is partial evidence for the shared lifecycle shape; it is not a CLI
wire contract. Versioned command/watch envelopes, process and binary
integration, cancellation transport, and client integration remain planned
under #53.

## Conformance matrix

The checked-in fixture gives each capability a stable ID, semantic authority,
protocol, canonical scenario, expected structured evidence, edge cases, client
and platform status, and owning follow-up. The rows below are the normative
capability set; the checker rejects drift between this document and the JSON
fixture.

- `lsp.initialize.capabilities`: advertise the supported sync, UTF-16, and LSP feature capabilities from the real server.
- `lsp.publish.diagnostics`: publish structured diagnostics for malformed source through the real LSP transport.
- `lsp.completion.navigation`: project shared-kernel completion and definition results without client-side semantics.
- `lsp.utf16.positions`: preserve ranges across CRLF and non-BMP source text.
- `lsp.overlay.recovery`: accept an incomplete overlay, then refresh it when a newer complete overlay arrives.
- `lsp.stale.version`: refuse an older document version without replacing the current overlay or publishing stale evidence.
- `lsp.cancellation`: document the current unsupported cancellation surface and its owner rather than claiming a timeout is cancellation.
- `command.structured.results`: project the shared `BuildStatusProjection` fields while reserving CLI wire, process, binary, and client integration for #53.
- `editor.filetype.registration`: exercise `.recite` activation and file association through the checked-in Neovim runtimepath package.
- `editor.vscode.syntax-projection`: reserve the syntax-only TextMate projection for #97.
- `editor.neovim.syntax-projection`: record ABI14 Tree-sitter parser/query loading through the Neovim package; the shared grammar remains owned by #98.
- `editor.zed.syntax-projection`: reserve Zed syntax and compatibility evidence for #192.
- `lsp.completion`: project structured completion items from the shared snapshot.
- `lsp.definition`: resolve same-project and cross-file definitions through the shared snapshot.
- `lsp.hover`: project structured hover content and symbol ranges from the shared kernel.
- `lsp.references`: project declaration-first, source-ordered references with explicit declaration inclusion.
- `lsp.rename`: project source-preserving workspace edits for resolved symbols; version preconditions remain incomplete.
- `lsp.code-actions`: project source-preserving stable-ID repairs from the shared kernel.
- `workspace.project.discovery`: discover canonical sibling sources under the configured project root.
- `workspace.configuration`: keep root and project configuration ownership outside client semantics.
- `authoring.stable-id.operations`: reserve the shared-kernel missing-ID repair; broader stable-ID edit preconditions remain incomplete.
- `schema.localisation.resolution`: project the current compiler catalogue identity/fingerprint and CLI locale-fallback evidence; combined LSP schema/catalogue provenance remains planned.
- `command.compile.validate.extract`: reserve versioned structured compile, validate, and extract records for #53; current CLI output is not machine protocol evidence.
- `command.run.trace`: reserve versioned structured runtime and trace records for #53; current CLI tests are not command protocol evidence.
- `command.watch.lifecycle`: project the current protocol-neutral build lifecycle fields; versioned watch wire, process, binary, cancellation transport, and client evidence remain planned for #53.

Executable evidence covers the shared LSP operations, project-root discovery,
the bounded stable-ID repair, compiler catalogue fallback, the compiler's
protocol-neutral build projection, CLI locale fallback through the checked-in
`fixtures/recite/valid/locale_fallback_fr.po` catalogue, the syntax-only
Tree-sitter grammar check, the Neovim runtimepath check, and the checked-in
VS Code/VSCodium package scaffold. The grammar check proves generated-parser
reproducibility, canonical fixture coverage, recovery boundaries, and lexical
captures; the Neovim check adds Linux/0.12.5 filetype, LSP, and ABI14 parser
evidence. The VS Code package check validates the generated VSIX contents and
the Node tests exercise the real `recite-lsp` process over stdio on Linux.
Those checks do not establish installed VS Code or VSCodium host activation,
macOS or Windows support, marketplace publication, or a distributable archive
in source control. This still does not claim a versioned CLI/watch envelope,
process or binary integration, combined LSP schema/catalogue transport,
cancellation transport, native version-safe rename, or a TextMate or Zed
grammar.

Capability rows with direct VS Code/VSCodium package, adapter, or live-server
evidence use `partial` client status and include `scripts/check-vscode.sh` in
their evidence commands. Rows for native rename, command/watch integration,
and other untested client operations remain planned even though the shared
extension artifact exists.

The rows currently draw from these scenarios. The source and schema files are
the canonical fixtures; derived inputs are transformations or protocol events,
not copied Recite or schema sources.

- `lsp-stdio-baseline`: initialize, open, and query the canonical language fixture.
- `diagnostic-recovery`: derive an incomplete overlay from the canonical malformed source, then recover with the canonical valid language fixture.
- `utf16-crlf-non-bmp`: apply CRLF and a non-BMP scalar to the canonical malformed-source case.
- `stale-overlay`: send a newer accepted overlay followed by an older one, then query the current text.
- `stable-id-repair`: derive a missing-ID overlay from the canonical language fixture and request a shared-kernel repair.
- `multi-file-project`: materialize two canonical source fixtures under one root and resolve a qualified cross-file target.
- `client-syntax-projections`: record partial syntax-only Tree-sitter evidence alongside the checked-in VS Code/VSCodium package projection; installed host setup remains untested over the canonical language fixtures.
- `schema-localisation-reference`: combine the canonical manifests and pressure source with the checked-in PO catalogue to exercise the current shared/CLI locale-fallback evidence.
- `command-watch-reference`: exercise the protocol-neutral `BuildStatusProjection`; CLI wire, process, binary, cancellation transport, and client lifecycle evidence remain planned for #53.

## Client, platform, and distribution status

Linux, macOS, and Windows are intended first-class desktop platforms. This
contract records support claims separately so a Linux test run cannot imply
Windows or macOS packaging evidence. At this checkpoint the shared LSP and
the VS Code/VSCodium Node client have partial evidence on Linux only. The
Neovim runtimepath source is checked in and exercised on Linux with Neovim
0.12.5. Neovim 0.10.4 is an explicit compatibility target not yet executed in
this checkout; no marketplace or Open VSX distribution is claimed.

| Client | Shared artifact | Linux | macOS | Windows | Status |
| --- | --- | --- | --- | --- | --- |
| VS Code | checked-in extension scaffold; generated VSIX | partial | planned | planned | partial |
| VSCodium | the same checked-in scaffold and generated VSIX | partial | planned | planned | partial |
| Neovim | checked-in native runtimepath setup plus Tree-sitter grammar; no package distribution | partial | planned | planned | partial |
| Zed | future extension package | planned | planned | planned | planned |

The VS Code and VSCodium partial status is deliberately narrower than host
support: the extension source is checked in, deterministic VSIX generation and
package validation pass, and Linux Node tests exercise a real `recite-lsp`
process. Installed VS Code/VSCodium activation smoke is still missing, as are
macOS and Windows checks. Native rename remains unregistered until a
version-safe adapter exists; structured command and watch integration remains
owned by #53.

VS Code Marketplace and Open VSX are separate distribution claims. Publication,
signing, and installation smoke are still planned. A shared VSIX means the VS
Code and VSCodium clients do not acquire separate semantic implementations; it
does not mean either marketplace already carries an artifact.

## Reopening conditions

This contract should be revised when a shared-kernel operation changes its
wire shape, when the server negotiates another position encoding, when
asynchronous work and cancellation become real protocol behavior, or when a
client/package has executable evidence on a named platform. Such a change must
update the JSON fixture, this document, and the corresponding tests together.

The contract does not cover the GUI workbench, engine embedding, remote
services, marketplace publication, or installed-host compatibility. The
checked-in Tree-sitter grammar remains a syntax artifact; Neovim consumes it
through its runtimepath package, while Zed support remains planned under #192.
