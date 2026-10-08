# Serialization compatibility decision

This defines Recite's current binary boundaries. Recite is an unpublished work in progress. Its
snapshot contract is being completed as v1; development iterations do not create released
compatibility obligations or require new format numbers.

## Decision

Keep the current MessagePack contracts, with each surface governed separately:

| Surface                | Current contract                                                                                                                         | Boundary                                                                                                                                                                                                                                     |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Compiled assets        | Deterministic MessagePack v0 (`format_version = 0`, `compiler_compatibility_version = 0`) with fixed arrays and an explicit encoding tag | The compiler, core decoder, adapters, and the [shared wire registry](../crates/recite-core/src/compiled/wire.rs) share this contract.                                                                                                        |
| Runtime snapshots      | MessagePack encoding of `DialogueSessionSnapshot` with `snapshot_format_version = 1`                                                     | Hosts store snapshot bytes as opaque save data. Restore validates the canonical compiled payload fingerprint as well as header, source, schema, and pending-effect identity.                                                                 |
| Preview snapshots      | MessagePack preview envelope with `version = 1`                                                                                          | The envelope preserves preview state and embeds the validated session snapshot.                                                                                                                                                              |
| FFI output batches     | Named-map MessagePack with `batch_format_version = 0`                                                                                    | The [C ABI boundary](c-abi-boundary-design.md#output-payload-encoding) owns buffer, status, ordering, and host-copy rules; the batch is not the compiled-asset wire.                                                                         |
| FFI condition payloads | MessagePack argument arrays and tagged result maps; no independent format version                                                        | The ABI owns this payload and its buffer lifetimes. Host result and error storage remains valid after callback return until the next condition callback for that session or the enclosing Recite operation returns, whichever happens first. |

These are five compatibility surfaces, even where they currently use the same codec. They are not
one shared version: compiled assets expose their format and compiler-compatibility versions,
snapshots expose their snapshot format and carry asset identity, and batches expose their batch
format. Condition payloads have no independent version and are fixed by the current ABI contract. A
compiler, crate, or host version does not silently select a different reader. Compact JSON remains
an inspection encoding for fixtures, debugging, and CLI tooling. It is not a second runtime asset,
snapshot, or FFI format.

Readers reject unsupported encoding and compatibility versions, including FFI batch versions, before
interpreting payloads. They never guess support from incidental fields or silently fall back. The
[C ABI contract](c-abi-boundary-design.md#output-payload-encoding) has no current negotiation; a
future FFI format needs an explicit ABI-major change, additive versioned entrypoint or versioned
envelope design.

### The initial v1 snapshot contract

Runtime snapshots carry canonical compiled payload identity, checked before restore reconstructs
saved requests. Header/source fingerprints alone cannot detect changed effect arguments or semantic
tables.

Both session snapshots and preview envelopes use their initial v1 format. Development snapshots may
be discarded and regenerated; there are no released snapshot formats to migrate. Malformed
snapshots, including those missing required payload identity, are rejected. A reader must never
invent missing identity from the asset supplied during restore.

## Why MessagePack remains

MessagePack has a complete validated Recite implementation and real host paths. The
[compiled profile](spec/build-cli.md#122-compiled-format) supplies determinism and model rules that
a generic codec does not. Snapshots and FFI retain their separate validation and lifecycles.

Generate current asset and session sizes with `memory_profile_report` as described in
[profiling and optimisation](profiling-and-optimisation.md). No candidate format has comparable
Recite measurements. Generic claims such as “zero-copy” or “fast” do not justify replacing the
implemented boundary.

| Candidate                                                                                | Decision record and primary evidence                                                                                                                                                                      |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Deterministic CBOR](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2)            | General future escape hatch. Its deterministic profile is credible, but Recite would still own mapping, limits, validation, and migration. External save inspection does not currently justify that work. |
| [FlatBuffers](https://flatbuffers.dev/evolution/)                                        | Asset-only measured hypothesis. Its direct-read benefit must be demonstrated on Recite's large immutable assets; it is not a snapshot or FFI default.                                                     |
| [Protocol Buffers](https://protobuf.dev/programming-guides/serialization-not-canonical/) | Conditional on generated bindings becoming a product requirement. Non-canonical deterministic output makes it a poor default for asset fingerprints.                                                      |
| [BSON](https://bsonspec.org/spec.html)                                                   | Reject as a default: its document/Mongo ecosystem and duplicate-key behavior do not answer Recite's compatibility problem.                                                                                |
| [Cap'n Proto](https://capnproto.org/otherlang.html)                                      | Reject as a default: schema/toolchain and cross-language support costs are not justified by an unmeasured layout benefit.                                                                                 |

## Unpublished format corrections

Before the first tagged release, [§12.2](spec/build-cli.md#122-compiled-format) permits intentional
compiled v0 corrections. Update models, codecs, validation, inspection and fixtures together, with
byte changes reviewed. Snapshot v1 corrections follow the same coordination; discarded development
snapshots need no migration reader. FFI batches and condition payloads keep their own contracts.
After the first release, compiled changes follow explicit format/compatibility version rules.

## Future format gate

The migration requirements below apply when replacing a published format.

Consider a replacement for one named artifact only when a measured Recite requirement or shipped
host needs it. Migration must go through typed Recite models. An accepted candidate must provide:

- an explicit encoding identifier, format and compatibility versions, and an unambiguous boundary
  probe;
- a deterministic profile defining typed mappings, limits, malformed-input rules and inspection;
- preservation of the artifact's IDs, source maps, ordered metadata, fingerprints, reason trees,
  effect IDs, locale, trace counters and traversal state;
- encoded-size, release encode/decode, allocation and relevant load/memory measurements on named
  fixtures;
- typed-model equality, round-trip, malformed-input, determinism and old/new reader tests; and
- conformance through every shipped host claiming the artifact, including ABI buffer lengths,
  allocator ownership, statuses, callback non-reentrancy and condition errors where applicable.

The first rollout is additive or dual-read. Writers keep producing the old format until the
migration is demonstrated.

## Migration and deprecation

The preservation and evidence requirements above apply to each conversion.

Retirement is artifact-specific; there is no universal same-major support window or indefinite
support promise. Before any old reader is removed, each published artifact and host must be rebuilt,
converted, explicitly retired, or covered by a supported reader. The deprecation and removal
decision is then documented for that artifact:

- Compiled assets are rebuildable. Retire the old reader only after supported toolchains and hosts,
  plus published assets, are accounted for and the breaking release is documented.
- Durable runtime saves require an old-reader or conversion path, backup and release guidance,
  asset-identity and snapshot validation, and a breaking compatibility decision before the old path
  is retired. Hosts must not rewrite opaque snapshot bytes merely to inspect them.
- FFI v0 remains through its ABI-major contract. An additive versioned surface does not authorise
  its removal; an explicit ABI deprecation/removal decision and the artifact accounting above still
  apply.

## Evidence required to revisit this decision

An authorised comparison must exercise the gate above using the deterministic fixture generator and
one hand-reviewed fixture covering every compiled tag, repeated metadata, source maps, lookups,
fingerprints, all effect modes, a pending blocking effect and nested FFI reason trees. Include real
Rust, C# and Godot/Unity host decoding. FlatBuffers and Cap'n Proto must also measure how mapped or
direct reads affect load time and heap use; Protobuf must measure generated-schema maintenance
costs.
