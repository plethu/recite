# Engine companion architecture

The [adapter contract](engine-adapter-contract.md) defines common semantics. This guide explains
ownership choices that differ across Bevy, Godot and Unity; their READMEs own setup and tested
support.

## Runtime ownership

`recite-core` owns canonical data, schema validation, compiled asset identity, and freshness
comparisons. `recite-runtime` owns traversal, conditions, effect requests, localisation resolution,
and serialisable session state.

Bevy and Godot call `recite-adapter` directly; Unity calls it through `recite-ffi`. The shared
driver owns preparation, transactional batches and adapter error classification. Each host owns its
import, callbacks and native output conversion.

The driver borrows `DialogueContext` and `LocaleResolution` for an operation. Host callbacks, engine
object references, interpolation values, and catalogue resources remain outside the serialised
session. This allows a Bevy system to supply game context without imposing Bevy's threading
constraints on Godot callables or foreign callbacks.

An operation prepares a candidate session transition and its ordered outputs. Fallible output
encoding must succeed before the driver commits the candidate. Rejected operations preserve the
previous session. Host signal delivery and game-side effect execution happen after the operation
returns; the adapter cannot undo arbitrary host behavior. Conditions therefore remain pure queries.

Catalogue imports use the existing core PO parser and the shared owned catalogue. An import
validates a complete candidate before replacing the available catalogue. Native catalogue handles
retain ownership at the ABI boundary; attaching one to a session captures a catalogue revision so
later handle mutation or disposal cannot invalidate that session's provider. Bevy borrows its
catalogue resource for each operation; replacing that resource affects subsequent operations,
including an active session. This explicit catalogue update does not replace the session's compiled
dialogue revision.

The prepared/active distinction belongs to the adapter lifecycle. Prompt, blocking-effect, and ended
states remain owned by the runtime rather than being copied into a parallel adapter state machine.

The adapter remembers choice IDs from successfully committed prompt batches to distinguish stale
selections from unknown IDs. This observation history commits with the session operation. Restore
seeds it from the snapshot's previous prompt and selected-choice history; older unselected choices
are not present in the existing snapshot format and cannot be recovered as observations.

## Assets and refresh

All three companions use `reload_for_next_session_only`. An active session retains its original
loaded asset. A successfully validated import becomes available to the next session; a failed import
retains the last valid revision and reports the rejected candidate. The host exposes when the active
revision differs from the available revision.

Compatibility, freshness, and availability are separate observations. Compatibility determines
whether a saved session can resume. Freshness compares compiled content with available source and
schema inputs. Availability identifies the validated revision currently loaded by the engine. A
shipped game without source files can check compatibility without claiming source freshness.

Importers obtain identity from decoded content. Asset names, engine handles, Unity GUIDs, and Godot
resource paths do not replace Recite compatibility identity. Engine-native references may accompany
it as provenance.

## Authoring ownership

Native producers own their declarations and registration identities. They lower into Recite's
canonical schema validator/exporter; failed export preserves the previous manifest. Temporary TOML
is transport to that validator, not a new source owner. Generated manifests remain usable by the
compiler, LSP and Writer without an engine process. Build/watch tooling is not a runtime dependency.

## Engine boundaries

| Host  | Session owner and output                                         | Host-specific ownership                                                                |
| ----- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Bevy  | Documented session resource, ordered requests and output batches | Asset loader, schedule configuration, game-query context, producer entrypoint          |
| Godot | Dialogue Node, structured results and signals                    | Resource persistence, class registration, callables, Variant conversion, import plugin |
| Unity | Managed service and GameObject runner                            | Native handles and callback allocations, managed decoding, importer, editor assemblies |

Bevy exposes schedule points for context preparation, dialogue operations, and output consumption.
Callers must establish ordering between competing request producers. Runtime and adapter
dependencies do not enable a renderer merely to run dialogue.

Godot emits signals after the driver operation has released its mutable borrow. Resource
deserialisation revalidates persisted data. Native class names and existing Resource formats require
deliberate migration if changed.

Signal and event wrappers queue a complete committed batch before delivery. If a handler invokes
another dialogue operation synchronously, its outputs follow the remaining outputs from the current
batch. Reentrant host callbacks must not reorder runtime events.

Unity separates its session facade from native session and catalogue ownership, condition callbacks,
and output decoding. Built-in localisation accepts writer-owned PO documents and uses the shared
native catalogue. The older managed catalogue resolver is removed with its source-level consumers;
the C ABI retains locale callbacks for independent hosts. Disposal follows the ABI's owner-thread
rule. Runtime assemblies exclude editor APIs. An optional DOTS facade must use the same service and
declare its own tested support; the base package does not require Entities.

Unity's importer retains validated compiled bytes in a derived cache under
`Library/Recite/CompiledCache`, keyed by the asset GUID. A rejected candidate can therefore expose
its error while the imported resource retains the last valid revision. Cached bytes are revalidated
before use. Clearing `Library` also clears this fallback; the source must then produce a valid
import again.

Godot retains validated bytes alongside its derived imported Resource. A rejected replacement leaves
the source path loadable and stores the import error on the retained Resource. Clearing the derived
import cache removes this fallback and requires a valid source import again.

## Compatibility and verification

Internal Rust and C# APIs may be changed with their consumers. Authored IDs, compiled bytes,
snapshots, existing ABI payloads and status numbers, and persisted engine resources retain their
contracts. An additive ABI capability uses the existing minor-version policy and synchronized
header/binding checks. Extracted behavior replaces its prior implementation in the same change.

Support records belong to the package READMEs and
[authoring checks](engine-authoring-workflows.md#workflow-checks). Distinguish shared semantics,
actual host execution and clean packaged consumers. A managed/native test does not establish Unity
import or player behavior; compiled-only imports cannot establish source/schema freshness.
