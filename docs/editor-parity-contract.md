# Editor parity contract

This is the shared contract for Recite's first-class text authoring surfaces.
It describes what an editor client can rely on and where the client stops being
the authority. It is deliberately useful before a VS Code, Neovim, or Zed
package exists: a planned client is not an implemented client, and a packaged
artifact is not a published artifact.

The machine-readable companion is
[`fixtures/editor-parity/contract.json`](../fixtures/editor-parity/contract.json).
The checker and the stdio tests are part of the repository gate:

```text
scripts/check-editor-parity.sh
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

The shared command boundary is structured. Compile, validate, extract, run,
trace, and watch consumers use typed/versioned records, not localised human
CLI output. The current human-oriented watch stream is not a machine contract;
its lifecycle, binary discovery, cancellation, and machine-readable status
follow-up is tracked by #53. This parity slice records that limitation and
does not claim command or watch integration merely because the CLI has a human
command today.

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
- `command.structured.results`: reserve parity evidence for the structured command/watch contract owned by #53.

The initial executable evidence covers the first six rows. Cancellation and
structured command/watch rows remain honest planned/unsupported boundaries.
Client packages and syntax grammars are follow-up work; their status is not
upgraded by these server tests.

The rows currently draw from these scenarios. The source and schema files are
the canonical fixtures; derived inputs are transformations or protocol events,
not copied Recite or schema sources.

- `lsp-stdio-baseline`: initialize, open, and query the canonical language fixture.
- `diagnostic-recovery`: derive an incomplete overlay from the canonical malformed source, then recover with the canonical valid language fixture.
- `utf16-crlf-non-bmp`: apply CRLF and a non-BMP scalar to the canonical malformed-source case.
- `stale-overlay`: send a newer accepted overlay followed by an older one, then query the current text.
- `schema-localisation-reference`: reserve shared schema and localisation evidence for the existing canonical manifests and pressure fixture.
- `command-watch-reference`: reserve structured command and watch lifecycle evidence for #53.

## Client, platform, and distribution status

Linux, macOS, and Windows are intended first-class desktop platforms. This
contract records support claims separately so a Linux test run cannot imply
Windows or macOS packaging evidence. At this checkpoint the shared LSP has
partial protocol evidence on Linux only; no client artifact is implemented.

| Client | Shared artifact | Linux | macOS | Windows | Status |
| --- | --- | --- | --- | --- | --- |
| VS Code | one future VSIX | planned | planned | planned | planned |
| VSCodium | the same future VSIX | planned | planned | planned | planned |
| Neovim | future setup/grammar package | planned | planned | planned | planned |
| Zed | future extension package | planned | planned | planned | planned |

VS Code Marketplace and Open VSX are separate distribution claims. Packaging,
publication, signing, and installation smoke are all still planned. A shared
VSIX means the VS Code and VSCodium clients do not acquire separate semantic
implementations; it does not mean either marketplace already carries an
artifact.

## Reopening conditions

This contract should be revised when a shared-kernel operation changes its
wire shape, when the server negotiates another position encoding, when
asynchronous work and cancellation become real protocol behavior, or when a
client/package has executable evidence on a named platform. Such a change must
update the JSON fixture, this document, and the corresponding tests together.

The contract does not cover the GUI workbench, engine embedding, remote
services, marketplace publication, or client implementation. Those remain the
separate milestone and issue surfaces named in the fixture.
