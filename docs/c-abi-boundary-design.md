# Recite C ABI Boundary Design

`recite-ffi` exposes the shared native boundary used by non-Rust engine adapters. The
[generated C header](../include/recite.h) owns exported declarations; the
[adapter contract](engine-adapter-contract.md) owns session and runtime semantics. This guide
explains how handles, buffers, callbacks, status codes and structured payloads carry those contracts
across a language boundary.

FFI changes must preserve the ownership and failure rules below, regenerate the header and run the
FFI and Unity conformance gates. Typed binding generation remains a post-v1 direction; it does not
introduce a second semantic implementation.

## Why a C ABI

Bevy and Godot adapters link the Rust `recite-adapter`/`recite-runtime` crates. No FFI is needed
because both are Rust (or use a Rust-first bridge like gdext). Unity gameplay code is C# on Mono or
IL2CPP and can only call native code through P/Invoke, which requires a stable C ABI (`extern "C"`
functions in a `cdylib` or `staticlib`). The C ABI is the lowest common denominator for every
non-Rust host: C++, C#, GDScript-native-extension alternatives, and eventually any language with a C
FFI layer.

## Crate Shape

`recite-ffi` is a thin `extern "C"` wrapper over `recite-adapter`, `recite-runtime`, and
`recite-core`. The shared driver owns session lifecycle and transactional draining; FFI owns
handles, buffers, callbacks, and status numbers.

```toml
[lib]
crate-type = ["cdylib", "staticlib"]
```

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

Two handle types:

- **Asset handle** — wraps a decoded `CompiledDialogue` (via `Arc<CompiledDialogue>` as in the Godot
  adapter). Valid until `recite_asset_free` is called.
- **Session handle** — wraps an active `DialogueSession` plus its condition registry. Valid until
  `recite_session_free` or the session ends. The session handle carries its own compiled-asset
  reference (incrementing the `Arc` refcount), so freeing the asset handle before the session handle
  is safe.

Why not raw pointers exposed as `*mut c_void`? Handles decouple the ABI from Rust's pointer model,
allow a validity check on the Recite side before dereferencing (returning `invalid_handle_error`
instead of UB), and avoid exposing Rust's allocator address space to the host. The `u64` type is
stable across all target pointer widths.

A handle value of `0` is reserved to mean "null / no handle." Every `recite_*_new` function returns
`0` on failure.

Mapping to contract obligations:

- §2 compiled asset identity: the asset handle owns the decoded data; its lifetime is explicit and
  host-managed.
- §3 session ownership: one session handle per declared owner; `recite_session_begin` returns
  `session_already_active_error` if called more than once on the same handle.
- §16.3 (spec): single active session per owner enforced at the FFI boundary.

## Output Payload Encoding

**Decision: MessagePack length-prefixed byte buffers.**

After each session operation that drains traversal (`recite_session_start`, `recite_session_begin`,
`recite_session_choose`, `recite_session_acknowledge_effect`, and `recite_session_restore`) the
crate writes a single serialized output batch into a caller-supplied buffer slot (see Buffer
Ownership below). The batch has its own MessagePack envelope and encoder in `recite-ffi`; it is
distinct from the runtime session snapshot codec. The current batch envelope has
`batch_format_version = 0`. Condition callback payloads have no independent version field: their
shape is fixed by this ABI v0 contract and the major-version policy below. A future callback or
batch format change requires an explicitly designed compatibility mechanism (an ABI-major reset, an
additive versioned entrypoint, or a versioned envelope); there is no negotiation in the current ABI.

**Why not C structs?** Contract §5 structured output is deeply nested: choice availability reason
trees (`all` / `any` / leaf), projection affordances, deferred effect lists, inline markup.
Attempting to freeze this as a fixed-arity C struct layout would:

- couple the ABI to v0 wire shape that spec §12.2 explicitly permits to change before the first
  tagged release;
- require the host to understand Rust's struct padding rules or depend on a repr(C) layout that will
  widen with every new contract feature;
- duplicate a serialization design that already exists and is already versioned.

MessagePack has maintained host implementations; the versioned payload separates the byte ABI from
the host's typed projection. Rust uses Serde rather than a handwritten tagged-map decoder.

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

**Draining behaviour:** each session call drains traversal synchronously and returns one ordered
output batch. The batch stops at the first prompt, blocking effect, end event, or structured error.
This matches what the Godot adapter does (`adapter.rs` — it drains until a host-observable
boundary). The host does not need to call `next` in a loop; `recite-ffi` does it internally. This is
the behaviour documented per contract §4.

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
`RECITE_ERR_LOCALISATION` without changing the handle. Fuzzy and obsolete entries are ignored. Bare
gettext contexts serve both line and choice domains; prefixed availability reason and presentation
label contexts retain their distinct domains.

`recite_session_set_catalog` explicitly switches a session from callback mode to an owned catalogue
revision; installing a callback switches back. A session retains its attached revision after the
handle changes or is freed. Refreshing catalogues therefore builds a new candidate handle from all
configured PO files, then attaches it after every import succeeds. For restored sessions, attach the
catalogue after preparation and before begin so the first batch is localized.

## Session Lifecycle Functions

The generated [public C header](../include/recite.h) defines the exported functions and parameter
types. The Rust implementation and FFI contract tests own their mappings to runtime operations.

Create a session or call `recite_session_prepare_restore`, install its condition handlers,
interpolation values and locale configuration, then begin traversal. Failed preparation publishes no
handle; failed begin retains the prepared checkpoint for correction and retry. The two convenience
functions `start` and `restore` begin immediately and are suitable only when no host configuration
is needed. Failures free their prepared session and leave output slots unchanged. The grammatical
variant is not serialized and must be supplied again when restoring.

Choosing a choice or acknowledging an effect drains the next output batch. Restoring a pending
blocking effect re-emits it once; a pending prompt produces an empty resumption batch. Ended
snapshots reject restore with `NoActiveSession`.

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

For example, a callback that returns plural attempts can use session-owned or heap-owned storage. It
must release that owner only after the enclosing native call returns (including error and rollback
paths):

```c
struct LocaleOwner {
    char text[64];
    char locale[16];
    char context[64];
    char key[32];
    ReciteLocaleAttempt attempts[1];
};

static ReciteLocaleResult locale_callback(
    const ReciteLocaleQuery *query, void *userdata)
{
    struct LocaleOwner *owner = userdata; /* not callback-local storage */
    (void)query;
    /* Fill owner->... before returning and do not mutate it until the call
       that invoked Recite has returned. */
    owner->attempts[0] = (ReciteLocaleAttempt){
        owner->locale, owner->context, owner->key, 0,
        RECITE_LOCALE_ATTEMPT_MATCHED};
    return (ReciteLocaleResult){
        1, owner->text, 0, owner->locale, owner->context, owner->key,
        owner->attempts, 1, NULL};
}
```

Returning pointers to arrays, strings, or error messages allocated on the callback stack, or freeing
them when the callback returns, violates the host contract. C++ wrappers should use equivalent owner
storage and catch C++ exceptions before entering the `extern "C"` callback; Rust cannot catch a C++
exception or an `extern "C"` Rust panic.

## String and Buffer Ownership

**Rule: callee allocates output; host copies then frees.**

```c
typedef struct {
    uint8_t *data;       // heap-allocated by recite-ffi; NULL on error
    uintptr_t len;       // byte length; 0 if data is NULL
} ReciteBuffer;
```

Input strings (`start_block`, `locale`, `choice_id`, etc.) are caller-owned borrows. They are valid
only for the duration of the call. `recite-ffi` never stores a pointer to caller memory past the
function return.

Output buffers (`batch_out`, `snapshot_out`) are allocated by `recite-ffi` on its Rust allocator.
The host must call `recite_buffer_free` exactly once after consuming the data. Freeing with the
wrong allocator is UB; this must be documented prominently in the generated C header.

Unity must distribute one native `recite-ffi` library for both Mono and IL2CPP P/Invoke. Never free
a buffer through another copy, a separately recompiled backend library or a separately linked
runtime: allocation and free must reach the same library and allocator.

Binary payloads (output batches, snapshots) are length-prefixed byte buffers, not NUL-terminated C
strings. NUL bytes may appear inside msgpack data. NUL termination is used only for the host-facing
error detail string (see Error Codes).

All UTF-8. The host must not pass non-UTF-8 bytes in string inputs; `recite-ffi` validates and
returns `validation_error` if encoding is invalid.

Interpolation inputs use the same explicit typed scalar model as the canonical runtime
`InterpolationValues` map:

```c
typedef enum {
    RECITE_INTERPOLATION_VALUE_KIND_STRING = 0,
    RECITE_INTERPOLATION_VALUE_KIND_INTEGER = 1,
    RECITE_INTERPOLATION_VALUE_KIND_FLOAT = 2,
    RECITE_INTERPOLATION_VALUE_KIND_BOOLEAN = 3,
} ReciteInterpolationValueKind;

typedef struct {
    const char *name;             // UTF-8 NUL-terminated; borrowed
    uint32_t kind;               // RECITE_INTERPOLATION_VALUE_KIND_* constant
    const char *string_value;    // borrowed; used for STRING
    int64_t integer_value;       // used for INTEGER
    double float_value;          // finite; used for FLOAT
    uint8_t boolean_value;       // exactly 0 or 1; used for BOOLEAN
} ReciteInterpolationValue;
```

The records and string payloads are borrowed only for the call and copied into session-owned
storage. Duplicate names, invalid UTF-8, unknown kind constants, non-finite floats, and invalid
boolean payloads return `RECITE_ERR_VALIDATION` without changing the existing map. Passing zero
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
entrypoints. Compiled assets, snapshots and condition payloads retain their existing v0 encodings;
availability-reason origin is an optional addition to batch v0.

## Error Codes

Every `extern "C"` function returns a `ReciteStatus` (i32). Zero means success; negative values are
error categories. The stable integer assignments are:

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

These map directly to the stable machine categories in contract §12 plus two additional codes for
FFI-layer concerns:

- `RECITE_ERR_INVALID_HANDLE` — the host passed an unknown or already-freed handle. Not a
  `DialogueError` variant; detected at the FFI boundary before delegating to the runtime.
- `RECITE_ERR_DIALOGUE_FAULT` — maps to `DialogueError::TraversalLimitExceeded`, which the Godot
  adapter (`adapter_error.rs`) maps to `DialogueFault`. This indicates a dialogue authoring bug
  (e.g. an infinite divert), not an API misuse.

`RECITE_ERR_REJECTED_REFRESH` covers the `rejected_changed_asset_refresh_error` contract §12
category; it is raised by the FFI layer when a host attempts to pass a changed asset to an active
session.

Projection error codes (`-18`, `-19`, `-20`) are capability-gated: adapters that do not expose
presentation projection never emit them (contract §12).

**`DialogueError` → `ReciteStatus` mapping:**

| `DialogueError` variant            | `ReciteStatus`                                                                                                                         |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `UnknownBlock`                     | `RECITE_ERR_UNKNOWN_START_BLOCK`                                                                                                       |
| `UnsupportedCompiledFormat`        | `RECITE_ERR_STALE_OR_INCOMPATIBLE`                                                                                                     |
| `AssetMismatch`                    | `RECITE_ERR_STALE_OR_INCOMPATIBLE`                                                                                                     |
| `AssetContentMismatch`             | `RECITE_ERR_STALE_OR_INCOMPATIBLE`                                                                                                     |
| `SchemaMismatch`                   | `RECITE_ERR_SCHEMA_MISMATCH`                                                                                                           |
| `MalformedCompiledAsset`           | `RECITE_ERR_ASSET_LOAD_OR_DECODE`                                                                                                      |
| `EffectPending`                    | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT`                                                                                                    |
| `NoEffectPending`                  | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT`                                                                                                    |
| `WrongEffectAcknowledgement`       | `RECITE_ERR_EFFECT_ACKNOWLEDGEMENT`                                                                                                    |
| `PromptPending`                    | `RECITE_ERR_STALE_CHOICE`                                                                                                              |
| `NoPromptPending`                  | `RECITE_ERR_STALE_CHOICE`                                                                                                              |
| `InvalidChoice`                    | `RECITE_ERR_INVALID_CHOICE`                                                                                                            |
| `UnavailableChoice`                | `RECITE_ERR_UNAVAILABLE_CHOICE`                                                                                                        |
| `ConditionEvaluationFailed`        | `RECITE_ERR_MISSING_CONDITION_HANDLER`, `RECITE_ERR_CONDITION_EVALUATION`, or `RECITE_ERR_INVALID_CONDITION_RESULT` by structured kind |
| `ConditionResultTypeMismatch`      | `RECITE_ERR_INVALID_CONDITION_RESULT`                                                                                                  |
| `ConditionDepthLimitExceeded`      | `RECITE_ERR_CONDITION_EVALUATION`                                                                                                      |
| `UnsupportedSessionSnapshotFormat` | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`                                                                                                 |
| `SessionSnapshotEncodeFailed`      | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`                                                                                                 |
| `SessionSnapshotDecodeFailed`      | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`                                                                                                 |
| `InvalidSessionSnapshot`           | `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`                                                                                                 |
| `SessionEnded`                     | `RECITE_ERR_NO_ACTIVE_SESSION`                                                                                                         |
| `TraversalLimitExceeded`           | `RECITE_ERR_DIALOGUE_FAULT`                                                                                                            |

The `recite-adapter` `From<DialogueError> for AdapterError` implementation owns semantic categories.
FFI maps those categories to stable numeric statuses. Condition failures carry a structured runtime
kind, so callback categories do not depend on a thread-local side channel.

`recite_session_restore` applies the operation-specific override described in Save and Load Handoff:
`AssetMismatch` and `AssetContentMismatch` become `RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`, while the
typed `SchemaMismatch` remains `RECITE_ERR_SCHEMA_MISMATCH`.

Each function that returns a non-zero status also writes a NUL-terminated, UTF-8 detail string into
a thread-local that the host can retrieve with `recite_last_error_message() -> const char*`. The
pointer is valid until the next `recite-ffi` call on the same thread. The host must copy it before
calling further functions.

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
    const uint8_t *value_msgpack;// host-owned; valid after callback return (see below)
    uintptr_t value_len;         // valid when ok = 1
    const char *error_message;   // same result lifetime when ok = 0
} ReciteConditionResult;

typedef ReciteConditionResult (*ReciteConditionFn)(
    const ReciteConditionQuery *query,
    void *userdata
);
```

The host registers one function pointer per condition name before starting the session. During
traversal, `recite-ffi` invokes the matching handler synchronously — the call is inline with
`next_with` traversal, exactly as in the Godot adapter (`adapter.rs` — `BTreeMap<String,
Box<ConditionHandler>>` with `Fn(ConditionCall<'_>) -> ConditionHandlerResult`).

The function pointer must be non-null, synchronous, and non-panicking. Host wrappers must enforce
the no-panic/no-throw/no-unwind contract and return `ok = 0` for an evaluation failure; they must
not unwind across `extern "C"`, re-enter Recite, or retain borrowed query/result pointers. A Rust
panic in an `extern "C"` callback aborts before Recite can catch it, and a C++ exception must be
caught by the host wrapper before entering Recite.

**Why callbacks, not pre-resolved query batches?** The alternative (pause traversal, return the
pending condition set to the host, wait for the host to re-enter with answers) is a two-round-trip
protocol. It requires the host to maintain explicit "condition query pending" state between calls
and makes the traversal loop stateful from the host's perspective. For Unity (Mono/IL2CPP),
single-threaded condition evaluation from a P/Invoke call site is simpler than a polling loop. The
callback approach is also what the Godot MVP proved works under a Rust-foreign-language boundary
(Godot conditions are GDScript `Callable`s invoked through the gdext callback path).

**Threading constraint:** condition callbacks are invoked on the same thread that called the
`recite-ffi` traversal function. They must not call back into `recite-ffi` (no reentrancy). Hosts
that evaluate conditions on a different thread must marshal via `userdata` and synchronize
themselves. This is the same single-threaded evaluation model the Godot adapter uses.

The three condition error categories from contract §6 map through `ReciteConditionResult.ok = 0`:

- Handler not registered → `RECITE_ERR_MISSING_CONDITION_HANDLER` (detected in `recite-ffi` before
  invoking the callback, just as the Godot adapter checks its `BTreeMap`).
- Handler returns `ok = 0` with a message → `RECITE_ERR_CONDITION_EVALUATION`.
- Handler returns a msgpack value whose type mismatches the schema declaration →
  `RECITE_ERR_INVALID_CONDITION_RESULT` (detected by the runtime during `ConditionValue` type
  validation, as `ConditionResultTypeMismatch`).

The condition result value is a msgpack-encoded `ConditionValue` (bool or enum variant string).
Arguments are a msgpack-encoded list of `ConditionArgument` values. The host-side msgpack
representation must match the schema-declared parameter types; mismatches produce
`RECITE_ERR_INVALID_CONDITION_RESULT`.

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
`RECITE_ERR_CONDITION_EVALUATION` (a null error pointer uses a stable fallback), and `1` requires a
non-null, non-empty, complete result map. Scalars, maps with missing, duplicate, or unknown keys,
wrong field types, truncated payloads, and trailing bytes are rejected as
`RECITE_ERR_INVALID_CONDITION_RESULT`.

The native query bytes and function-name pointer are Rust-owned borrows valid only during the
synchronous callback. Host result bytes and error strings must remain immutable and valid after
callback return until the next condition callback for that session or the enclosing Recite operation
returns, whichever happens first. Recite decodes the result before invoking the next callback, so a
session-owned reusable buffer is sufficient. Callbacks must not re-enter `recite-ffi`.

## Threading and Reentrancy

A session handle is not thread-safe. The host must not call `recite-ffi` functions on the same
session handle from multiple threads concurrently. This mirrors the Rust `!Sync` nature of
`DialogueSession`.

An asset handle is safe to share across threads for reading (backed by `Arc<CompiledDialogue>`), but
`recite_asset_free` must not race with any session that holds a reference to the same asset.

`recite-ffi` functions are not reentrant. A condition callback must not call any `recite-ffi`
function.

The host's `userdata` pointer is passed back to condition callbacks as-is. `recite-ffi` does not
dereference or hold it. The host must ensure it remains valid and accessible on the calling thread
for the duration of the traversal call.

## Save and Load Handoff

`recite_session_snapshot` encodes the complete runtime session state as a length-prefixed msgpack
byte buffer (via `snapshot_session` + `encode_session_messagepack`). The host treats this as opaque:
stores it in its game save data, reads it back, and passes it to `recite_session_restore` later.

`recite_session_restore` reconstructs the session by validating the snapshot against the supplied
asset handle (via `decode_session_messagepack` + `restore_session`). A schema-fingerprint difference
returns `RECITE_ERR_SCHEMA_MISMATCH`; schema comparison is performed first, so a snapshot that
differs in both schema and another identity/content field still returns
`RECITE_ERR_SCHEMA_MISMATCH`. All other asset identity/content differences during restore return
`RECITE_ERR_SAVE_LOAD_INCOMPATIBILITY`. This operation-specific mapping keeps schema drift
actionable without changing ordinary runtime stale-asset handling. The call still enforces the
contract §9 requirement that session state is tied to a specific compiled asset.

The host must not deserialize, modify, or re-serialize the snapshot bytes. Doing so silently breaks
deterministic resume (contract §9 — the snapshot includes trace counters, the divert stack, pending
blocking effects, and other determinism-critical state).

If a blocking effect was pending when the snapshot was taken, restoring the session re-emits that
effect once in the resumption batch with the same request ID, and leaves it pending until the host
acknowledges it. The stable ID lets the host reconcile, replay, fast-forward, or treat the effect as
complete; the runtime does not know whether the game-side operation happened before the save
(contract §9).

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

Specifically: a generated C# `ReciteDialogueService` would P/Invoke into `recite_session_start`,
`recite_session_choose`, etc., and decode the msgpack output batch into typed C# structs. The C ABI
does not need to know about those typed structs; they are a generation-time concern.

## Delivery ownership

[Engine companion delivery](https://github.com/plethu/recite/milestone/23) owns platform and package
acceptance. Completed ABI implementation issue routing is
[historical evidence](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#c-abi-delivery).

`pkg-config` or CMake find-module support remains outside v1 scope unless a downstream package needs
it.

## Open Items

- **`validation_error` category coverage:** the contract §12 category `validation_error` has no
  direct `DialogueError` variant (it is raised by host-level checks such as invalid UTF-8 input,
  malformed handle, or unsupported batch format version). The mapping table above covers all current
  `DialogueError` variants; if future variants add a `Validation` case, the table and the
  `RECITE_ERR_VALIDATION` code are already in place.
