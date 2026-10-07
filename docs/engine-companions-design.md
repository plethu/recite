# Engine companion architecture

This design applies the [adapter contract](engine-adapter-contract.md) and
[acceptance matrix](adapter-acceptance-matrix.md) to Bevy, Godot, and Unity. It records the shared
ownership decisions for the engine companions. GitHub milestone
[7 Engine Companions](https://github.com/plethu/recite/milestone/23) owns delivery state. The
interfaces described here are the implementation target; support claims require the checks described
below.

## Runtime ownership

`recite-core` owns canonical data, schema validation, compiled asset identity, and freshness
comparisons. `recite-runtime` owns traversal, conditions, effect requests, localisation resolution,
and serialisable session state.

`recite-adapter` owns the behavior shared by embedded runtime clients:

- a loaded immutable dialogue asset;
- one session per driver, including preparation before the first traversal;
- ordered output batches and the operation transaction boundary;
- operation-aware adapter error classification;
- an owned catalogue implementing the runtime's locale provider capability.

Bevy and Godot call this crate directly. Unity calls it through `recite-ffi`. The shared crate does
not depend on an engine, compiler, filesystem watcher, native handle registry, or UI framework.
Runtime events remain the common Rust output model; each foreign-language boundary owns its
necessary conversion.

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

Engine producers collect declarations through explicit host registration and lower them into the
existing schema contract. Recite's canonical loader, validator, exporter, and fingerprint functions
remain authoritative. Export failure must preserve the previously published manifest.

The shared schema-export command provides the same canonical export path to non-Rust editor tools.
Bevy tooling may call the underlying Rust capability. The CLI's structured protocol supplies
machine-readable build and diagnostic results; engine tools must not parse human output.

Native producers supply their engine kind and stable registration identity. Unity asset GUIDs and
Godot resource UIDs or explicit registration IDs identify the owner; paths remain diagnostic
provenance. A temporary TOML declaration is transport to the canonical validator, not a transfer of
source ownership to a standalone producer. Core computes the producer fingerprint after applying
that identity, without changing ordinary standalone exports.

Godot owns declaration resources and editor import/export commands. Unity owns editor registration
and asset-backed declarations in an editor-only assembly. Bevy owns explicit Rust registration and
an export entrypoint. Generated manifests remain inputs to the compiler, LSP, and writer without
requiring an engine process. Compilation and watch orchestration reuse `recite-build` and the CLI;
they are not dependencies of the shipped runtime integration.

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

Support is recorded by exact engine version, platform, architecture, scripting backend where
relevant, and artifact. The starting Godot target is the pinned 4.6 API and 4.6.3 host. Unity 6.7+
is the primary development and runtime modernization target; the declared 2022.3 minimum is one
pinned, best-effort compatibility lane with a Mono player check. The 6.7 primary lane checks an
IL2CPP player and probes experimental CoreCLR separately. The managed .NET fixture establishes
neither host backend. Bevy's version is pinned to 0.19.1 with direct app, ECS, asset, and reflection
dependencies. The dialogue integration does not enable rendering or windowing by default.

| Initial verification target                       | Required host evidence                                                                                      |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Bevy 0.19.1, Linux x86_64                         | Real `App` and `AssetServer`, external consumer executable                                                  |
| Godot 4.6.3 official standard build, Linux x86_64 | GDExtension import, Resource persistence, Node signals, clean addon consumer                                |
| Unity 6.7.0b2, Linux x86_64                       | Primary Editor import, EditMode/PlayMode on Editor Mono, IL2CPP player, separate experimental CoreCLR probe |
| Unity 2022.3.62f3, Linux x86_64                   | Best-effort minimum compatibility: import, EditMode/PlayMode, and Mono player                               |

These are verification targets, not blanket support declarations. Named Linux host runs are
preserved as
[historical evidence](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#engine-companions).
Other platforms require native builds and host evidence. Experimental CoreCLR checks do not
establish production support; the managed .NET fixture does not establish an engine host backend.

Verification has three distinct layers:

1. Shared and host-independent tests exercise session semantics, structured failures, rollback,
   localisation, and ABI ownership.
2. Adapter runners execute the versioned conformance scenarios through actual Bevy, Godot, and Unity
   surfaces. Required adapter scenarios cannot be counted as passed merely because the runtime
   reference runner skips them.
3. Clean consumer projects install packaged artifacts, import content, run the example, exercise
   refresh and save/load, and build the declared player or executable target.

The common example must cover conditions, typed effect requests, locale and variant selection,
save/load at a prompt and blocking effect, and an edit, rebuild, import, next-session refresh loop.
Performance evidence records loading, idle cost, output conversion, and retained revisions on a
named profile. Unavailable engine or platform checks remain explicit gaps in the acceptance matrix;
source inspection and successful managed compilation do not replace them.
