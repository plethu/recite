# Serialization compatibility decision

**Status:** accepted for Recite before v1, 2026-08-27

**Issue:** [#138](https://github.com/plethu/recite/issues/138)

This records the product decision for Recite's current binary boundaries. It
settles retention and migration policy; it does not authorise a replacement
codec, a benchmark spike, or a new wire version.

## Decision

Keep MessagePack v0 for each current surface:

| Surface | Current contract | Boundary |
| --- | --- | --- |
| Compiled assets | Deterministic MessagePack v0 (`format_version = 0`, `compiler_compatibility_version = 0`) with fixed arrays and an explicit encoding tag | The compiler, core decoder, adapters, and the [wire synchronization matrix](compiled-wire-synchronization.md) share this contract. |
| Runtime snapshots | MessagePack encoding of `DialogueSessionSnapshot`, independently versioned; the current snapshot format is v1 | Hosts store snapshot bytes as opaque save data. Restore validates the snapshot against the compiled asset, including pending-effect identity. |
| FFI output batches | Named-map MessagePack with `batch_format_version = 0` | The [C ABI boundary](c-abi-boundary-design.md#output-payload-encoding) owns buffer, status, ordering, and host-copy rules; the batch is not the compiled-asset wire. |
| FFI condition payloads | MessagePack argument arrays and tagged result maps | The callback owns the borrowed input and result bytes under the existing C ABI contract. A future encoding change needs an explicit negotiated version. |

These are four compatibility surfaces, even where they currently use the same
codec. Their format and compatibility versions are independent. Existing
fields and values remain as shipped; a compiler, crate, or host version does
not silently select a different reader. Compact JSON remains an inspection
encoding for fixtures, debugging, and CLI tooling. It is not a second runtime
asset, snapshot, or FFI format.

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
licensing, and ecosystem 10%. The resulting ordinal totals (out of 100, not
benchmark results) are:

| Candidate | Compiled assets | Snapshots | FFI batches and conditions |
| --- | ---: | ---: | ---: |
| MessagePack | 92 | 92 | 88 |
| Deterministic CBOR | 79 | 79 | 79 |
| FlatBuffers | 76 | 66 | 66 |
| Protocol Buffers | 70 | 70 | 71 |
| Cap'n Proto | 67 | 65 | 65 |
| BSON | 52 | 52 | 52 |

The evidence does not justify format churn for external save inspection. There
is no current Bevy serialization consumer, and no candidate has yet shown a
Recite-level size, allocation, load, or cross-host advantage that repays a
second codec and its migration surface.

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
not byte-to-byte translation. The conversion must preserve stable IDs, source
maps, fingerprints, repeated metadata order, reason trees, effect IDs, locale,
trace counters, and traversal pointers. A decoder rejects unknown encoding or
compatibility values before interpreting the payload.

Every accepted replacement names a documented same-major migration window and
its end condition. The old reader remains available during that window; this
is a bounded release obligation, not an indefinite support promise.

Compiled assets are rebuildable: regeneration from source is the preferred
path, with the old reader retained while published assets and declared hosts
move across the window. Durable runtime saves require more care: the release
must document backup and conversion behavior, validate asset identity and
snapshot state, and test restore at line, prompt, and pending-blocking-effect
boundaries before deprecating the old reader. Hosts must not rewrite opaque
snapshot bytes merely to inspect them.

FFI changes are negotiated explicitly. A host either advertises and selects a
supported batch or condition-payload version, or Recite rejects the operation
with a structured compatibility error. Existing hosts keep using v0 or reject
the newer contract; they never infer support from payload shape. Length-
prefixed buffers, allocator ownership, statuses, callback non-reentrancy, and
condition error categories remain part of the ABI migration test.

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
