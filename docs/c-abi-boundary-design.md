# C ABI ownership and encoding

`recite-ffi` exposes the shared native boundary used by non-Rust engine adapters. The
[generated C header](../include/recite.h) owns exported declarations; the
[adapter contract](engine-adapter-contract.md) owns session and runtime semantics. This guide
explains how handles, buffers, callbacks, status codes and structured payloads carry those contracts
across a language boundary.

FFI changes must preserve the ownership and failure rules below, regenerate the header and run the
FFI and Unity conformance gates. Typed binding generation remains a post-v1 direction; it does not
introduce a second semantic implementation.

## Why a C ABI

Unity and other non-Rust hosts call the shared native runtime through C. Bevy and Godot use Rust
integration directly. The boundary keeps traversal in one implementation while allowing native host
APIs above it.

## Crate Shape

`recite-ffi` is a thin `extern "C"` wrapper over `recite-adapter`, `recite-runtime`, and
`recite-core`. The shared driver owns session lifecycle and transactional draining; FFI owns
handles, buffers, callbacks, and status numbers.

`cdylib` produces a `.dll`/`.so`/`.dylib` for runtime P/Invoke loading. `staticlib` is available for
host build systems that prefer link-time integration. Both expose the same `extern "C"` surface;
only the link mode differs.

The crate must not re-implement traversal, session ownership semantics, or error categories. Runtime
owns traversal; `recite-adapter` owns shared session and error-category behaviour. The FFI crate
owns the C ABI plumbing.

## Handle Model

**Decision: opaque handle-based.** Handles are opaque `u64` identifiers produced and consumed only
through `recite-ffi` functions. The host never dereferences, copies into its own persistent state,
or interprets the handle bits.

There are asset, session and catalogue handles:

- **Asset handle** — wraps a decoded `CompiledDialogue` (via `Arc<CompiledDialogue>` as in the Godot
  adapter). Valid until `recite_asset_free` is called.
- **Session handle** — wraps an active `DialogueSession` plus its condition registry. Valid until
  `recite_session_free`, including after traversal ends. The session handle carries its own
  compiled-asset reference (incrementing the `Arc` refcount), so freeing the asset handle before the
  session handle is safe.
- **Catalogue handle** — owns a validated catalogue revision. Attaching it captures that revision;
  later mutation or disposal of the handle does not change the session's provider.

Zero is the null handle. Unknown or freed handles are rejected, but raw caller pointers still
require valid memory. A session handle enforces its own lifecycle; the host adapter owns the rule
that a scene object or service has at most one active session. FFI handles do not identify host
owners.

## Output Payload Encoding

Payloads are MessagePack values carried in owned buffers with an explicit byte length. There is no
additional length prefix inside the bytes.

After each session operation that drains traversal (`recite_session_start`, `recite_session_begin`,
`recite_session_choose`, `recite_session_acknowledge_effect`, and `recite_session_restore`) the
crate writes a single serialized output batch into a caller-supplied buffer slot (see Buffer
Ownership below). The batch has its own MessagePack envelope and encoder in `recite-ffi`; it is
distinct from the runtime session snapshot codec. The current batch envelope has
`batch_format_version = 0`. Condition callback payloads have no independent version field: their
shape is fixed by this ABI v0 contract and the major-version policy below. A future callback or
batch format change requires an explicitly designed compatibility mechanism (an ABI-major reset, an
additive versioned entrypoint, or a versioned envelope); there is no negotiation in the current ABI.

MessagePack preserves nested structured values without a second fixed C structure hierarchy. Rust
uses Serde; hosts decode the versioned payload into their own typed representation.

The Unity byte-codec probe with MessagePack-CSharp 3.1.11 preserved managed adapter conformance and
reduced allocation. Adoption is deferred under the current self-contained UPM distribution: bundled
assemblies conflict with a consumer's existing MessagePack installation, while upstream uses a
[shared NuGet installation](https://github.com/MessagePack-CSharp/MessagePack-CSharp#unity-support).
Reevaluate when Unity distribution chooses one shared dependency owner. Another timing run or
hands-on session does not resolve that ownership tradeoff; keep the Recite-specific typed projection
and strict malformed-input checks in either implementation.

The batch output format is versioned with a `batch_format_version` field (u16) in the envelope.
Adapters may reject batches with an unrecognised version and surface `validation_error`.

Availability reasons optionally carry `origin`: either `condition_call` with `function` and tagged
`args`, or `requirement_expression` with `source_text`. This is an additive batch-v0 field; its
absence means provenance is unavailable. Hosts preserve both origins and reject unknown kinds.

Each traversal operation returns one ordered batch, stopping at a prompt, blocking effect, ending or
error. Shared adapter semantics govern transactional draining; hosts do not need a separate `next`
loop.

## Asset Metadata

`recite_asset_info` is an additive 0.6.0 entrypoint for importer identity checks. It returns a
separate named MessagePack map with `asset_info_format_version = 0`. The map carries `asset_id`,
`content_fingerprint` (`algorithm` string and binary `digest`), nullable `schema_fingerprint` of the
same shape, numeric `format_version` and `compiler_compatibility_version`, `compiler_version`, and
`source_map_id`. Metadata inspection does not replace runtime compatibility validation. The caller
frees the returned buffer with `recite_buffer_free`.

## Owned Gettext Catalogue

`recite_catalog_add_po` uses the core lossless PO parser and atomically merges a file into an owned
catalogue. Identical duplicate entries are accepted; conflicting translations or plural rules return
`RECITE_STATUS_LOCALISATION` without changing the handle. Fuzzy and obsolete entries are ignored.
Bare gettext contexts serve both line and choice domains; prefixed availability reason and
presentation label contexts retain their distinct domains.

`recite_session_set_catalog` explicitly switches a session from callback mode to an owned catalogue
revision; installing a callback switches back. A session retains its attached revision after the
handle changes or is freed. Refreshing catalogues therefore builds a new candidate handle from all
configured PO files, then attaches it after every import succeeds. For restored sessions, attach the
catalogue after preparation and before begin so the first batch is localized.

## Session Lifecycle Functions

The generated header defines the exported functions and parameter types. The Rust implementation and
FFI contract tests own their mappings to runtime operations.

Create a session or call `recite_session_prepare_restore`, install its condition handlers,
interpolation values and locale configuration, then begin traversal. Failed preparation publishes no
handle; failed begin retains the prepared checkpoint for correction and retry. The two convenience
functions `start` and `restore` begin immediately and are suitable only when no host configuration
is needed. Failures free their prepared session and leave output slots unchanged. The grammatical
variant is not serialized and must be supplied again when restoring.

Choosing a choice or acknowledging an effect drains the next output batch. Restoring a pending
blocking effect re-emits it once; a pending prompt produces an empty resumption batch. Ended
snapshots reject restore with `RECITE_STATUS_NO_ACTIVE_SESSION`.

`EffectAck::Completed` maps to `ack_completed = 1`; failure maps to `ack_completed = 0` with its
reason. Hosts that cannot surface the reason pass `failure_reason = null` and still acknowledge
failure. Free session handles and returned buffers through their matching Recite deallocation
functions.

### Locale provider callback

`ReciteLocaleFn` is a typed, synchronous callback rather than a Rust trait object. Recite supplies a
borrowed `ReciteLocaleQuery` for each line, choice, or plural lookup. The host returns
`ReciteLocaleResult` with a translated template, or `text = NULL` to request the authored source
fallback. Plural results include the selected arm, matched locale/context/key, and ordered candidate
attempts so adapters can preserve resolution traces. The host owns the complete returned pointer
tree—`text`, `error_message`, every matched string, the attempts array, and each attempt string—and
must keep it immutable and valid from callback return until the enclosing Recite API call returns.
Recite copies the tree before that call returns. Stack or callback-local temporaries are forbidden,
and the host must release owner storage only after the enclosing call has returned. This lifetime
rule applies independently to every synchronous call that can traverse (`start`, `choose`,
`acknowledge`, and `restore`); there is no callback-level release point. Hosts must also keep the
callback and `userdata` valid until the session is freed or the provider is cleared. A null session
locale bypasses the callback entirely. For each plural query, hosts must enumerate candidates in
this exact order: the requested variant context (`context&variant`) across the locale's
most-specific-to-base fallback chain, followed by the base context across the same chain. Missing
plural rules, missing entries, empty translations, and fuzzy translations continue to the next
candidate; represent empty or fuzzy catalogue records as
`RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION`. A catalogue conflict must not be reported as a match.
`RECITE_LOCALE_ATTEMPT_MATCHED` terminates the sequence, and its selected arm and matched provenance
must come from the validated plural rule and matching candidate. If no candidate matches, return
`text = NULL`, `selected_arm = -1`, and null match provenance so traversal applies the authored
English source fallback. Violating this ordering is a host contract violation: Recite copies and
reports the supplied attempt sequence but cannot enforce lookup order inside a custom callback.
Callbacks must not re-enter Recite, panic, unwind, or throw across the C ABI. Because these callback
types use `extern "C"`, a Rust panic in a callback aborts before Recite can catch it. C and C++
hosts must enforce the strict non-null, synchronous, no-panic/no-throw/no-unwind contract; C++ hosts
should call through an `extern "C"` wrapper that catches C++ exceptions and returns `ok = 0`. C++
exceptions cannot be caught by Rust. The Unity managed wrapper catches managed exceptions and
returns the same failure result.

## String and Buffer Ownership

Recite allocates output; the host copies it and frees it through the same library. Failed calls
leave caller output slots unchanged. Initialize those slots deliberately; a failure does not promise
a null pointer or zero length.

Input strings (`start_block`, `locale`, `choice_id`, etc.) are caller-owned borrows. They are valid
only for the duration of the call. `recite-ffi` never stores a pointer to caller memory past the
function return.

Output buffers (`batch_out`, `snapshot_out`) are allocated by `recite-ffi` on its Rust allocator.
The host must call `recite_buffer_free` exactly once after consuming the data. Freeing with the
wrong allocator is UB; this must be documented prominently in the generated C header.

Unity must distribute one native `recite-ffi` library for both Mono and IL2CPP P/Invoke. Never free
a buffer through another copy, a separately recompiled backend library or a separately linked
runtime: allocation and free must reach the same library and allocator.

Binary payloads (output batches, snapshots) use a pointer and separate byte length, not
NUL-terminated C strings. NUL bytes may appear inside msgpack data. NUL termination is used only for
the host-facing error detail string (see Error Codes).

All UTF-8. The host must not pass non-UTF-8 bytes in string inputs; `recite-ffi` validates and
returns `RECITE_STATUS_VALIDATION` if encoding is invalid.

Interpolation inputs use the runtime typed scalar model: string, integer, finite float or boolean.
The generated header owns the records and kind constants.

The records and string payloads are borrowed only for the call and copied into session-owned
storage. Duplicate names, invalid UTF-8, unknown kind constants, non-finite floats, and invalid
boolean payloads return `RECITE_STATUS_VALIDATION` without changing the existing map. Passing zero
records clears the map. Interpolation values are deliberately not part of the opaque session
snapshot; hosts must provide them again on restore when the resumption drain needs them.

## Generated C Header

The committed C header lives at `include/recite.h` and is generated from `crates/recite-ffi` with
`cbindgen.toml`. Downstream adapters, including the Unity MVP, should consume this header rather
than hand-maintaining type or function declarations.

Run `scripts/generate-ffi-header.sh --write` after changing the FFI surface. The project gate runs
`scripts/generate-ffi-header.sh` without `--write`, which fails if the committed header is stale.

Header version constants (`RECITE_FFI_VERSION_MAJOR`, `RECITE_FFI_VERSION_MINOR`, and
`RECITE_FFI_VERSION_PATCH`) match the `recite-ffi` crate version. The ABI is currently unreleased
and unstable: 0.x minor revisions may change the interface, and hosts must ship matching headers,
bindings and native libraries. After stabilization, incompatible changes require a major bump; patch
versions remain for documentation and implementation changes.

ABI 0.7 uses prepared creation/restoration, setters and begin in place of the old option-combination
entrypoints. Compiled assets, snapshots and condition payloads keep their independently versioned
encodings; availability-reason origin is an optional addition to batch v0.

## Error Codes

Status-returning operations use `ReciteStatus`: `RECITE_STATUS_OK` means success and negative values
identify failure categories. Free functions and error-message accessors have their own signatures.
The generated header owns the integer assignments; the
[Rust mapping](../crates/recite-ffi/src/error.rs) and shared adapter classification own their
operation-specific translation.

Unknown or freed handles fail at the FFI boundary. Traversal limits map to
`RECITE_STATUS_DIALOGUE_FAULT`; snapshot format, decoding and restore incompatibilities map to
`RECITE_STATUS_SAVE_LOAD_INCOMPATIBILITY`. Projection errors are capability-gated and are not
emitted by adapters without presentation projection.

Each function that returns a non-zero status also writes a NUL-terminated, UTF-8 detail string into
a thread-local that the host can retrieve with `recite_last_error_message() -> const char*`. The
pointer is valid until the next `recite-ffi` call on the same thread. The host must copy it before
calling further functions.

## Conditions Across the Boundary

Register one synchronous callback per condition name before beginning traversal. The callback runs
on the calling thread and receives borrowed function and argument data. The generated header owns
the callback declarations; the MessagePack contract below owns their payloads.

The function pointer must be non-null, synchronous, and non-panicking. Host wrappers must enforce
the no-panic/no-throw/no-unwind contract and return `ok = 0` for an evaluation failure; they must
not unwind across `extern "C"`, re-enter Recite, or retain borrowed query pointers. A Rust panic in
an `extern "C"` callback aborts before Recite can catch it, and a C++ exception must be caught by
the host wrapper before entering Recite.

Condition failures map to these statuses:

- Handler not registered → `RECITE_STATUS_MISSING_CONDITION_HANDLER` (detected in `recite-ffi`
  before invoking the callback, just as the Godot adapter checks its `BTreeMap`).
- Handler returns `ok = 0` with a message → `RECITE_STATUS_CONDITION_EVALUATION`.
- Handler returns a msgpack value whose type mismatches the schema declaration →
  `RECITE_STATUS_INVALID_CONDITION_RESULT` (detected by the runtime during `ConditionValue` type
  validation, as `ConditionResultTypeMismatch`).

The condition result value is a msgpack-encoded `ConditionValue` (bool or enum variant string).
Arguments are a msgpack-encoded list of `ConditionArgument` values. The host-side msgpack
representation must match the schema-declared parameter types; mismatches produce
`RECITE_STATUS_INVALID_CONDITION_RESULT`.

### Condition callback MessagePack v0

The callback argument payload is frozen as one MessagePack array. Every item is an exact two-entry
named map with the producer's canonical key order `kind` followed by `value`. Map key order is not
semantically significant to a host decoder, but duplicate and unknown keys are invalid. The runtime
argument order is the array order; an empty call is encoded as an empty array (`90`). The five
records are:

| Runtime argument   | `kind`       | `value`                                                  |
| ------------------ | ------------ | -------------------------------------------------------- |
| `Identifier(&str)` | `identifier` | UTF-8 string                                             |
| `String(&str)`     | `string`     | UTF-8 string                                             |
| `Integer(i64)`     | `integer`    | signed i64 using the shortest MessagePack integer marker |
| `Float(f64)`       | `float`      | finite float64                                           |
| `Boolean(bool)`    | `boolean`    | MessagePack boolean                                      |

For example, `[identifier("sword"), string("hazel"), integer(3), float(1.5), boolean(true)]` is
produced as the following canonical bytes:

```text
95
82 a4 6b696e64 aa 6964656e746966696572 a5 76616c7565 a5 73776f7264
82 a4 6b696e64 a6 737472696e67     a5 76616c7565 a5 68617a656c
82 a4 6b696e64 a7 696e7465676572   a5 76616c7565 03
82 a4 6b696e64 a5 666c6f6174       a5 76616c7565 cb 3ff8000000000000
82 a4 6b696e64 a7 626f6f6c65616e   a5 76616c7565 c3
```

The result map uses the same named-map convention and is exactly either
`{"kind":"bool","value":<bool>}` or `{"kind":"enum","variant":<UTF-8 string>}`. The producer emits
the keys in that order. On the result side, `ok` is exactly `0` or `1`: `0` reports
`RECITE_STATUS_CONDITION_EVALUATION` (a null error pointer uses a stable fallback), and `1` requires
a non-null, non-empty, complete result map. Scalars, maps with missing, duplicate, or unknown keys,
wrong field types, truncated payloads, and trailing bytes are rejected as
`RECITE_STATUS_INVALID_CONDITION_RESULT`.

The native query bytes and function-name pointer are Rust-owned borrows valid only during the
synchronous callback. Host result bytes and error strings must remain immutable and valid after
callback return until the next condition callback for that session or the enclosing Recite operation
returns, whichever happens first. Recite decodes the result before invoking the next callback, so a
session-owned reusable buffer is sufficient. Callbacks must not re-enter `recite-ffi`.

## Threading and Reentrancy

Hosts serialize operations on a session and keep callbacks and `userdata` valid for that session's
lifetime. Asset handles may be shared for reads. A session retains its own asset reference, so an
asset handle can be freed while an existing session continues; do not race disposal with an
operation still using the disposed handle.

Condition and locale callbacks must not re-enter Recite. Same-thread session-registry re-entry
returns `RECITE_STATUS_VALIDATION` before locking; a reentrant void free records the error and
leaves the handle intact. This protection cannot make foreign pointers valid, catch foreign
exceptions or prevent a callback waiting on another thread that needs Recite's held registry lock.
Callbacks must return their result synchronously without throwing, panicking or unwinding across the
ABI.

## Save and Load Handoff

`recite_session_snapshot` encodes the complete runtime session state as raw MessagePack bytes in an
owned buffer with a separate length. The host treats this as opaque: stores it in its game save
data, reads it back, and passes it to `recite_session_restore` later.

`recite_session_restore` reconstructs the session by validating the snapshot against the supplied
asset handle (via `decode_session_messagepack` + `restore_session`). A schema-fingerprint difference
returns `RECITE_STATUS_SCHEMA_MISMATCH`; schema comparison is performed first, so a snapshot that
differs in both schema and another identity/content field still returns
`RECITE_STATUS_SCHEMA_MISMATCH`. All other asset identity/content differences during restore return
`RECITE_STATUS_SAVE_LOAD_INCOMPATIBILITY`. This operation-specific mapping keeps schema drift
actionable without changing ordinary runtime stale-asset handling. The call still enforces the
contract §9 requirement that session state is tied to a specific compiled asset.

Keep snapshot bytes opaque and restore host-owned conditions, interpolation values and catalogue
configuration before beginning. Pending-effect reconciliation remains the game's responsibility, as
specified in the [adapter save/load contract](engine-adapter-contract.md#9-save-and-load-handoff).

## Schema Manifest and Projection

Schema manifest production (contract §7) and presentation projection (contract §5, spec §5.6.1) are
not part of the v1 `recite-ffi` surface.

Schema manifests are produced by host build tooling or editor integration that writes a JSON file
read by the Recite compiler and LSP. The schema manifest format is already host-agnostic and
JSON-based; no C ABI is needed for the manifest production path.

Projection queries are a capability-gated feature. Adapters that expose them must document the query
protocol. For v1, projection in a C ABI context is deferred: the typed projection surface
(`generate-bindings`, spec §13.9) is the natural fit, and that is post-v1. A Unity MVP that does not
expose projection is conformant. If a v1 Unity adapter chooses to expose projection, it must do so
through an agreed extension to this design (filed as a follow-up) and must not invent a private FFI
shape.

## Relationship to `generate-bindings` (spec §13.9)

The `generate-bindings` direction (post-v1) generates typed host-language wrappers — C# condition
stubs, effect records/enums, typed session service classes — from schema. Those wrappers target the
`recite-ffi` C ABI as their underlying call surface. Keeping the ABI narrow, handle-based, and
versioned now means the generated layer can add types without changing the ABI underneath.
