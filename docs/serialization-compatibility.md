# Serialization compatibility decision

**Status:** accepted for Recite before v1, 2026-08-27

**Issue:** [#138](https://github.com/plethu/recite/issues/138)

This records the product decision for Recite's current binary boundaries. It
settles retention and migration policy; it does not authorise a replacement
codec, a benchmark spike, or a new wire version.

## Decision

Keep the current MessagePack contracts, with each surface governed separately:

| Surface | Current contract | Boundary |
| --- | --- | --- |
| Compiled assets | Deterministic MessagePack v0 (`format_version = 0`, `compiler_compatibility_version = 0`) with fixed arrays and an explicit encoding tag | The compiler, core decoder, adapters, and the [wire synchronization matrix](compiled-wire-synchronization.md) share this contract. |
| Runtime snapshots | MessagePack encoding of `DialogueSessionSnapshot` with its own `snapshot_format_version = 1` | Hosts store snapshot bytes as opaque save data. Restore validates the snapshot against the compiled asset, including pending-effect identity. |
| FFI output batches | Named-map MessagePack with `batch_format_version = 0` | The [C ABI boundary](c-abi-boundary-design.md#output-payload-encoding) owns buffer, status, ordering, and host-copy rules; the batch is not the compiled-asset wire. |
| FFI condition payloads | MessagePack argument arrays and tagged result maps; no independent format version | The current ABI contract fixes this payload. Recite owns the query, name, and argument bytes and lends them to the callback; the host owns result and error bytes, which Recite borrows only until the callback returns. |

These are four compatibility surfaces, even where they currently use the same
codec. They are not one shared version: compiled assets expose their format and
compiler-compatibility versions, snapshots expose their snapshot format and
carry asset identity, and batches expose their batch format. Condition
payloads have no independent version and are fixed by the current ABI
contract. Existing fields and values remain as shipped; a compiler, crate, or
host version does not silently select a different reader. Compact JSON remains
an inspection encoding for fixtures, debugging, and CLI tooling. It is not a
second runtime asset, snapshot, or FFI format.

## Why MessagePack remains

MessagePack already has the smallest complete Recite implementation: its
fixtures, deterministic compiled-table profile, strict decoder validation,
snapshot restore checks, and C# / Godot / Unity-adjacent host paths exist. The
compiled asset profile supplies the application rules that generic MessagePack
does not: fixed array arity, explicit tags, ordered repeated metadata, sorted
lookups, stable IDs, source maps, and fingerprints. Snapshots and FFI values
are structured data rather than asset fingerprints, and already have their own
validation and lifecycle rules.

The existing measured baseline covers the named fixture profiles below. These
are measurements of the current MessagePack path, not claims about another
codec:

| Profile | Compiled asset bytes | Maximum session bytes |
| --- | ---: | ---: |
| `tiny` | 32,845 | 535 |
| `small` | 337,885 | 706 |
| `medium` | 3,416,640 | 1,181 |
| `large` | 17,362,479 | 2,013 |
| `epic` | 30,272,948 | 3,612 |
| `realistic:v1-pack` | 12,235 | 719 |

See the [memory profile report](benchmark-reports/memory-profiles-known-limits.md)
for fixture counts, session checkpoints, and measurement limits. No candidate
format has comparable Recite measurements. Generic claims such as “zero-copy”
or “fast” are not a reason to change a shipped boundary.

The alternatives were considered with these weights: compatibility and
migration 25%; host and platform portability 20%; deterministic
inspectability and recovery 20%; maintainability and authoring ergonomics
15%; measured performance and size potential 10%; and FOSS governance,
licensing, and ecosystem 10%. These weights informed the qualitative record
below; they are not benchmark scores.

| Candidate | Decision record and primary evidence |
| --- | --- |
| MessagePack | Retain. The existing deterministic asset profile, strict readers, fixtures, snapshot restore, and host paths are the only complete Recite implementation. |
| [Deterministic CBOR](https://www.rfc-editor.org/rfc/rfc8949.html#section-4.2) | General future escape hatch. Its deterministic profile is credible, but Recite would still own mapping, limits, validation, and migration. External save inspection does not currently justify that work. |
| [FlatBuffers](https://flatbuffers.dev/evolution/) | Asset-only measured hypothesis. Its direct-read benefit must be demonstrated on Recite's large immutable assets; it is not a snapshot or FFI default. |
| [Protocol Buffers](https://protobuf.dev/programming-guides/serialization-not-canonical/) | Conditional on generated bindings becoming a product requirement. Non-canonical deterministic output makes it a poor default for asset fingerprints. |
| [BSON](https://bsonspec.org/spec.html) | Reject as a default: its document/Mongo ecosystem and duplicate-key behavior do not answer Recite's compatibility problem. |
| [Cap'n Proto](https://capnproto.org/otherlang.html) | Reject as a default: schema/toolchain and cross-language support costs are not justified by an unmeasured layout benefit. |

For compiled assets and snapshots, MessagePack's existing validation and typed
restore paths outweigh a second codec. For FFI batches, named maps and the host
boundary matter more than a schema generator. For condition payloads, the
current ABI ownership and strictness are the contract; no alternate encoding
is implied.
There is no current Bevy serialization consumer, and no candidate has shown a
Recite-level size, allocation, load, or cross-host advantage that repays a
second codec and its migration surface.

## The compiled-asset v0 correction window

For compiled assets only, [§12.2 of the production spec](recite-production-spec.md#122-compiled-format)
permits an intentional v0 wire-shape correction before the first tagged
release. It must update the model, writer, reader, validator, inspection
projection, wire matrix, and focused fixtures together, with the byte change
reviewed as evidence. That is a coordinated decision, never a silent encoder
change. Runtime snapshots, FFI batches, and condition payloads keep their own
contracts; this window does not authorise changes to them. After the first
tagged release, compiled-asset field or tag changes require the format or
compatibility-version rule below.

## Future format gate

A future encoding may be considered only for one named artifact at a time, and
only after a measured Recite requirement or a concrete shipped-host need. An
accepted candidate must provide:

- an explicit encoding identifier, format version, and compatibility version;
- an unambiguous boundary probe or container, rather than a guess from payload
  shape or a MessagePack header;
- a deterministic Recite profile with typed model mappings, limits, malformed
  input rules, and inspection behavior;
- equal typed-model results, stable IDs, source maps, ordered metadata,
  fingerprints, reason trees, effect IDs, locale, and traversal state where the
  artifact carries them;
- measurements of encoded size, release encode/decode time, allocations, and
  relevant load or memory behavior on the named fixtures;
- malformed-input, round-trip, determinism, and old/new reader tests; and
- conformance evidence for every shipped host that claims the artifact.

For FFI, this gate starts with a separate ABI design. It must choose an
ABI-major change, an additive versioned entrypoint, or a versioned envelope,
then define capability/version handling and host rejection against the current
v0 strictness in [#171](https://github.com/plethu/recite/issues/171). This
decision does not claim that negotiation exists today.

The first rollout is additive or dual-read: a new reader may understand the
old format and the new format, while writers continue to produce the old
format until the migration is demonstrated. Unknown versions fail before
payload interpretation. No host guesses a format from incidental fields, and
there is no silent fallback.

The candidate posture is deliberately narrow:

- Deterministic CBOR is the general future escape hatch if a named artifact
  gains a real standards, tooling, or inspection need.
- FlatBuffers is an asset-only hypothesis. It must demonstrate a material
  load or heap benefit for Recite's large immutable assets while preserving the
  current model and inspection path.
- Protocol Buffers is conditional on generated bindings becoming a product
  requirement. It is not the default for canonical compiled asset bytes.
- BSON and Cap'n Proto are rejected as current defaults. Reconsidering either
  would require a new, artifact-specific decision with evidence.

No alternate encoding is being added by this decision.

## Migration and deprecation

When a future format is accepted, migration goes through typed Recite models,
not byte-to-byte translation. The conversion must preserve the stable IDs,
source maps, fingerprints, repeated metadata order, reason trees, effect IDs,
locale, trace counters, and traversal pointers that the named artifact carries.
A decoder rejects unknown encoding or compatibility values before interpreting
the payload.

Retirement is artifact-specific; there is no universal same-major support
window or indefinite support promise. Before any old reader is removed, each
published artifact and host must be rebuilt, converted, explicitly retired, or
covered by a supported reader. The deprecation and removal decision is then
documented for that artifact:

- Compiled assets are rebuildable. Retire the old reader only after supported
  toolchains and hosts, plus published assets, are accounted for and the
  breaking release is documented.
- Durable runtime saves require an old-reader or conversion path, backup and
  release guidance, asset-identity and snapshot validation, and a breaking
  compatibility decision before the old path is retired. Hosts must not
  rewrite opaque snapshot bytes merely to inspect them.
- FFI v0 remains through its ABI-major contract. An additive versioned surface
  does not silently authorise removal of v0; the accounting rule above and an
  explicit ABI-boundary deprecation/removal decision still apply. A separate
  design must define host rejection and ownership before any new batch or
  condition encoding is interpreted. Unity v0 batch rejection is required and
  tracked by [#171](https://github.com/plethu/recite/issues/171).

Length-prefixed buffers, allocator ownership, statuses, callback
non-reentrancy, and condition error categories remain part of the ABI migration
test. Existing hosts keep using the current contract or reject the newer one;
they never infer support from payload shape.

## Evidence required to revisit this decision

A future spike, if one is later authorised, must use the existing deterministic
fixture generator plus a hand-reviewed fixture containing every compiled tag,
ordered repeated metadata, source maps, lookups, fingerprints, all effect
modes, a pending blocking effect, and nested FFI reason trees. It must compare
typed-model equality, deterministic hashes, bytes, release timings,
allocations, malformed inputs, and real Rust/C#/Godot/Unity host decoding.
FlatBuffers and Cap'n Proto must additionally demonstrate whether mapped or
direct reads improve actual Recite load or heap behavior. Protobuf must measure
generated-schema maintenance as well as wire behavior.

This decision is complete when MessagePack v0, JSON inspection, artifact
versioning, migration boundaries, and the candidate gate above are understood
by the production spec and adapter documentation. It does not freeze a future
encoding, promise a support duration that has not been justified, or change a
public wire contract.
