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

## Handle Model

Handles are opaque `u64` identifiers produced and consumed through `recite-ffi`. Hosts must not
dereference, persist or interpret their bits.

There are asset, session and catalogue handles:

- **Asset handle** — owns a decoded asset until `recite_asset_free`.
- **Session handle** — owns a session and condition registry until `recite_session_free`, including
  after traversal ends. It retains the asset independently, so freeing the asset handle is safe.
- **Catalogue handle** — owns a validated catalogue revision. Attaching it captures that revision;
  later mutation or disposal of the handle does not change the session's provider.

Zero is the null handle. Unknown or freed handles are rejected, but raw caller pointers still
require valid memory. A session handle enforces its own lifecycle; the host adapter owns the rule
that a scene object or service has at most one active session. FFI handles do not identify host
owners.

## Output Payload Encoding

Payloads are MessagePack values carried in owned buffers with an explicit byte length. There is no
additional length prefix inside the bytes.

Each traversal operation (`start`, `begin`, `choose`, `acknowledge_effect`, `restore`) writes one
ordered batch to the caller's output slot, stopping at a prompt, blocking effect, ending or error.
The [batch encoder](../crates/recite-ffi/src/output/encode.rs) is distinct from the snapshot codec.
Its envelope contains `batch_format_version = 0` (u16); adapters must reject unknown versions as
`validation_error`. Transactional draining follows the adapter contract; hosts need no separate
`next` loop.

Condition payloads have no independent version field. Future callback or batch changes require an
explicit compatibility mechanism under
[serialization compatibility](serialization-compatibility.md), such as an ABI-major reset, versioned
entrypoint or envelope. The current ABI has no negotiation.

The Unity byte-codec probe with MessagePack-CSharp 3.1.11 preserved managed adapter conformance and
reduced allocation. Adoption is deferred under the current self-contained UPM distribution: bundled
assemblies conflict with a consumer's existing MessagePack installation, while upstream uses a
[shared NuGet installation](https://github.com/MessagePack-CSharp/MessagePack-CSharp#unity-support).
Reevaluate when Unity distribution chooses one shared dependency owner. Either implementation must
retain Recite's typed projection and strict malformed-input checks.

Availability reasons optionally carry `origin`: either `condition_call` with `function` and tagged
`args`, or `requirement_expression` with `source_text`. This is an additive batch-v0 field; its
absence means provenance is unavailable. Hosts preserve both origins and reject unknown kinds.

## Asset Metadata

`recite_asset_info` returns a named MessagePack map for importer identity checks, with
`asset_info_format_version = 0`. The map carries `asset_id`, `content_fingerprint` (`algorithm`
string and binary `digest`), nullable `schema_fingerprint` of the same shape, numeric
`format_version` and `compiler_compatibility_version`, `compiler_version`, and `source_map_id`.
Metadata inspection does not replace runtime compatibility validation. The caller frees the returned
buffer with `recite_buffer_free`.

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

`ReciteLocaleFn` receives a borrowed `ReciteLocaleQuery` and returns a translated template in
`ReciteLocaleResult`, or `text = NULL` for authored source fallback. A null session locale bypasses
the callback.

The host owns the entire result pointer tree: text, error message, matched locale/context/key,
attempts array and each attempt's strings. Keep it immutable and valid until the enclosing Recite
API call returns, when Recite has finished copying it. This applies to every call that traverses;
there is no callback-level release point. Stack or callback-local temporaries are forbidden. The
callback and `userdata` must remain valid until the session is freed or the provider cleared. The
common restrictions in [Threading and Reentrancy](#threading-and-reentrancy) also apply.

For plural queries, enumerate the requested variant context (`context&variant`) across the locale's
most-specific-to-base chain, then the base context across that chain. Missing plural rules, missing
entries, empty translations and fuzzy translations continue to the next candidate; represent empty
or fuzzy records as `RECITE_LOCALE_ATTEMPT_MISSING_TRANSLATION`. Conflicts are not matches.
`RECITE_LOCALE_ATTEMPT_MATCHED` ends the sequence; its arm and provenance must come from that
candidate's validated rule and matching entry. If none matches, return `text = NULL`,
`selected_arm = -1` and null match provenance for English source fallback. Recite copies and reports
the supplied attempts but cannot enforce a custom callback's actual lookup order.

## String and Buffer Ownership

Recite allocates output; the host copies it and frees it through the same library. Failed calls
leave caller output slots unchanged. Initialize those slots deliberately; a failure does not promise
a null pointer or zero length.

Input strings (`start_block`, `locale`, `choice_id`, etc.) are borrowed for the call; Recite retains
no pointer after return.

Call `recite_buffer_free` exactly once for each output buffer. Another allocator causes undefined
behavior; the generated header must state this prominently.

Unity must distribute one native `recite-ffi` library for both Mono and IL2CPP P/Invoke. Never free
a buffer through another copy, a separately recompiled backend library or a separately linked
runtime: allocation and free must reach the same library and allocator.

Binary payloads use a pointer and byte length and may contain NUL bytes. Error detail strings are
NUL-terminated UTF-8. Invalid UTF-8 string inputs return `RECITE_STATUS_VALIDATION`.

Interpolation inputs use the runtime typed scalar model: string, integer, finite float or boolean.
The generated header owns the records and kind constants.

The records and string payloads are borrowed only for the call and copied into session-owned
storage. Duplicate names, invalid UTF-8, unknown kind constants, non-finite floats, and invalid
boolean payloads return `RECITE_STATUS_VALIDATION` without changing the existing map. Passing zero
records clears the map. Interpolation values are deliberately not part of the opaque session
snapshot; hosts must provide them again on restore when the resumption drain needs them.

## Generated C Header

Consume the generated header linked above rather than hand-maintaining declarations. It is generated
from `crates/recite-ffi` with `cbindgen.toml`.

Run `scripts/generate-ffi-header.sh --write` after changing the FFI surface. The project gate runs
`scripts/generate-ffi-header.sh` without `--write`, which fails if the committed header is stale.

Header version constants (`RECITE_FFI_VERSION_MAJOR`, `RECITE_FFI_VERSION_MINOR`, and
`RECITE_FFI_VERSION_PATCH`) match the `recite-ffi` crate version. The ABI is currently unreleased
and unstable: 0.x minor revisions may change the interface, and hosts must ship matching headers,
bindings and native libraries. After stabilization, incompatible changes require a major bump; patch
versions remain for documentation and implementation changes.

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

The callback pointer must be non-null. Do not retain borrowed query pointers. The common callback
restrictions in [Threading and Reentrancy](#threading-and-reentrancy) apply.

Condition failures map to these statuses:

- Missing handler: `RECITE_STATUS_MISSING_CONDITION_HANDLER`.
- Evaluation failure (`ok = 0`): `RECITE_STATUS_CONDITION_EVALUATION`.
- Malformed result or schema type mismatch: `RECITE_STATUS_INVALID_CONDITION_RESULT`.

Argument and result encoding follows the MessagePack contract below and the schema-declared types.

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
session-owned reusable buffer is sufficient.

## Threading and Reentrancy

Hosts serialize operations on a session and retain its condition callbacks and `userdata` for the
session's lifetime. Locale callback lifetime is specified above. Asset handles may be shared for
reads; do not race disposal with an operation using that handle.

Condition and locale callbacks must not re-enter Recite. Same-thread session-registry re-entry
returns `RECITE_STATUS_VALIDATION` before locking; a reentrant void free records the error and
leaves the handle intact. This protection cannot make foreign pointers valid, catch foreign
exceptions or prevent a callback waiting on another thread that needs Recite's held registry lock.
Callbacks must return synchronously without throwing, panicking or unwinding across the ABI. Rust
panics in `extern "C"` callbacks abort before Recite can catch them; Rust cannot catch C++
exceptions. C++ hosts should use an `extern "C"` wrapper that catches exceptions and returns `ok =
0`. The Unity wrapper does the same for managed exceptions.

## Save and Load Handoff

`recite_session_snapshot` returns complete runtime state as opaque MessagePack bytes in an owned
buffer. Store those bytes with the game save. Restore validates them against the supplied asset:
schema-fingerprint differences return `RECITE_STATUS_SCHEMA_MISMATCH`, taking precedence over other
identity/content differences, which return `RECITE_STATUS_SAVE_LOAD_INCOMPATIBILITY`.

Use the prepared lifecycle above to restore host conditions, interpolation values and catalogue
configuration before traversal begins. Pending-effect reconciliation remains the game's
responsibility under the
[adapter save/load contract](engine-adapter-contract.md#9-save-and-load-handoff).

## Schema Manifest and Projection

Schema production and presentation projection are outside the v1 FFI surface. Host build/editor
tooling writes the shared JSON schema manifest for the compiler and LSP.

Projection is capability-gated and deferred at this boundary; Unity may conform without it. An
adapter that adds projection must document its query protocol through an agreed extension to this
design, not a private FFI shape. See the [adapter contract](engine-adapter-contract.md).

## Relationship to `generate-bindings` (spec §13.9)

Post-v1 schema-generated host wrappers would call this ABI; they would not implement a second
runtime. See [§13.9](spec/build-cli.md#139-future-generate-bindings).
