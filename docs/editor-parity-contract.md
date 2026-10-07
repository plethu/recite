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
update the overlay and produces diagnostics for that version. A change whose version is not greater
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

Shared stdio and deterministic coordinator tests cover this contract. Installed editor cancellation
and non-Linux proof remain unclaimed. Issue #206 owns this serious-v1 scheduler/performance
capability, not M4 command/watch work; #53 retains the historical command/watch evidence. See the
[cancellation design and measurements](lsp-cancellation-design.md).

## Evidence input boundary

The parity evidence compiler digest uses Git's path set rather than walking the filesystem. It
includes tracked files and nonignored untracked files, in stable repository-relative byte order, so
source changes still invalidate evidence even when a file's mtime is restored. It deliberately
excludes ignored build output, editor packages, documentation-site output, and Python bytecode;
creating or rewriting those files must not trigger a Cargo evidence rebuild.

Tracked and force-added files remain inputs even when their names resemble an ignored output path
such as `target/`, `node_modules/`, `__pycache__/`, or a `.pyc`/`.pyo` file. The Git index mode and
current worktree permission mode are included too, so executable-bit changes cannot reuse stale
evidence.

The repository-metadata exception is the exact root `CLAUDE.md` path and paths below the exact root
`.claude/` directory. These are agent metadata in this checkout and are excluded before symlink
checks because the tracked checkout intentionally represents them as metadata symlinks. A similarly
named `nested/CLAUDE.md` or `nested/.claude/` path is not metadata and follows the ordinary digest
and symlink rules.

An ignored untracked file is not an accepted compiler-input surface. If a `build.rs`, `include!`,
generated source step, or other compiler action needs a file that is currently ignored, remove the
ignore rule or force-add the file to Git. Force-added files are tracked inputs and therefore count.
The checker does not pretend to discover an arbitrary ignored Cargo input from a pre-compilation
filesystem walk.

The pinned `docs-site/themes/hugo-book` dependency is outside the Cargo compiler boundary and is
excluded, including its gitlink. Other nested repositories and Git submodules are not accepted
digest inputs. Git may enumerate an untracked nested repository as a directory or a staged submodule
as a mode-160000 gitlink; either form fails closed with a controlled checker error. Remove the
nested repository/submodule from the compiler tree or make its source files ordinary repository
inputs before collecting evidence.

## Structured commands and watch

The command boundary is structured for the finite `compile`, `validate`, `extract`, `run`, and
`trace` commands and the streaming `watch` command. Their opt-in version-1 NDJSON contracts are
documented in [`docs/cli-structured-protocol.md`](cli-structured-protocol.md) and exercised by the
external `recite-cli` tests, the VS Code/VSCodium adapter tests, and the Neovim headless command
lane. Configured Linux host runners additionally cover the bounded VS Code/VSCodium, Neovim, and Zed
command paths. Run `scripts/run-editor-host-check.sh {vscode|neovim|zed}` to retain each native
run's log and result under `target/editor-host-evidence/`. The shared CLI remains semantic
authority: clients resolve a local binary, pass argv and the project root, validate every record,
and project typed diagnostics and runtime/watch data without parsing human output. Neovim owns a
separate `vim.system` process lifecycle and one watch child, including cooperative cancel, bounded
teardown, and stale-result fencing. A late or malformed record is a protocol failure. Zed's compile,
validate, extract, and watch entries are static terminal tasks only: the installed Linux host proves
exact argv, cwd, and exit status for the finite tasks and genuine terminal Ctrl-C termination for
watch, while the tasks pass structured output to the host terminal but do not parse records, replace
diagnostics, or provide a fake stdin cancellation controller. Zed intentionally has no built-in
run/trace task because asset, block, and fixture inputs cannot be guessed; a project may add an
explicit task. Zed's installed-host evidence is deliberately partial: its task terminal does not
parse records into a diagnostic controller or expose a native cancellation API. Non-Linux platform
evidence remains outside this contract.

The VS Code/VSCodium adapter deliberately contributes no line-oriented `problemMatcher` or task
definition. Such a matcher would parse localized or nested NDJSON text and would duplicate the
structured boundary. The command adapter's typed `DiagnosticCollection` is the problem integration
for this slice; native task/workbench affordances remain a separate host surface.

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
