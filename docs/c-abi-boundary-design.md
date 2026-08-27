# Recite C ABI Boundary Design

Design note for `recite-ffi`: the shared C ABI surface used by non-Rust engine
adapters. This is the design record for [#128 Adapters: design the C ABI
boundary for non-Rust engine adapters](https://github.com/plethu/recite/issues/128).

Normative adapter semantics live in `docs/engine-adapter-contract.md` and are
**not** repeated here — this note maps each obligation to a concrete ABI
mechanism. Consult the contract for behavioural requirements; consult this note
for how those requirements cross the language boundary.

## Goals and Non-Goals

**Goals:**
- Settle handle model, payload encoding, string/buffer ownership, error codes,
  and condition protocol — the decisions implemented by the [Unity adapter MVP
  (#73)](https://github.com/plethu/recite/issues/73).
- Produce a stable, narrow surface that future non-Rust adapters (Unreal,
  GameMaker — post-v1) and the deferred `generate-bindings` direction (spec
  §13.9) can layer on without renegotiating the ABI.

**Non-goals:**
- Implementing the `recite-ffi` crate, delivered by [#130](https://github.com/plethu/recite/issues/130).
- C header generation (`cbindgen`), delivered by [#131](https://github.com/plethu/recite/issues/131).
- Typed binding generation (spec §13.9). Post-v1.
- Any adapter MVP work. Each adapter, including the completed Unity MVP in
  [#73](https://github.com/plethu/recite/issues/73), implements against this
  note; they do not live here.
- Changing normative adapter-contract semantics. The contract is authoritative
  and unchanged.

## Why a C ABI

Bevy and Godot adapters link `recite-runtime` directly as a Rust crate. No FFI
is needed because both are Rust (or use a Rust-first bridge like gdext). Unity
gameplay code is C# on Mono or IL2CPP and can only call native code through
P/Invoke, which requires a stable C ABI (`extern "C"` functions in a `cdylib`
or `staticlib`). The C ABI is the lowest common denominator for every non-Rust
host: C++, C#, GDScript-native-extension alternatives, and eventually any
language with a C FFI layer.

## Crate Shape

The new crate is named `recite-ffi`. It is a thin `extern "C"` wrapper over
`recite-runtime` and `recite-core` — exactly the crates the Godot adapter
consumes (`crates/recite-godot/src/adapter.rs`). It has no other Recite
workspace dependencies.

```toml
[lib]
crate-type = ["cdylib", "staticlib"]
```

`cdylib` produces a `.dll`/`.so`/`.dylib` for runtime P/Invoke loading.
`staticlib` is available for host build systems that prefer link-time
integration. Both expose the same `extern "C"` surface; only the link mode
differs.

The crate must not re-implement traversal, session ownership semantics, or error
categories. All of those live in `recite-runtime` and `recite-core`; the FFI
crate is plumbing.

## Handle Model

**Decision: opaque handle-based.** Handles are opaque `u64` identifiers
produced and consumed only through `recite-ffi` functions. The host never
dereferences, copies into its own persistent state, or interprets the handle
bits.

Two handle types:
- **Asset handle** — wraps a decoded `CompiledDialogue` (via `Arc<CompiledDialogue>`
  as in the Godot adapter). Valid until `recite_asset_free` is called.
- **Session handle** — wraps an active `DialogueSession` plus its condition
  registry. Valid until `recite_session_free` or the session ends. The session
  handle carries its own compiled-asset reference (incrementing the `Arc`
  refcount), so freeing the asset handle before the session handle is safe.

Why not raw pointers exposed as `*mut c_void`? Handles decouple the ABI from
Rust's pointer model, allow a validity check on the Recite side before
dereferencing (returning `invalid_handle_error` instead of UB), and avoid
exposing Rust's allocator address space to the host. The `u64` type is
stable across all target pointer widths.

A handle value of `0` is reserved to mean "null / no handle." Every
`recite_*_new` function returns `0` on failure.

Mapping to contract obligations:
- §2 compiled asset identity: the asset handle owns the decoded data; its
  lifetime is explicit and host-managed.
- §3 session ownership: one session handle per declared owner; `recite_session_begin`
  returns `session_already_active_error` if called more than once on the same handle.
- §16.3 (spec): single active session per owner enforced at the FFI boundary.

## Output Payload Encoding

**Decision: MessagePack length-prefixed byte buffers.**

After each session operation (`recite_session_start`, `recite_session_begin`,
`recite_session_choose`,
`recite_session_acknowledge_effect`) the crate writes
a single serialized output batch into a caller-supplied buffer slot (see Buffer
Ownership below). The batch has its own MessagePack envelope and encoder in
`recite-ffi`; it is distinct from the runtime session snapshot codec. The
current batch envelope has `batch_format_version = 0`. Condition callback
payloads have no independent version field: their shape is fixed by this ABI v0
contract and the major-version policy below. A future callback or batch format
change requires an explicitly designed compatibility mechanism (an ABI-major
reset, an additive versioned entrypoint, or a versioned envelope); there is no
negotiation in the current ABI.

**Why not C structs?**
Contract §5 structured output is deeply nested: choice availability reason trees
(`all` / `any` / leaf), projection affordances, deferred effect lists, inline
markup. Attempting to freeze this as a fixed-arity C struct layout would:

- couple the ABI to v0 wire shape that spec §12.2 explicitly permits to change
  before the first tagged release;
- require the host to understand Rust's struct padding rules or depend on a
  repr(C) layout that will widen with every new contract feature;
- duplicate a serialization design that already exists and is already versioned.

MessagePack adds one host-side dependency (a msgpack decoder), but every
supported host language has a mature msgpack library, and the versioned payload
means the ABI can evolve without breaking older host integrations.

The batch output format is versioned with a `batch_format_version` field (u16)
in the envelope. Adapters may reject batches with an unrecognised version and
surface `validation_error`.

**Draining behaviour:** each session call drains traversal synchronously and
returns one ordered output batch. The batch stops at the first prompt,
blocking effect, end event, or structured error. This matches what the Godot
adapter does (`adapter.rs` — it drains until a host-observable boundary). The
host does not need to call `next` in a loop; `recite-ffi` does it internally.
This is the behaviour documented per contract §4.

## Session Lifecycle Functions

Each `extern "C"` function maps to runtime free functions. All functions return
a `ReciteStatus` integer code (see Error Codes); output is written into
caller-supplied out-pointers (see Buffer Ownership).

```
// Asset lifecycle
ReciteStatus recite_asset_load(
    const uint8_t *bytes, uintptr_t len,
    uint64_t *asset_handle_out
);
void recite_asset_free(uint64_t asset_handle);

// Session lifecycle — two-step form (use when conditions appear in opening block)
ReciteStatus recite_session_create(
    uint64_t asset_handle,
    const char *start_block,    // nullable; UTF-8 NUL-terminated; borrowed
    const char *locale,         // nullable; UTF-8 NUL-terminated; borrowed
    uint64_t *session_handle_out
);
ReciteStatus recite_session_begin(
    uint64_t session_handle,
    ReciteBuffer *batch_out     // first output batch
);

// Condition registration (call after recite_session_create, before recite_session_begin)
ReciteStatus recite_session_register_condition(
    uint64_t session_handle,
    const char *name,           // UTF-8 NUL-terminated; borrowed
    ReciteConditionFn handler,  // function pointer; see Conditions section
    void *userdata              // passed back to handler; host owns
);

// Convenience: create + register nothing + begin in one call.
// Use only when no conditions appear in the opening block.
ReciteStatus recite_session_start(
    uint64_t asset_handle,
    const char *start_block,    // nullable; UTF-8 NUL-terminated; borrowed
    const char *locale,         // nullable; UTF-8 NUL-terminated; borrowed
    uint64_t *session_handle_out,
    ReciteBuffer *batch_out     // first output batch
);

// Traversal
ReciteStatus recite_session_choose(
    uint64_t session_handle,
    const char *choice_id,      // UTF-8 NUL-terminated; borrowed
    ReciteBuffer *batch_out
);
ReciteStatus recite_session_acknowledge_effect(
    uint64_t session_handle,
    const char *effect_request_id,  // UTF-8 NUL-terminated; borrowed
    uint8_t ack_completed,          // 1 = Completed, 0 = Failed
    const char *failure_reason,     // nullable; UTF-8 NUL-terminated; borrowed
    ReciteBuffer *batch_out
);

// Save / load
ReciteStatus recite_session_snapshot(
    uint64_t session_handle,
    ReciteBuffer *snapshot_out
);
ReciteStatus recite_session_restore(
    uint64_t asset_handle,
    const uint8_t *snapshot_bytes, uintptr_t snapshot_len,
    uint64_t *session_handle_out,
    ReciteBuffer *batch_out     // resumption batch; empty only at pending prompt
                              // boundary; pending effect re-emits once
);

// Teardown
void recite_session_free(uint64_t session_handle);

// Buffer deallocation
void recite_buffer_free(ReciteBuffer *buf);
```

Runtime function mapping:
- `recite_asset_load` → `decode_compiled_dialogue_messagepack`
- `recite_session_create` → `start_scene_with_options` (no traversal; stores session with `begun: false`)
- `recite_session_begin` → drain via `next_with` (sets `begun: true`; errors if called twice)
- `recite_session_start` → `recite_session_create` + `recite_session_begin` in one call
- `recite_session_choose` → `choose_with` then drain via `next_with`
- `recite_session_acknowledge_effect` → `acknowledge_effect` then drain via `next_with`
- `recite_session_snapshot` → `encode_session_messagepack`
- `recite_session_restore` → `decode_session_messagepack` then drain (re-emits a pending blocking effect once; empty batch at a pending-prompt boundary; `NoActiveSession` for ended-session snapshots)

`EffectAck::Completed` maps to `ack_completed = 1`; `EffectAck::Failed { reason }` maps
to `ack_completed = 0` with the failure reason in `failure_reason`. Both paths
are required by contract §4; hosts that cannot surface failure must pass
`ack_completed = 0, failure_reason = null` rather than silently dropping the error.

## String and Buffer Ownership

**Rule: callee allocates output; host copies then frees.**

```c
typedef struct {
    uint8_t *data;       // heap-allocated by recite-ffi; NULL on error
    uintptr_t len;       // byte length; 0 if data is NULL
} ReciteBuffer;
```

Input strings (`start_block`, `locale`, `choice_id`, etc.) are caller-owned
borrows. They are valid only for the duration of the call. `recite-ffi` never
stores a pointer to caller memory past the function return.

Output buffers (`batch_out`, `snapshot_out`) are allocated by `recite-ffi` on
its Rust allocator. The host must call `recite_buffer_free` exactly once after
consuming the data. Freeing with the wrong allocator is UB; this must be
documented prominently in the generated C header.

Binary payloads (output batches, snapshots) are length-prefixed byte buffers, not
NUL-terminated C strings. NUL bytes may appear inside msgpack data. NUL
termination is used only for the host-facing error detail string (see Error Codes).

All UTF-8. The host must not pass non-UTF-8 bytes in string inputs; `recite-ffi`
validates and returns `validation_error` if encoding is invalid.

## Generated C Header

The committed C header lives at `include/recite.h` and is generated from
`crates/recite-ffi` with `cbindgen.toml`. Downstream adapters, including the
Unity MVP, should consume this header rather than hand-maintaining type or
function declarations.

Run `scripts/generate-ffi-header.sh --write` after changing the FFI surface.
The project gate runs `scripts/generate-ffi-header.sh` without `--write`, which
fails if the committed header is stale.

Header version constants (`RECITE_FFI_VERSION_MAJOR`,
`RECITE_FFI_VERSION_MINOR`, and `RECITE_FFI_VERSION_PATCH`) match the
`recite-ffi` crate version. A major version bump is required for breaking C ABI
changes, including renumbering or removing stable `ReciteStatus` codes. Minor
versions are for additive ABI-compatible symbols, and patch versions are for
documentation or implementation-only changes.

## Error Codes

Every `extern "C"` function returns a `ReciteStatus` (i32). Zero means success;
negative values are error categories. The stable integer assignments are:

```c
typedef enum {
    RECITE_OK                            =  0,
    RECITE_ERR_VALIDATION                = -1,
    RECITE_ERR_ASSET_LOAD_OR_DECODE      = -2,
    RECITE_ERR_STALE_OR_INCOMPATIBLE     = -3,
    RECITE_ERR_SCHEMA_MISMATCH           = -4,
    RECITE_ERR_NO_ACTIVE_SESSION         = -5,
    RECITE_ERR_SESSION_ALREADY_ACTIVE    = -6,
    RECITE_ERR_UNKNOWN_START_BLOCK       = -7,
    RECITE_ERR_INVALID_CHOICE            = -8,
    RECITE_ERR_UNAVAILABLE_CHOICE        = -9,
    RECITE_ERR_STALE_CHOICE              = -10,
    RECITE_ERR_MISSING_CONDITION_HANDLER = -11,
    RECITE_ERR_CONDITION_EVALUATION      = -12,
    RECITE_ERR_INVALID_CONDITION_RESULT  = -13,
    RECITE_ERR_EFFECT_ACKNOWLEDGEMENT    = -14,
    RECITE_ERR_REJECTED_REFRESH          = -15,
    RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY = -16,
    RECITE_ERR_LOCALISATION              = -17,
    RECITE_ERR_MISSING_PROJECTION_HANDLER = -18,
    RECITE_ERR_PROJECTION_EVALUATION     = -19,
    RECITE_ERR_INVALID_PROJECTION_RESULT = -20,
    RECITE_ERR_INVALID_HANDLE            = -21,
    RECITE_ERR_DIALOGUE_FAULT            = -22,
} ReciteStatus;
```

These map directly to the stable machine categories in contract §12 plus two
additional codes for FFI-layer concerns:
- `RECITE_ERR_INVALID_HANDLE` — the host passed an unknown or already-freed
  handle. Not a `DialogueError` variant; detected at the FFI boundary before
  delegating to the runtime.
- `RECITE_ERR_DIALOGUE_FAULT` — maps to `DialogueError::TraversalLimitExceeded`,
  which the Godot adapter (`adapter_error.rs`) maps to `DialogueFault`. This
  indicates a dialogue authoring bug (e.g. an infinite divert), not an API
  misuse.

`RECITE_ERR_REJECTED_REFRESH` covers the `rejected_changed_asset_refresh_error`
contract §12 category; it is raised by the FFI layer when a host attempts to
pass a changed asset to an active session.

Projection error codes (`-18`, `-19`, `-20`) are capability-gated: adapters
that do not expose presentation projection never emit them (contract §12).

**`DialogueError` → `ReciteStatus` mapping:**

| `DialogueError` variant | `ReciteStatus` |
|---|---|
| `UnknownBlock` | `RECITE_ERR_UNKNOWN_START_BLOCK` |
| `UnsupportedCompiledFormat` | `RECITE_ERR_STALE_OR_INCOMPATIBLE` |
| `AssetMismatch` | `RECITE_ERR_STALE_OR_INCOMPATIBLE` |
| `AssetContentMismatch` | `RECITE_ERR_STALE_OR_INCOMPATIBLE` |
| `SchemaMismatch` | `RECITE_ERR_SCHEMA_MISMATCH` |
| `MalformedCompiledAsset` | `RECITE_ERR_ASSET_LOAD_OR_DECODE` |
| `EffectPending` | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT` |
| `NoEffectPending` | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT` |
| `WrongEffectAcknowledgement` | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT` |
| `PromptPending` | `RECITE_ERR_STALE_CHOICE` |
| `NoPromptPending` | `RECITE_ERR_STALE_CHOICE` |
| `InvalidChoice` | `RECITE_ERR_INVALID_CHOICE` |
| `UnavailableChoice` | `RECITE_ERR_UNAVAILABLE_CHOICE` |
| `ConditionEvaluationFailed` | `RECITE_ERR_CONDITION_EVALUATION` (or decoded category; see Conditions) |
| `ConditionResultTypeMismatch` | `RECITE_ERR_INVALID_CONDITION_RESULT` |
| `ConditionDepthLimitExceeded` | `RECITE_ERR_CONDITION_EVALUATION` |
| `UnsupportedSessionSnapshotFormat` | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY` |
| `SessionSnapshotEncodeFailed` | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY` |
| `SessionSnapshotDecodeFailed` | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY` |
| `InvalidSessionSnapshot` | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY` |
| `SessionEnded` | `RECITE_ERR_NO_ACTIVE_SESSION` |
| `TraversalLimitExceeded` | `RECITE_ERR_DIALOGUE_FAULT` |

This mapping is based on the `From<DialogueError> for AdapterError` implementation
in the Godot adapter (`crates/recite-godot/src/adapter_error.rs:140`), which
was the first exercise of all error variants against the contract §12 categories.
The FFI layer re-uses the same logic; it must not diverge. If a new `DialogueError`
variant is added, both the Godot `From` impl and this table must be updated
together.

`recite_session_restore` applies the operation-specific override described in
Save and Load Handoff: `AssetMismatch` and `AssetContentMismatch` become
`RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`, while the typed `SchemaMismatch`
remains `RECITE_ERR_SCHEMA_MISMATCH`.

Each function that returns a non-zero status also writes a NUL-terminated,
UTF-8 detail string into a thread-local that the host can retrieve with
`recite_last_error_message() -> const char*`. The pointer is valid until the
next `recite-ffi` call on the same thread. The host must copy it before calling
further functions.

## Conditions Across the Boundary

**Decision: synchronous callback function pointers.**

```c
typedef struct {
    const char *function_name;   // Recite-owned callback borrow; UTF-8 NUL-terminated
    const uint8_t *args_msgpack; // Recite-owned callback borrow; msgpack argument list
    uintptr_t args_len;
} ReciteConditionQuery;

typedef struct {
    uint8_t ok;                  // 1 = success, 0 = error
    const uint8_t *value_msgpack;// host-owned; Recite-borrowed until callback return
    uintptr_t value_len;         // valid when ok = 1
    const char *error_message;   // host-owned borrow; valid until callback return when ok = 0
} ReciteConditionResult;

typedef ReciteConditionResult (*ReciteConditionFn)(
    const ReciteConditionQuery *query,
    void *userdata
);
```

The host registers one function pointer per condition name before starting the
session. During traversal, `recite-ffi` invokes the matching handler
synchronously — the call is inline with `next_with` traversal, exactly as in
the Godot adapter (`adapter.rs` — `BTreeMap<String, Box<ConditionHandler>>`
with `Fn(ConditionCall<'_>) -> ConditionHandlerResult`).

**Why callbacks, not pre-resolved query batches?**
The alternative (pause traversal, return the pending condition set to the host,
wait for the host to re-enter with answers) is a two-round-trip protocol. It
requires the host to maintain explicit "condition query pending" state between
calls and makes the traversal loop stateful from the host's perspective. For
Unity (Mono/IL2CPP), single-threaded condition evaluation from a P/Invoke call
site is simpler than a polling loop. The callback approach is also what the
Godot MVP proved works under a Rust-foreign-language boundary (Godot
conditions are GDScript `Callable`s invoked through the gdext callback path).

**Threading constraint:** condition callbacks are invoked on the same thread
that called the `recite-ffi` traversal function. They must not call back into
`recite-ffi` (no reentrancy). Hosts that evaluate conditions on a different
thread must marshal via `userdata` and synchronize themselves. This is the same
single-threaded evaluation model the Godot adapter uses.

The three condition error categories from contract §6 map through
`ReciteConditionResult.ok = 0`:
- Handler not registered → `RECITE_ERR_MISSING_CONDITION_HANDLER` (detected in
  `recite-ffi` before invoking the callback, just as the Godot adapter checks
  its `BTreeMap`).
- Handler returns `ok = 0` with a message → `RECITE_ERR_CONDITION_EVALUATION`.
- Handler returns a msgpack value whose type mismatches the schema declaration →
  `RECITE_ERR_INVALID_CONDITION_RESULT` (detected by the runtime during
  `ConditionValue` type validation, as `ConditionResultTypeMismatch`).

The condition result value is a msgpack-encoded `ConditionValue` (bool or enum
variant string). Arguments are a msgpack-encoded list of `ConditionArgument`
values. The host-side msgpack representation must match the schema-declared
parameter types; mismatches produce `RECITE_ERR_INVALID_CONDITION_RESULT`.

### Condition callback MessagePack v0

The callback argument payload is frozen as one MessagePack array. Every item is
an exact two-entry named map with the producer's canonical key order `kind`
followed by `value`. Map key order is not semantically significant to a host
decoder, but duplicate and unknown keys are invalid. The runtime argument order
is the array order; an empty call is encoded as an empty array (`90`). The five
records are:

| Runtime argument | `kind` | `value` |
| --- | --- | --- |
| `Identifier(&str)` | `identifier` | UTF-8 string |
| `String(&str)` | `string` | UTF-8 string |
| `Integer(i64)` | `integer` | signed i64 using the shortest MessagePack integer marker |
| `Float(f64)` | `float` | finite float64 |
| `Boolean(bool)` | `boolean` | MessagePack boolean |

For example, `[identifier("sword"), string("hazel"), integer(3),
float(1.5), boolean(true)]` is produced as the following canonical bytes:

```text
95
82 a4 6b696e64 aa 6964656e746966696572 a5 76616c7565 a5 73776f7264
82 a4 6b696e64 a6 737472696e67     a5 76616c7565 a5 68617a656c
82 a4 6b696e64 a7 696e7465676572   a5 76616c7565 03
82 a4 6b696e64 a5 666c6f6174       a5 76616c7565 cb 3ff8000000000000
82 a4 6b696e64 a7 626f6f6c65616e   a5 76616c7565 c3
```

The result map uses the same named-map convention and is exactly either
`{"kind":"bool","value":<bool>}` or
`{"kind":"enum","variant":<UTF-8 string>}`. The producer emits the keys
in that order. On the result side, `ok` is exactly `0` or `1`: `0` reports
`RECITE_ERR_CONDITION_EVALUATION` (a null error pointer uses a stable fallback),
and `1` requires a non-null, non-empty, complete result map. Scalars, maps with
missing, duplicate, or unknown keys, wrong field types, truncated payloads, and
trailing bytes are rejected as `RECITE_ERR_INVALID_CONDITION_RESULT`.

The native query bytes and function-name pointer are Rust-owned borrows valid
only during the synchronous callback. Host result bytes and error strings are
host-owned borrows valid only until callback return. Hosts must copy anything
they retain, and callbacks must not re-enter `recite-ffi`.

## Threading and Reentrancy

A session handle is not thread-safe. The host must not call `recite-ffi`
functions on the same session handle from multiple threads concurrently. This
mirrors the Rust `!Sync` nature of `DialogueSession`.

An asset handle is safe to share across threads for reading (backed by
`Arc<CompiledDialogue>`), but `recite_asset_free` must not race with any session
that holds a reference to the same asset.

`recite-ffi` functions are not reentrant. A condition callback must not call
any `recite-ffi` function.

The host's `userdata` pointer is passed back to condition callbacks as-is.
`recite-ffi` does not dereference or hold it. The host must ensure it remains
valid and accessible on the calling thread for the duration of the traversal
call.

## Save and Load Handoff

`recite_session_snapshot` encodes the complete runtime session state as a
length-prefixed msgpack byte buffer (via `snapshot_session` +
`encode_session_messagepack`). The host treats this as opaque: stores it in its
game save data, reads it back, and passes it to `recite_session_restore` later.

`recite_session_restore` reconstructs the session by validating the snapshot
against the supplied asset handle (via `decode_session_messagepack` +
`restore_session`). A schema-fingerprint difference returns
`RECITE_ERR_SCHEMA_MISMATCH`; schema comparison is performed first, so a
snapshot that differs in both schema and another identity/content field still
returns `RECITE_ERR_SCHEMA_MISMATCH`. All other asset identity/content
differences during restore return `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`. This
operation-specific mapping keeps schema drift actionable without changing
ordinary runtime stale-asset handling. The call still enforces the contract §9
requirement that session state is tied to a specific compiled asset.

The host must not deserialize, modify, or re-serialize the snapshot bytes.
Doing so silently breaks deterministic resume (contract §9 — the snapshot
includes trace counters, the divert stack, pending blocking effects, and other
determinism-critical state).

If a blocking effect was pending when the snapshot was taken, restoring the
session re-emits that effect once in the resumption batch with the same request
ID, and leaves it pending until the host acknowledges it. The stable ID lets the
host reconcile, replay, fast-forward, or treat the effect as complete; the
runtime does not know whether the game-side operation happened before the save
(contract §9).

## Schema Manifest and Projection

Schema manifest production (contract §7) and presentation projection (contract
§5, spec §5.6.1) are not part of the v1 `recite-ffi` surface.

Schema manifests are produced by host build tooling or editor integration that
writes a JSON file read by the Recite compiler and LSP. The schema manifest
format is already host-agnostic and JSON-based; no C ABI is needed for the
manifest production path.

Projection queries are a capability-gated feature. Adapters that expose them
must document the query protocol. For v1, projection in a C ABI context is
deferred: the typed projection surface (`generate-bindings`, spec §13.9) is the
natural fit, and that is post-v1. A Unity MVP that does not expose projection is
conformant. If a v1 Unity adapter chooses to expose projection, it must do so
through an agreed extension to this design (filed as a follow-up) and must not
invent a private FFI shape.

## Relationship to `generate-bindings` (spec §13.9)

The `generate-bindings` direction (post-v1) generates typed host-language
wrappers — C# condition stubs, effect records/enums, typed session service
classes — from schema. Those wrappers target the `recite-ffi` C ABI as their
underlying call surface. Keeping the ABI narrow, handle-based, and versioned
now means the generated layer can add types without changing the ABI underneath.

Specifically: a generated C# `ReciteDialogueService` would P/Invoke into
`recite_session_start`, `recite_session_choose`, etc., and decode the msgpack
output batch into typed C# structs. The C ABI does not need to know about those
typed structs; they are a generation-time concern.

## Follow-Up Issues

The C ABI design and its implementation follow-ups were completed under the
historical `Milestone 8: Engine Adapter Contract`:

1. [#128 Adapters: design the C ABI boundary for non-Rust engine adapters](https://github.com/plethu/recite/issues/128)
   records this design.

2. [#130 FFI: implement recite-ffi crate (extern C surface)](https://github.com/plethu/recite/issues/130)
   delivered the `recite-ffi` workspace member.

3. [#131 FFI: cbindgen header generation and packaging](https://github.com/plethu/recite/issues/131)
   delivered the stable `include/recite.h` header. `pkg-config` or CMake
   find-module support remains out of scope for v1 unless a downstream package
   needs it.

The remaining Unity-facing ABI, refresh, documentation, and packaging work is
owned by the current [Milestone 23: 7 Engine Companions](https://github.com/plethu/recite/milestone/23):

- [#85 Unity: add editor import and refresh workflow](https://github.com/plethu/recite/issues/85)
  applies the ABI and changed-asset policy to the editor workflow.
- [#86 Docs: document engine authoring refresh workflows and reload limits](https://github.com/plethu/recite/issues/86)
  records the supported refresh and ABI distribution boundaries.
- [#133 Unity: prepare Asset Store and UPM distribution package](https://github.com/plethu/recite/issues/133)
  owns the Unity package and native artifact distribution surface.

## Open Items

- **`validation_error` category coverage:** the contract §12 category
  `validation_error` has no direct `DialogueError` variant (it is raised by
  host-level checks such as invalid UTF-8 input, malformed handle, or
  unsupported batch format version). The mapping table above covers all current
  `DialogueError` variants; if future variants add a `Validation` case, the
  table and the `RECITE_ERR_VALIDATION` code are already in place.

- **IL2CPP allocator mismatch:** `recite_buffer_free` must call the same
  allocator that allocated the buffer — i.e. Rust's allocator inside
  `recite-ffi`. If a Unity IL2CPP build links against a different copy of
  `recite-ffi` than the one that produced the buffer (e.g. a statically linked
  runtime vs a pre-built `.dll`), the free call goes to the wrong allocator and
  is undefined behaviour. The current Unity refresh and packaging work in
  [#85](https://github.com/plethu/recite/issues/85) and [#133](https://github.com/plethu/recite/issues/133)
  must document that `recite-ffi` is always distributed as a single pre-built
  `.dll`/`.so` that both Mono and IL2CPP P/Invoke load at runtime — never
  recompiled per backend or statically linked into the Unity player separately.
