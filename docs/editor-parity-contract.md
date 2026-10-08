# Editor parity contract

This is the shared contract for Recite's first-class text authoring surfaces. It describes what an
editor client can rely on and where the client stops being the authority. A partial client is not a
published client, and a packaged artifact is not a published artifact. Installed-host evidence is
recorded incrementally per client: the current host lanes cover Linux x86_64 VS Code, VSCodium,
Neovim, and Zed, while platform, accessibility, and distribution claims remain separate.

The machine-readable companion is
[`fixtures/editor-parity/contract.json`](../fixtures/editor-parity/contract.json). The checker and
the stdio tests are part of the repository gate:

```text
scripts/check-editor-parity.sh
scripts/check-vscode.sh
cargo test --locked -p recite-lsp --test editor_parity
```

## Ownership

Recite's parser, compiler, schema and localisation resolvers, authoring kernel, CLI contracts, and
runtime own meaning. They own source spans, stable IDs, structured diagnostics, completion
candidates, navigation symbols, source preserving edits, project discovery, and deterministic
preview or command records. An editor must project these values; it must not parse Recite or grow a
second validator in TypeScript, Lua, a grammar, or an extension.

The LSP owns the protocol boundary only:

- URI and document-version transport;
- JSON-RPC and LSP request/notification shapes;
- UTF-16 position projection for the initial negotiated encoding;
- full-document synchronisation, open/close/save handling, and protocol-level stale-result mapping;
- cancellation when the server has an explicit cancellation contract.

The clients own activation, file associations, syntax-only highlighting, editor configuration,
command presentation, task/problem integration, packaging, and host accessibility integration.
TextMate and Tree-sitter grammars are tolerant lexical projections. They do not decide whether an
ID, reference, metadata value, condition, effect, or markup construct is valid.

## Document and result rules

The initial LSP position encoding is UTF-16. Positions are measured in UTF-16 code units on a
logical line: CRLF's carriage return is not part of the line, and a non-BMP scalar consumes two code
units. Clients must send the encoding advertised by `initialize`; the conformance fixture includes
CRLF and a non-BMP scalar rather than relying on ASCII-only tests.

Open documents are overlays. Accepted full-document or sequential UTF-16 ranged `didChange` events
update the overlay and produce diagnostics for that version. A change whose version is not greater
than the current open version is stale and is refused without replacing text or publishing a result.
Malformed batches are refused atomically without consuming the version. The server advertises
incremental synchronization and accepts full replacements. A partial or incomplete buffer is still
an editor input: the server may publish parser diagnostics and the client keeps editing; it must not
turn a temporary parse failure into a different language.

Reference results are declaration-first when the declaration is requested, then source-ordered by
URI and range. Clients must preserve that order rather than sorting or deduplicating semantic
locations locally.

Every result that can be applied to source must retain the document URI, version, stable IDs, source
ranges, and diagnostic codes supplied by the shared contract. Clients must not apply an edit against
a document version they no longer have. A result that is stale, unavailable, or cancelled is not
silently presented as current success.

The shared server handles `$/cancelRequest` while analysis and queries run on separate workers. An
observed cancellation returns `RequestCancelled` (-32800) exactly once, with no partial edit result.
Superseded requests return `RequestFailed` (-32803) with `data.reason = "stale_snapshot"`; saturated
query capacity uses `"server_busy"`. Results are checked again at transport handoff. Clients must
not infer cancellation from a request timeout.

Shared stdio and coordinator tests cover cancellation and freshness. Installed-client and non-Linux
evidence remain separate; see the [LSP architecture](lsp-cancellation-design.md).

## Evidence input boundary

Evidence is tied to Git-enumerated source content and modes, not mtimes. Ignored generated output is
excluded; compiler inputs must be tracked or nonignored. Nested repositories and submodules are
unsupported inputs. The exact metadata exclusions, digest algorithm and hostile cases belong to
[`content_digest.py`](../scripts/editor_parity/content_digest.py) and the parity checker, rather
than a second algorithm specification here.

## Structured commands and watch

Finite commands and streaming watch use the [versioned CLI protocol](cli-structured-protocol.md).
Clients pass explicit argv/project roots and preserve typed diagnostics and results. Malformed or
late records are not current success. Native process lifecycle, diagnostic presentation and
cancellation controls belong to each client.

The [VS Code](../editors/vscode/README.md), [Neovim](../editors/recite-neovim/README.md) and
[Zed](../editors/zed/README.md) guides own their concrete command surfaces. VS Code and Neovim
consume structured records; Zed's static tasks display them in a terminal without claiming a
structured diagnostic or cancellation controller. Run `scripts/run-editor-host-check.sh
{vscode|neovim|zed}` for the bounded native lanes.

## Capability and evidence authority

The [structured capability fixture](../fixtures/editor-parity/contract.json) is the single catalogue
of scenarios, clients, artifacts, platform status and executable evidence. Its checker validates
reciprocal artifact references, canonical source inputs, capability coverage and honest status
claims. Documentation is explanatory; CI does not enforce particular prose or historical issue
wording.

Use the fixture's commands to reproduce a capability check. Headless protocol tests, package checks
and installed-host tests prove different surfaces. A host record must identify its client, version,
platform, runner and observed assertions; it cannot upgrade other clients or platforms. Historical
issue references describe provenance, while `follow_up` identifies outstanding work.

Installed-host lanes currently cover bounded Linux x86_64 workflows in VS Code, VSCodium, Neovim and
Zed. They do not establish macOS/Windows behavior, marketplace publication, arbitrary focus
traversal, screen-reader support or complete accessibility. Keyboard workflow evidence covers named
activation, diagnostic navigation, supported commands, status/failure presentation and watch
stopping where available. The broader workbench accessibility surface is separate.

VS Code and VSCodium share one extension and VSIX. Their guarded `recite.renameBlock` command
retains version preconditions; native F2 rename is not registered. Neovim uses a native runtimepath
package and syntax-only Tree-sitter grammar. Zed uses its extension launcher and static tasks:
finite task evidence records argv/cwd/status, watch stopping records terminal Ctrl-C, and neither
claims a structured diagnostic or cancellation controller. Built-in Zed run/trace tasks require
explicit project inputs and remain unsupported.

Shared stdio cancellation and scheduler tests do not establish installed-client cancellation.
Combined LSP schema/catalogue provenance, stale-version host behavior and Zed response-range
conversion retain their explicit fixture status. Keep broader release claims in the fixture only
when the corresponding executable evidence exists.
