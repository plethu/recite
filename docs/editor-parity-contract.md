# Editor parity contract

This contract defines what Recite's first-class text clients can rely on. The
[capability fixture](../fixtures/editor-parity/contract.json) owns client, platform, artifact and
evidence status. A package or partial client does not establish publication or support.
`scripts/check-editor-parity.sh` and the complete gate verify that fixture.

## Ownership

The shared parser, compiler, schema/localisation resolvers, authoring kernel, CLI and runtime own
meaning: IDs, diagnostics, completion/navigation, source-preserving edits, discovery and
deterministic preview. Clients project those values and must not introduce another parser or
validator.

The LSP owns URI/version transport, protocol shapes, position conversion, synchronization and
protocol-level freshness/cancellation. Clients own activation, file associations, configuration,
command presentation, tasks, packaging and host accessibility. TextMate/Tree-sitter remain tolerant
lexical projections, with no authority over IDs, references, schemas or traversal.

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

An observed `$/cancelRequest` returns `RequestCancelled` (-32800) exactly once, with no partial edit
result. Superseded requests return `RequestFailed` (-32803) with `data.reason = "stale_snapshot"`;
saturated query capacity uses `"server_busy"`. Results are checked again at transport handoff.
Clients must not infer cancellation from a request timeout.

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

The fixture is the single catalogue of scenarios, clients, artifacts and executable evidence. Its
checker verifies reciprocal references, canonical source inputs, coverage and status claims. Use its
commands to reproduce checks; historical issues record provenance and `follow_up` records
outstanding work. CI does not require particular prose or historical issue wording.

Protocol, package and installed-host checks establish different surfaces. A host result names the
client/version, platform, runner and assertions; it cannot establish support for another profile,
publication or complete accessibility. Shared stdio cancellation tests do not establish
installed-client cancellation. Broader claims require corresponding executable or native evidence in
the fixture; concrete command surfaces remain in the linked package guides.
