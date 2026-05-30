# LSP Project Index Design

This document records the initial project-index design for Recite's LSP work.
It is the design input for milestone 7 implementation issues, especially the
first LSP lifecycle and document-sync issue. The production requirements remain
in `docs/recite-production-spec.md` sections 14, 18, 19.5, and 22.

## Goals

- Make editor diagnostics and navigation use the same deterministic language
  model as the parser, compiler, schema, and ID validation work.
- Keep unsaved editor text authoritative for all LSP answers.
- Start with full document synchronization and explicit per-file summaries.
- Avoid premature framework and incrementality choices until Recite has measured
  LSP workload data.
- Keep the first implementation small enough to review and test.

## Non-Goals

- This design does not add Rust code, Cargo dependencies, CLI behavior, parser
  behavior, compiler behavior, runtime behavior, or schema behavior.
- Scene manifest discovery is out of scope for the first LSP project index.
- Incremental text patches, `ropey`, and Salsa are deferred.
- Visual editor behavior is out of scope.

## Default Library Choices

Use `lsp-server` plus `lsp-types` for the first implementation.

`lsp-server` gives Recite a small JSON-RPC/LSP transport scaffold without
forcing a larger async service framework. Its published crate metadata lists an
MIT OR Apache-2.0 license and the rust-analyzer repository as its source.

`lsp-types` gives typed LSP request, notification, response, diagnostic, range,
position, and capability payloads. Its repository is MIT licensed. Its stable
README currently describes LSP 3.16 support with proposed 3.17 features behind a
feature flag, so the implementation must check any 3.17-only type before relying
on it.

Do not start with `tower-lsp`. It is MIT OR Apache-2.0, but it imposes more
framework shape than the current milestone needs and its current release pins
`lsp-types = 0.94.1`. Recite can revisit it later if the hand-written server
loop becomes the larger maintenance burden.

Do not start with `ropey`. Full-text sync can store document text as `String`
while Recite validates LSP position conversion and document version handling.
Reconsider `ropey` when large-file latency, line-index maintenance, memory
behavior, or repeated text patch bugs show that a rope is solving a measured
problem.

Do not start with Salsa. The initial dependency graph should remain explicit:
parse a changed file, replace that file's summary, and merge deterministic
indexes from sorted per-file summaries. Reconsider Salsa if project recompute
costs become hard to localize, semantic dependencies stop fitting per-file
summaries, or invalidation code becomes more complex than the language work.

## Architecture

The LSP state is split into four durable concepts.

### `SavedProjectIndex`

`SavedProjectIndex` is the disk-backed project view. It is built from known
project files and stores one summary per saved `.recite` file, plus project-wide
merged indexes derived from those summaries.

The first project discovery pass should include:

- workspace `.recite` files under the opened root, excluding hidden
  directories, `target`, build output, vendored dependency directories, and
  generated output directories;
- one explicit schema manifest path supplied by initialization options or
  server configuration.

If the client supplies explicit Recite source roots, use those instead of
walking the entire workspace root. Until scene manifests land, this fallback
discovery stays conservative: source roots and excludes should be configuration,
not hard-coded assumptions about a game repository.

The first pass should not invent scene manifest discovery. That belongs in a
later issue after the production manifest shape is implemented.

The saved index must have a refresh path after initial load. On save or file
watch notifications for a `.recite` file, re-read that file from disk, replace
its saved summary, and rebuild the saved merged indexes. On create, add the file
if it is inside configured discovery roots. On delete, remove the saved summary
and clear diagnostics for that URI unless an open buffer still overlays it. On
rename, remove the old URI and index the new URI as a create. If file watching
is not available from a client, the implementation should still update saved
state on `didSave` and on `didClose` by re-reading the file before falling back
from open state.

### `OpenDocumentStore`

`OpenDocumentStore` owns editor buffers sent through LSP document sync. It
tracks:

- document URI;
- latest accepted document version;
- full text;
- line index for position conversion;
- parse result and syntax diagnostics for the current text;
- the current per-file summary for the open buffer.

Open documents override disk state. If a file is open, every diagnostic,
completion, definition, hover, reference, rename, and code action query must use
the open-buffer summary rather than the saved summary for that URI.

### `LiveProjectSnapshot`

`LiveProjectSnapshot` is the query view used by LSP handlers. It overlays
`OpenDocumentStore` on top of `SavedProjectIndex`, then exposes merged,
deterministically sorted indexes.

For an open URI, the live snapshot uses the open summary. For a closed URI, it
uses the saved summary. If an open document has syntax errors, its recoverable
summary should still participate where possible so unrelated files can keep
working, but each summary must say which symbol classes are complete enough for
cross-file use. The merged snapshot must not emit cross-file diagnostics from an
incomplete symbol class.

The snapshot is rebuilt after each accepted document change by replacing only
the changed file summary and then merging project indexes from per-file
summaries. This keeps incrementality explicit without making the compiler itself
incremental.

### `SchemaIndex`

`SchemaIndex` is the LSP-readable schema view. It should be loaded from the
explicit schema manifest path and summarized separately from dialogue source
files. It supplies:

- speakers and registry values for completions;
- metadata keys and value policy for diagnostics and completions;
- condition and effect functions for diagnostics, completions, and hover;
- markup policy for inline markup diagnostics;
- origin spans or producer context when the manifest provides them.

When schema loading fails, the LSP should publish structured schema diagnostics
and continue with source-only features that do not need schema data.

When the schema manifest changes on disk or through configuration, reload it,
replace `SchemaIndex`, rebuild the live snapshot, and refresh schema-aware
diagnostics for affected open documents.

## Per-File Summary Shape

Each `.recite` file summary should contain structured values, not prose-only
conventions:

- file URI and normalized project-relative path;
- document version when produced from an open buffer;
- parse diagnostics;
- completeness flags for each cross-file symbol class;
- block definitions with source spans;
- block references with source spans;
- line IDs and choice IDs with source spans;
- missing-ID spans;
- metadata keys and values with spans;
- condition function references with spans;
- effect function references with spans;
- inline markup spans and parsed tags;
- recoverable syntax regions;
- any file-local registry values once the language supports them.

The file URI must be canonicalized before indexing. Prefer `file://` URIs that
round-trip through a canonical absolute path under a configured root, then store
a normalized project-relative path for deterministic sorting and diagnostics.
Different URI spellings for the same file must not produce duplicate summaries.

The merged index should be derived by stable sorting, not by hash-map iteration.
Use project-relative path, source order, and explicit symbol names as tie
breakers. This matters for deterministic diagnostics, completions, references,
and rename edits. A merged index must include only summaries whose completeness
flag covers the symbol class being merged.

## Document Synchronization

The first implementation should advertise full document synchronization:

- `textDocumentSync.openClose = true`;
- `textDocumentSync.change = Full`.
- `textDocumentSync.save = true`.

On `didOpen`, store the full text, parse that buffer, create its per-file
summary, and rebuild the live snapshot.

On `didChange`, require a full-text content change. Since `didChange` is a
notification, stale versions that are older than the latest accepted version for
the same URI should be ignored, logged, and left out of the live snapshot rather
than answered with an error response. For an accepted version, replace the
stored text, reparse only that document, replace its per-file summary, rebuild
merged indexes, and publish diagnostics tagged with that document version.

On `didSave`, re-read the saved file from disk, replace the saved summary, and
rebuild saved indexes. The open buffer remains authoritative until `didClose`
because the editor may keep unsaved changes after a save notification depending
on client behavior.

On `didClose`, remove the open buffer. Before falling back, refresh the saved
summary from disk when the URI is still inside configured discovery roots. The
live snapshot should then fall back to the saved summary for that URI, or drop
the URI if it is not part of the saved project index. Publish diagnostics for
the saved state when useful, otherwise clear diagnostics for that URI.

Full sync does not mean whole-project reparsing. The server asks the client for
full document text on each change, then reparses only the changed document and
merges its replacement summary into the live project snapshot.

## Overlay Semantics

All LSP language features must read from `LiveProjectSnapshot`:

- diagnostics;
- completion;
- hover;
- go-to definition;
- find references;
- rename block;
- missing-ID code actions;
- block-stub code actions;
- schema-entry code actions.

This avoids a common editor bug where diagnostics use unsaved text while
navigation uses saved disk state. A query should either see a consistent live
snapshot or return a cancellation/stale-result response. It should not mix old
and new summaries in one answer.

## LSP Boundary Hazards

### Positions and Ranges

LSP positions are zero-based. LSP ranges are start-inclusive and end-exclusive.
Recite source spans and diagnostics must convert carefully at the boundary and
should keep their internal representation independent from `lsp_types::Range`.

Negotiate `general.positionEncodings` during initialize. Support UTF-16 because
it is the required compatibility path for broad editor support. Add tests for
ASCII, CRLF, multi-byte UTF-8, and non-BMP characters before any semantic LSP
feature depends on position conversion.

### Document Versions

Track the latest accepted version per open document. Diagnostics published for
open documents should include that version. Ignore and log stale `didChange`
notifications rather than allowing an older buffer to overwrite newer text.

Request handlers that start from snapshot version `N` and finish after the same
document has advanced to `N + 1` must either return a response that is still
valid for the old snapshot or report a stale/cancelled result according to the
request type. Rename and code actions should be stricter than hover because they
produce edits.

### Cancellation and Stale Results

LSP cancellation does not let the server leave a request hanging. A cancelled
request still needs a JSON-RPC response. Use `RequestCancelled` when the client
cancelled a request and the server observes that cancellation before completing
work. Use `ServerCancelled` only for requests that Recite explicitly makes
server-cancellable.

Use `ContentModified` only when the server detects that a request result is not
valid because the document changed outside the request's usable snapshot. Do not
use it merely because there are pending, unprocessed document change messages;
the LSP spec allows older-state results to be useful, and clients can cancel
requests they no longer want.

## Diagnostics

LSP diagnostics should reuse Recite diagnostic codes and structured spans. The
server may format a concise human-readable message for the editor, but tests
should assert the diagnostic code, severity, URI, range, and related information
where available.

Diagnostics should be published in deterministic order:

1. URI/project-relative path;
2. start position;
3. end position;
4. diagnostic code;
5. stable message key or message text.

Syntax diagnostics come from the changed file parse. Project diagnostics, such
as unknown block references or duplicate IDs, come from the merged live snapshot.
Schema diagnostics come from `SchemaIndex` and schema-aware validation over the
live snapshot. If a file summary marks a symbol class incomplete, suppress
cross-file diagnostics that depend on that class while still publishing the
file-local syntax diagnostics that explain why the summary is incomplete.

## Testing Direction for Issue 29

The first implementation issue should test:

- initialize capabilities advertise full sync;
- `didOpen`, full-sync `didChange`, and `didClose`;
- open-buffer overlay over saved disk state;
- per-file summary replacement without reparsing unrelated files;
- diagnostics include document versions for open documents;
- stale document changes cannot replace newer text;
- CRLF range conversion;
- non-BMP UTF-16 position conversion;
- cancellation responses for long-running handlers where implemented.

Later semantic LSP tests should reuse parser and compiler fixtures where
possible, asserting structured diagnostics and symbol results instead of prose
snapshots.

## Follow-Up Issue Routing

After this design is approved, update milestone 7 issues only.

Issue 29 should become ready for:

- LSP lifecycle and initialize capabilities;
- full document sync;
- saved-index refresh on `didSave`, `didClose`, file create, file delete, file
  rename, and schema manifest changes;
- source parsing for open documents;
- syntax diagnostics;
- conservative fallback discovery roots and excludes;
- URI canonicalization and duplicate-URI prevention;
- UTF-16 conversion tests;
- stale-version handling;
- open-buffer overlay tests;
- incomplete-summary diagnostic suppression;
- per-file summary replacement tests.

Issues 30 through 33 should remain blocked on issue 29 where they need a live
server, but they should cite this design instead of a vague architecture
blocker.

## Research Notes

- LSP 3.17 defines full text synchronization as `TextDocumentSyncKind.Full` and
  requires document open/change/close support to be implemented together when a
  server supports document sync.
- LSP 3.17 cancellation requires a response even when work is cancelled.
- LSP 3.17 defines `ContentModified` for results invalidated by document
  modification.
- Rust-analyzer's architecture notes are useful background, but Recite should
  not adopt Salsa until its own dependency and invalidation pressure justify it.
- `ropey` is a good candidate if text editing and line indexing become measured
  bottlenecks, but full sync plus tested position conversion is the smaller
  first step.
