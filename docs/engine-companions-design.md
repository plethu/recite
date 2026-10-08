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

The shared driver commits session state only after fallible output conversion succeeds. Signal and
event delivery happen afterwards; arbitrary host actions cannot be rolled back, so conditions remain
pure queries. The runtime owns prompt/blocking/ended states rather than a parallel adapter state
machine.

The adapter tracks observed choices to distinguish stale and unknown selection. Restore can recover
only choices represented in the snapshot; it cannot reconstruct older unselected observations.

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

Unity creates its service in `Awake`: field initializers may run on a loading thread. Handles stay
on that owner thread. Queued post-operation events permit nested runner calls; re-entry from a
native condition callback remains forbidden. The
[Unity package guide](../Packages/com.recite.dialogue/README.md) owns importer-cache, catalogue,
upgrade and host-support details. Godot retains validated bytes beside its derived Resource;
clearing its import cache removes that fallback and requires a valid source import again. Bevy
borrows the catalogue resource per operation, so replacement affects the next operation even in an
active session. This does not replace the compiled dialogue revision.

## Compatibility and verification

Internal Rust and C# APIs may be changed with their consumers. Authored IDs, compiled bytes,
snapshots, existing ABI payloads and status numbers, and persisted engine resources retain their
contracts. An additive ABI capability uses the existing minor-version policy and synchronized
header/binding checks. Extracted behavior replaces its prior implementation in the same change.

Support records belong to the package READMEs and
[authoring checks](engine-authoring-workflows.md#workflow-checks). Distinguish shared semantics,
actual host execution and clean packaged consumers. A managed/native test does not establish Unity
import or player behavior; compiled-only imports cannot establish source/schema freshness.
