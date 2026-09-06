# Editor parity contract

This is the shared contract for Recite's first-class text authoring surfaces.
It describes what an editor client can rely on and where the client stops being
the authority. A partial client is not a published client, and a packaged
artifact is not a published artifact. Installed-host evidence is recorded
incrementally per client: the current host lanes cover Linux x86_64 VS Code,
VSCodium, Neovim, and Zed, while platform, accessibility, and distribution
claims remain separate.

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

## Evidence input boundary

The parity evidence compiler digest uses Git's path set rather than walking the
filesystem. It includes tracked files and nonignored untracked files, in stable
repository-relative byte order, so source changes still invalidate evidence even
when a file's mtime is restored. It deliberately excludes ignored build output,
editor packages, documentation-site output, and Python bytecode; creating or
rewriting those files must not trigger a Cargo evidence rebuild.

Tracked and force-added files remain inputs even when their names resemble an
ignored output path such as `target/`, `node_modules/`, `__pycache__/`, or a
`.pyc`/`.pyo` file. The Git index mode and current worktree permission mode are
included too, so executable-bit changes cannot reuse stale evidence.

The only repository-metadata exception is the exact root `CLAUDE.md` path and
paths below the exact root `.claude/` directory. These are agent metadata in
this checkout and are excluded before symlink checks because the tracked
checkout intentionally represents them as metadata symlinks. A similarly named
`nested/CLAUDE.md` or `nested/.claude/` path is not metadata and follows the
ordinary digest and symlink rules.

An ignored untracked file is not an accepted compiler-input surface. If a
`build.rs`, `include!`, generated source step, or other compiler action needs a
file that is currently ignored, remove the ignore rule or force-add the file to
Git. Force-added files are tracked inputs and therefore count. The checker does
not pretend to discover an arbitrary ignored Cargo input from a pre-compilation
filesystem walk.

Nested repositories and Git submodules are not accepted digest inputs. Git may
enumerate an untracked nested repository as a directory or a staged submodule
as a mode-160000 gitlink; either form fails closed with a controlled checker
error. Remove the nested repository/submodule from the compiler tree or make
its source files ordinary repository inputs before collecting evidence.

## Structured commands and watch

The command boundary is structured for the finite `compile`, `validate`,
`extract`, `run`, and `trace` commands and the streaming `watch` command. Their
opt-in version-1 NDJSON contracts are documented in
[`docs/cli-structured-protocol.md`](cli-structured-protocol.md) and exercised
by the external `recite-cli` tests, the VS Code/VSCodium adapter tests, and the
Neovim headless command lane. Installed Linux host records additionally cover
the bounded VS Code/VSCodium, Neovim, and Zed command paths. The shared CLI remains semantic authority:
clients resolve a local binary, pass argv and the project root, validate every
record, and project typed diagnostics and runtime/watch data without parsing
human output. Neovim owns a separate `vim.system` process lifecycle and one
watch child, including cooperative cancel, bounded teardown, and stale-result
fencing. A late or malformed record is a protocol failure. Zed's compile,
validate, extract, and watch entries are static terminal tasks only: they pass
structured output to the host terminal but do not parse records, replace
diagnostics, or provide a fake stdin cancellation controller. Zed intentionally
has no built-in run/trace task because asset, block, and fixture inputs cannot
be guessed; a project may add an explicit task. Zed's installed-host evidence
is deliberately partial: its task terminal does not parse records into a
diagnostic controller or expose a native cancellation API. Non-Linux platform
evidence remains outside this contract.

The VS Code/VSCodium adapter deliberately contributes no line-oriented
`problemMatcher` or task definition. Such a matcher would parse localized or
nested NDJSON text and would duplicate the structured boundary. The command
adapter's typed `DiagnosticCollection` is the problem integration for this
slice; native task/workbench affordances remain a separate host surface.

## Conformance matrix

The checked-in fixture gives each capability a stable ID, semantic authority,
protocol, canonical scenario, expected structured evidence, edge cases, client
and platform status, and owning follow-up. The rows below are the normative
capability set; the checker rejects drift between this document and the JSON
fixture. A record's `artifact` is its primary artifact; an optional `artifacts`
array names the complete supporting set, must be unique, and must include the
primary artifact. A partial client may have a partial or implemented primary;
an implemented client and any partial or implemented distribution require an
implemented primary artifact.

- `lsp.initialize.capabilities`: advertise the supported sync, UTF-16, and LSP feature capabilities from the real server; installed VS Code/VSCodium and Zed Linux host crossings are recorded.
- `lsp.publish.diagnostics`: publish structured diagnostics for malformed source through the real LSP transport; installed Linux projection is recorded for all four clients.
- `lsp.completion.navigation`: project shared-kernel completion and definition results without client-side semantics; installed VS Code/VSCodium and Zed Linux responses are recorded.
- `lsp.utf16.positions`: preserve ranges across CRLF and non-BMP source text; the installed-host record is limited to VS Code/VSCodium because Zed non-BMP behavior is not claimed.
- `lsp.overlay.recovery`: accept an incomplete overlay, then refresh it when a newer complete overlay arrives; installed VS Code/VSCodium recovery is recorded.
- `lsp.stale.version`: refuse an older document version without replacing the current overlay or publishing stale evidence.
- `lsp.cancellation`: document the current unsupported cancellation surface and its owner rather than claiming a timeout is cancellation.
- `command.structured.results`: project typed/versioned finite CLI command records through the shared VS Code/VSCodium and Neovim adapters; no human stderr/output parsing is permitted.
- `editor.filetype.registration`: exercise `.recite` activation and file association through the checked-in Neovim runtimepath, VS Code/VSCodium package, and Zed language package projections; installed Linux activation is recorded for each client.
- `editor.vscode.syntax-projection`: project the checked-in syntax-only TextMate grammar and deterministic VSIX for VS Code/VSCodium; installed activation is covered but rendered syntax and non-Linux platforms remain untested.
- `editor.neovim.syntax-projection`: record ABI14 Tree-sitter parser/query loading through the Neovim package; the shared grammar remains owned by #98.
- `editor.zed.syntax-projection`: check the Zed language package, exact highlights-query projection, pinned grammar revision, and lexical capture evidence through `scripts/check-zed.sh`, with installed Linux activation/rendering recorded by `scripts/check-zed-host.sh`.
- `lsp.completion`: project structured completion items from the shared snapshot; installed VS Code/VSCodium and Zed Linux responses are recorded.
- `lsp.definition`: resolve same-project and cross-file definitions through the shared snapshot; installed VS Code/VSCodium and Zed Linux responses are recorded.
- `lsp.hover`: project structured hover content and symbol ranges from the shared kernel; installed VS Code/VSCodium and Zed Linux responses are recorded.
- `lsp.references`: project declaration-first, source-ordered references with explicit declaration inclusion; installed VS Code/VSCodium and Zed Linux responses are recorded.
- `lsp.rename`: project source-preserving workspace edits for resolved symbols through the explicit VS Code/VSCodium `recite.renameBlock` command; the command retains version preconditions while native F2 rename remains unregistered.
- `lsp.code-actions`: project source-preserving stable-ID repairs from the shared kernel; installed VS Code/VSCodium application is recorded. Zed sent the missing-ID request with `RECITE_ID001` and the selected range, but Recite returned the exact empty result and applied no edit, so Zed code actions are unsupported.
- `workspace.project.discovery`: discover canonical sibling sources under the configured project root.
- `workspace.configuration`: keep root and project configuration ownership outside client semantics.
- `authoring.stable-id.operations`: reserve the shared-kernel missing-ID repair; installed VS Code/VSCodium application is recorded, while broader stable-ID edit preconditions remain incomplete.
- `schema.localisation.resolution`: project the current compiler catalogue identity/fingerprint and CLI locale-fallback evidence; combined LSP schema/catalogue provenance remains planned.
- `command.compile.validate.extract`: exercise version-1 structured compile, validate, and extract records through the local-first VS Code/VSCodium and Neovim command adapters; installed VS Code/VSCodium and Zed static task invocation is recorded, while Zed does not parse task records into diagnostics.
- `command.run.trace`: exercise version-1 structured runtime and trace records through the local-first VS Code/VSCodium and Neovim command adapters; installed VS Code/VSCodium projection is recorded, while Zed built-in run/trace remains unsupported because the required asset, block, and fixture are explicit inputs.
- `command.watch.lifecycle`: exercise the version-1 watch wire, argv/cwd process boundary, cooperative cancel, bounded recovery, and typed diagnostic replacement through the VS Code/VSCodium and Neovim adapters; installed Linux start/stop evidence is recorded for VS Code/VSCodium, Neovim, and Zed, while Zed remains a host-terminal process with no parsed diagnostic controller or native cancellation controller.
- `editor.keyboard.workflow`: prove installed-host activation plus the required keyboard-only workflow in named installed VS Code/VSCodium, Neovim, and Zed hosts: reach and navigate diagnostics, invoke supported authoring commands, observe status/failure, and stop a running watch where the host exposes that workflow. The VS Code/VSCodium lane sends `Ctrl+1`, `Ctrl+P`, types `scratch/invalid.recite`, and presses `Return`, then asserts the active URI, `recite` language, and extension activation before using Problems/`F8` and the supported commands. This row is partial and remains owned by open issue #202; package, source, and headless protocol checks are not installed-host keyboard evidence.

Executable evidence covers the shared LSP operations, project-root discovery,
the bounded stable-ID repair, compiler catalogue fallback, the compiler's
protocol-neutral build projection, CLI locale fallback through the checked-in
`fixtures/recite/valid/locale_fallback_fr.po` catalogue, the finite version-1
structured CLI records through the external `recite-cli` command tests and the VS Code/VSCodium
finite and streaming adapter tests, the syntax-only Tree-sitter grammar check,
the Neovim runtimepath check, and the checked-in VS Code/VSCodium TextMate
grammar and package. The Tree-sitter check proves generated-parser
reproducibility, canonical fixture coverage, recovery boundaries, and lexical
captures; the Neovim check adds Linux/0.12.5 filetype, LSP, ABI14 parser, five
finite command operations, and watch cancel/exit evidence. The VS Code package
check validates the generated VSIX contents, including the grammar, and the
Node tests exercise the real `recite-lsp` process over stdio on Linux. The Zed
package check validates the manifest, language config, static task argv,
API-0.7.0 launcher, exact highlights query, and pinned grammar revision. The
source gate also runs the hostile empty/null code-action-result regression. The
installed-host runners add separately recorded Linux x86_64 evidence: VS Code
and VSCodium cover activation, LSP projections, commands, watch stop, and the
bounded keyboard path; Neovim 0.10.4 and 0.12.5 cover activation, diagnostics,
commands, watch stop, and the bounded keyboard path; Zed 1.18.1 covers local
development-extension activation, rendered syntax, LSP requests, static task
status, terminal Ctrl-C, and keyboard navigation/shutdown. Zed code actions are
not included in that positive host matrix: its real missing-ID request returned
an exact empty result with no edit. Host records are
incremental and never upgrade an untested client or platform implicitly.
The pinned TextMate tokenizer snapshots assert exact scopes for blocks, diverts,
plural pipes, interpolation, condition directives, anchors, and hostile
recovery cases. TextMate appearance is theme-controlled; anchor scopes are
merely de-emphasizable, never hidden by the grammar. The checked grammar
fixtures retain authored text and syntax markers without making colour the sole
semantic signal; this source-level property is not installed-host
screen-reader, arbitrary focus-traversal, or high-contrast/accessibility
evidence. These Node scope snapshots and automated host probes remain bounded
to their named checks.

The host runners do not establish macOS or Windows support, Marketplace/Open VSX
or Zed gallery publication, or a distributable archive in source control. They
also do not claim a native text problem matcher or task contribution: typed
command diagnostics are intentionally owned by the structured
`DiagnosticCollection` projection. Combined LSP schema/catalogue transport,
stale-version host behavior, Zed non-BMP behavior, and Zed rename edit
application remain outside this evidence. Zed task terminals do not parse
structured records into diagnostics and expose no native task cancellation
controller; the probe records terminal status and genuine Ctrl-C termination
only.

Capability rows with direct VS Code/VSCodium package, adapter, live-server, or
installed-host evidence, or direct Neovim/Zed command evidence, use `partial`
client status and include the corresponding gate in their evidence commands.
The Zed syntax/filetype and selected LSP rows are `partial` on Linux because
source/package and installed-host checks exist; Zed code actions are explicitly
unsupported because the real missing-ID request returned an exact empty result
with no edit; rows for stale-version,
non-BMP, native rename-edit application, and other untested host operations
remain planned or explicitly unsupported. Zed's static
compile/validate/extract/watch task definitions do not make it a structured
command/watch adapter: task terminals do not parse human or NDJSON output, and
Zed does not expose a native cancellation controller. Built-in Zed run/trace are
unsupported; explicit project tasks remain possible when their inputs are
known. Keyboard, task-panel, diagnostic-panel, colour, and accessibility
behavior remain host surfaces with only the bounded scripted workflow covered.
The `editor.keyboard.workflow` row is the narrower Milestone 4 host-evidence
contract: it is partial and remains owned by issue #202, with exact host
versions, platforms, key sequences, diagnostic navigation, command/status and
failure presentation, and watch stopping recorded where supported. It does not
claim the standalone GUI workbench or the broader Milestone 5 accessibility proof.

The rows currently draw from these scenarios. The source and schema files are
the canonical fixtures; derived inputs are transformations or protocol events,
not copied Recite or schema sources.

- `lsp-stdio-baseline`: initialize, open, and query the canonical language fixture.
- `diagnostic-recovery`: derive an incomplete overlay from the canonical malformed source, then recover with the canonical valid language fixture.
- `utf16-crlf-non-bmp`: apply CRLF and a non-BMP scalar to the canonical malformed-source case.
- `stale-overlay`: send a newer accepted overlay followed by an older one, then query the current text.
- `stable-id-repair`: derive a missing-ID overlay from the canonical language fixture and request a shared-kernel repair.
- `multi-file-project`: materialize two canonical source fixtures under one root and resolve a qualified cross-file target.
- `client-syntax-projections`: record partial syntax-only Tree-sitter evidence alongside the checked-in VS Code/VSCodium TextMate and Zed query/package projections; canonical and malformed inputs remain shared fixtures, while incomplete buffers are derived under `fixtures/editor-parity/vscode/` and `fixtures/editor-parity/zed/`; installed Linux host setup is recorded separately and rendered syntax remains bounded evidence.
- `schema-localisation-reference`: combine the canonical manifests and pressure source with the checked-in PO catalogue to exercise the current shared/CLI locale-fallback evidence.
- `command-watch-reference`: exercise finite and streaming CLI protocol records, local argv/cwd resolution, typed diagnostics, and watch cancellation/recovery through `scripts/check-vscode.sh`; `scripts/check-zed.sh` checks only Zed's static structured-task argv and documents the host-terminal limitation; installed Linux start/stop records are kept separate from parsed adapter evidence.
- `keyboard-workflow`: use the shared language and malformed-source fixtures in named installed hosts to record installed activation plus keyboard-only diagnostic navigation, supported command invocation, status/failure observation, and watch stopping where exposed; the VS Code/VSCodium key sequence opens `scratch/invalid.recite` through `Ctrl+1`, `Ctrl+P`, text entry, and `Return` before asserting activation, and source/package/headless checks remain supporting evidence only.

## Client, platform, and distribution status

Linux, macOS, and Windows are intended first-class desktop platforms. This
contract records support claims separately so a Linux test run cannot imply
Windows or macOS packaging evidence. At this checkpoint the shared LSP and the
VS Code/VSCodium clients have partial evidence on Linux only, including
installed activation and the bounded keyboard path. The Neovim runtimepath
source is checked in and exercised in installed Linux x86_64 Neovim 0.10.4 and
0.12.5 hosts. Zed has partial installed Linux x86_64 evidence through its local
development extension. No client has a macOS or Windows claim, and no
marketplace, Open VSX, or Zed gallery distribution is claimed.

| Client | Shared artifact | Linux | macOS | Windows | Status |
| --- | --- | --- | --- | --- | --- |
| VS Code | checked-in extension scaffold and TextMate grammar; generated VSIX | partial installed-host | planned | planned | partial |
| VSCodium | the same checked-in scaffold and TextMate grammar; generated VSIX | partial installed-host | planned | planned | partial |
| Neovim | checked-in native runtimepath setup plus Tree-sitter grammar; no package distribution | partial installed-host | planned | planned | partial |
| Zed | checked-in extension source, pinned grammar reference, language config/query, static tasks, and API-0.7.0 launcher | partial installed-host | planned | planned | partial |

The VS Code and VSCodium partial status is deliberately narrower than a full
release claim: the extension source and syntax grammar are checked in,
deterministic VSIX generation and package validation pass, Linux Node tests
exercise real `recite-lsp` and `recite` processes, and installed VS Code/VSCodium activation smoke is
covered by the host runner. The host lane
also covers the bounded keyboard workflow, selected LSP projections, command
results, and watch stop. macOS and Windows checks remain absent. Native F2
rename remains unregistered; the explicit `recite.renameBlock` command retains
LSP document versions and refuses stale or closed workspace edits, while the
host probe exercises only its guarded command path. Task/workbench integration
and a native text problem matcher remain outside this structured command slice.

VS Code Marketplace and Open VSX are separate distribution claims. Publication
and signing remain planned even though installed-host smoke now passes from a
local deterministic VSIX. A shared VSIX means the VS Code and VSCodium clients
do not acquire separate semantic implementations; it does not mean either
marketplace already carries an artifact. Zed's local development-extension
install is likewise not gallery publication.

## Reopening conditions

This contract should be revised when a shared-kernel operation changes its
wire shape, when the server negotiates another position encoding, when
asynchronous work and cancellation become real protocol behavior, or when a
client/package or installed host has executable evidence on a named platform.
Such a change must update the JSON fixture, this document, and the corresponding
tests together.

The contract does not cover the GUI workbench, engine embedding, remote
services, or marketplace publication. The `editor.keyboard.workflow` row is the
only installed-host keyboard claim in this contract; it does not turn source,
package, or headless checks into host evidence, and it does not replace the
broader Milestone 5 accessibility proof. The checked-in Tree-sitter grammar
remains a syntax artifact; Neovim consumes it through its runtimepath package.
The Neovim client and distribution records therefore name
`neovim-runtimepath` as their primary artifact and keep `tree-sitter-grammar`
as supporting material. Zed references the upstream grammar at the pinned
revision and projects its query, with exact drift and capture checks in
`scripts/check-zed.sh`; this source/package compatibility evidence does not
establish Zed macOS/Windows support, screen-reader/high-contrast integration,
gallery publication, dynamic tasks, parsed structured command/watch diagnostics,
Zed non-BMP or stale-version behavior, Zed rename edit application, or a native
task cancellation controller. The installed Linux probe does establish its
bounded keyboard navigation and task-terminal shutdown boundary, including
clean private-process exit, but does not widen the semantic contract.
