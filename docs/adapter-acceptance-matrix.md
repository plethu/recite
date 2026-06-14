# Recite v1 Adapter Acceptance Matrix

This matrix defines the v1 acceptance bar for the Godot, Bevy, and Unity
adapters. It refines the host-agnostic requirements in
`docs/engine-adapter-contract.md` without changing them.

The matrix is a planning and review checklist. The contract and conformance
fixtures remain normative for machine-checkable semantics; this document states
what each v1 engine adapter must document, expose, and test before it is treated
as production-quality.

## Applicability

| Field | Requirement |
| --- | --- |
| Engines | Godot, Bevy, Unity |
| Post-v1 engines | Unreal and GameMaker stay out of v1 scope |
| Shared semantic boundary | `docs/engine-adapter-contract.md` |
| Conformance artifact boundary | `fixtures/adapter-conformance/v1/` |
| Required changed-asset policy vocabulary | `reject_refresh_until_session_ends`, `reload_for_next_session_only`, `restart_required` |
| Required session scope | At least one active dialogue session per declared adapter owner |
| Required output style | Structured host-visible values, not formatted prose |

## Engine Surface Baseline

| ID | Capability | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-SURF-01 | Package shape | Godot 4 GDExtension crate or addon package with documented import/setup steps. | Rust crate integrated with Bevy apps through plugins, resources, events, and assets. | Unity Package Manager package with runtime-safe C# code and editor code isolated from player builds. |
| ADP-SURF-02 | Runtime owner | Node, autoload service, or resource manager with documented single-session ownership. | Resource, component, or dedicated session resource with documented single-session ownership per owner. | GameObject service/runner, C# service, or DOTS facade backed by one shared adapter core. |
| ADP-SURF-03 | Output delivery | Signals, typed C# events, GDScript-callable values, or pull-style methods. | Events/messages/resources/systems using Bevy idioms. | C# events, UnityEvent hooks where useful, service responses, or DOTS events/buffers. |
| ADP-SURF-04 | Schema producer fit | Godot resources, project settings, C#/GDScript registration, or editor plugin export. | Rust builders, derives, resources, asset collections, or editor-side export commands. | ScriptableObjects, importers, GUID-addressed assets, C# attributes/builders, Addressables, or editor tooling. |
| ADP-SURF-05 | Host independence | Adapter code may discover host inputs only inside producer/import tooling; compiler, CLI, LSP, and runtime validation must consume generated Recite artifacts. | Same. | Same. |
| ADP-SURF-06 | Release distribution | Before 1.0, provide a Godot Asset Library/addon-ready bundle with setup docs and examples. | Before 1.0, publish an ecosystem-native crate and Bevy plugin/example bundle that does not require copying repo internals. | Before 1.0, provide a Unity Asset Store or Unity Package Manager-friendly package with runtime/editor separation, examples, native library packaging, and upgrade notes. |

## Asset Loading and Authoring Refresh

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-ASSET-01 | Compiled asset loading | Load `.recitec` data through ResourceLoader/imported resources where practical. | Load compiled assets through Bevy's asset system where practical, with headless fallback for tests. | Load compiled assets as TextAsset, ScriptableObject/imported asset, or equivalent Unity asset preserving bytes and identity. |
| ADP-ASSET-02 | Compatibility identity | Preserve compiled asset compatibility identity for start/resume checks. | Same. | Same, including across the native C ABI boundary when used. |
| ADP-ASSET-03 | Freshness | Expose source/schema freshness checks when the adapter can see source and schema inputs; do not treat compatibility identity as freshness. | Same. | Same. |
| ADP-ASSET-04 | Authoring loop | Document edit source -> LSP diagnostics -> save IDs -> `recite watch` rebuild -> engine import -> restart session. | Same, adapted to Bevy asset reload workflows. | Same, adapted to Unity import/reimport workflows. |
| ADP-ASSET-05 | Rejected import/refresh | Surface rejected refresh attempts as structured adapter errors or editor-visible diagnostics. | Same. | Same. |
| ADP-ASSET-06 | Import failure | Malformed, incompatible, or stale compiled assets must not start sessions as if valid. | Same. | Same. |

## Active-Session Changed-Asset Policy

Each adapter must choose exactly one policy for v1, document it, and test it.
The three engines may choose different policies, but the chosen policy must be
declared through conformance output.

| ID | Policy | Required behavior |
| --- | --- | --- |
| ADP-CHANGE-01 | `reject_refresh_until_session_ends` | The adapter rejects import or refresh while the owner has an active session. The active session keeps running on its original asset. |
| ADP-CHANGE-02 | `reload_for_next_session_only` | The adapter accepts the new compiled asset into the host cache. The active session keeps its original compiled asset identity. The next session uses the new asset. |
| ADP-CHANGE-03 | `restart_required` | The adapter reports that the active session must be ended and restarted before the new asset can be used. |
| ADP-CHANGE-04 | Forbidden behavior | No v1 adapter may silently swap the compiled asset underneath an active session. |

## Runtime Operations

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-RUN-01 | Start | Start from a compiled asset, optional start block, and explicit/project-configured locale or source fallback. | Same. | Same. |
| ADP-RUN-02 | Select | Select by stable `ChoiceId`; index selection may exist only as a convenience that lowers to the current prompt's ID. | Same. | Same. |
| ADP-RUN-03 | Acknowledge blocking effects | Acknowledge by exact `EffectRequestId` and expose both completed and failed acknowledgements. | Same. | Same. |
| ADP-RUN-04 | End/dispose | Provide an explicit host-visible end or dispose operation. | Same. | Same. |
| ADP-RUN-05 | Advance shape | Document whether host code calls `next` directly or the adapter drains synchronously to the next boundary. | Same. | Same. |
| ADP-RUN-06 | Output ordering | Preserve runtime output order and stop drained batches at prompt, blocking effect, end, or structured error. | Same. | Same. |
| ADP-RUN-07 | Second active session | Starting a second session on the same owner emits a structured error; it must not panic, overwrite, or implicitly end. | Same. | Same. |
| ADP-RUN-08 | Invalid operations | No active session, stale choice, unavailable choice, invalid choice, and wrong effect acknowledgement produce stable structured error categories. | Same. | Same. |

## Conditions and Effects

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-CE-01 | Condition registration | Register pure condition handlers through C#/GDScript or Godot-native extension points. | Register pure typed Rust handlers or system-accessible query callbacks. | Register pure C# handlers; DOTS facade must share the same semantic core. |
| ADP-CE-02 | Condition failure categories | Missing handler, evaluation failure, and invalid result type must map to stable structured errors. | Same. | Same. |
| ADP-CE-03 | Condition purity | Handlers must not mutate game state, advance time, emit effects, or depend on nondeterministic ordering. | Same. | Same. |
| ADP-CE-04 | Effect emission | Emit deferred, immediate, and blocking effects as typed structured host-visible requests. | Same. | Same. |
| ADP-CE-05 | Effect non-execution | The runtime and adapter core must not execute game-side mutation. Host game code owns execution and acknowledgement. | Same. | Same. |
| ADP-CE-06 | Typed helpers | Generated or hand-written typed wrappers may exist, but they must preserve the original structured effect request. | Same. | Same. |

## Save and Load

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-SAVE-01 | Snapshot extraction | Expose the runtime session snapshot as one opaque value suitable for host save data. | Same. | Same. |
| ADP-SAVE-02 | Snapshot restore | Restore the complete runtime snapshot against a compatible compiled asset. | Same. | Same. |
| ADP-SAVE-03 | Game-state boundary | Do not serialize arbitrary game state into the Recite session snapshot. | Same. | Same. |
| ADP-SAVE-04 | Pending prompt | Save/load while waiting at a prompt must preserve previous prompt choices and selected-choice history. | Same. | Same. |
| ADP-SAVE-05 | Pending blocking effect | Save/load while waiting on a blocking effect must preserve the pending `EffectRequestId`. | Same. | Same. |
| ADP-SAVE-06 | Incompatibility | Snapshot/asset mismatch must surface `save_load_incompatibility_error` or the contract-defined compatible category. | Same. | Same. |

## Localisation

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-LOC-01 | Locale selection | Start with an explicit locale, project-configured locale, or source-text fallback. Do not silently derive locale from host environment. | Same. | Same. |
| ADP-LOC-02 | Text handoff | Preserve localized text, source text where useful, line IDs, choice IDs, metadata, and markup in structured output. | Same. | Same. |
| ADP-LOC-03 | Variant axis | Expose grammatical variant selection as explicit caller-driven state or operation input. | Same. | Same. |
| ADP-LOC-04 | Catalog failure | Malformed or unavailable catalogs must surface structured loading/localisation errors or diagnostics. | Same. | Same. |
| ADP-LOC-05 | Active locale changes | Changing locale mid-session is not required for v1 unless the adapter documents and tests the exact behavior. | Same. | Same. |

## Error Handling and Host Lifecycle

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-ERR-01 | Stable categories | Surface all stable adapter error categories from `docs/engine-adapter-contract.md` section 12, with projection categories gated by projection capability. | Same. | Same. |
| ADP-ERR-02 | Host context | Include host asset paths, resource identifiers, entity/component context, or Unity asset references where available without making callers parse prose. | Same, using Bevy asset/entity context where useful. | Same, using Unity asset paths, GUIDs, or object names where useful. |
| ADP-ERR-03 | Lifecycle cleanup | Free or drop active sessions, callbacks, native buffers, and host subscriptions deterministically when the owner exits. | Same. | Same, including native plugin buffers and pinned callback state. |
| ADP-ERR-04 | Headless testability | Provide a way to exercise host-independent semantics in automated tests without a rendered game scene. | Same. | Same. |
| ADP-ERR-05 | No prose contract | Public pass/fail, error, and conformance results must be machine-readable; display text is diagnostic only. | Same. | Same. |

## Minimum Example Requirements

Each v1 adapter must satisfy all rows in this section. The rows may be covered
by one small sample, a second focused sample, documentation, or automated tests;
they do not require five separate example projects. More examples are welcome,
but the minimum evidence must stay small enough to review.

| ID | Requirement | Godot | Bevy | Unity |
| --- | --- | --- | --- | --- |
| ADP-EX-01 | Basic dialogue | Scene demonstrating asset load, start, line output, prompt output, choice selection, and end. | App or example demonstrating the same through Bevy systems/events. | Scene or sample package demonstrating the same through C# events or UnityEvents. |
| ADP-EX-02 | Conditions and effects | Example showing a pure condition handler and immediate/blocking/deferred effect emission without runtime-side mutation. | Same. | Same. |
| ADP-EX-03 | Save/load | Example or test-facing sample showing snapshot extraction and restore, including prompt or blocking-effect state. | Same. | Same. |
| ADP-EX-04 | Authoring refresh | Example docs showing how source edits flow through `recite watch` and the engine import/refresh path. | Same. | Same. |
| ADP-EX-05 | Error surface | Example or documented test showing at least one structured adapter error exposed in host-native form. | Same. | Same. |

## Conformance and Performance Exit Criteria

| ID | Requirement |
| --- | --- |
| ADP-CONF-01 | The adapter must pass host-independent conformance scenarios for loading, start/select/ack/end, conditions, effects, save/load, localisation, and all mandatory stable error categories. |
| ADP-CONF-02 | The adapter must declare its changed-asset policy and pass the matching active-session refresh/import scenario. |
| ADP-CONF-03 | If the adapter exposes projection, it must pass projection conformance scenarios for ordering, handler dispatch, failure categories, stable affordance IDs, label template data, localized text, and structured fields. |
| ADP-CONF-04 | Editor/import UX that cannot reasonably be automated may be manually checked, but the host-independent semantic trace must remain automated. |
| ADP-PERF-01 | The adapter must report asset loading/conversion overhead, event emission overhead, active-session update overhead, condition dispatch overhead, and typed effect conversion overhead where applicable. |
| ADP-PERF-02 | The adapter should add negligible frame or tick cost when no dialogue session is active. |

## Review Use

Adapter PRs should cite the row IDs they satisfy. A row can be considered met
only when the adapter documents the behavior, exposes it through a host-native
surface, and has automated coverage unless this matrix explicitly allows a
manual editor/import check.
