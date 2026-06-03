# Unity Adapter Design

This document records the initial Unity adapter and schema-export strategy for
#81. It is a design input for follow-up implementation issues, not a committed
v1 public C# API or Rust API change.

Normative behavior remains in `docs/recite-production-spec.md` and
`docs/engine-adapter-contract.md`. This design applies those contracts to Unity
without adding a Unity-only dialogue runtime, schema format, or validation
surface.

## Goals

- Make Recite feel native to C# Unity projects.
- Support both GameObject/OO and DOTS-facing workflows.
- Keep Recite traversal, save/load, localisation, asset identity, errors, and
  changed-asset behavior shared across Unity facades.
- Export the canonical generated schema manifest from Unity-side declarations
  without requiring the Recite compiler, CLI, or LSP to execute Unity code.
- Keep the edit/save/`recite watch`/Unity import/restart loop explicit and
  deterministic.

## Non-Goals

- Implementing the Unity adapter MVP.
- Implementing Unity editor refresh behavior.
- Defining stable v1 C# APIs.
- Adding generated C# bindings or a generated C# runtime fork.
- Changing the Rust runtime, compiler, schema model, or manifest format.

## Package Shape

The Unity adapter should ship as a Unity Package Manager package, for example
`com.recite.dialogue`. The package should split runtime and editor surfaces so
player builds do not depend on UnityEditor APIs.

Recommended package layout:

```text
Packages/com.recite.dialogue/
  Runtime/
    ReciteDialogueAsset.cs
    ReciteDialogueService.cs
    ReciteSessionSnapshot.cs
    ReciteOutput.cs
    ReciteAdapterError.cs
    Native/
      ReciteNativeBridge.cs
    GameObjects/
      ReciteDialogueRunner.cs
    Dots/
      ReciteDialogueComponents.cs
      ReciteDialogueSystems.cs
  Editor/
    ReciteSchemaExporter.cs
    ReciteDialogueImporter.cs
    ReciteSettingsProvider.cs
    ReciteAssetPostprocessor.cs
  Tests/
    EditMode/
    PlayMode/
```

The exact file names can change during implementation. The important boundary
is that `Runtime/` owns player-safe adapter behavior, `Editor/` owns schema
export and import tooling, and GameObject/OO plus DOTS facades both call into
the same runtime adapter core.

## Runtime Boundary

The first Unity adapter should use a managed C# wrapper around a Rust/native
Recite runtime bridge. Unity should not fork traversal into generated C# logic.
Generated C# bindings may become useful later for typed conditions, effects, or
schema records, but they must remain wrappers around canonical compiled assets
and runtime semantics.

The native boundary should expose operations equivalent to the adapter
contract:

- load or decode a compiled asset and report structured load errors;
- start a session from a compiled asset identity, optional block, and locale;
- select a choice by stable `ChoiceId`;
- acknowledge a blocking effect by stable `EffectRequestId`;
- snapshot and restore the opaque Recite session state;
- emit ordered structured output batches and structured adapter errors.

`ReciteDialogueAsset` should be a Unity-native representation of compiled
Recite data. It may be a `ScriptableObject`, importer-produced asset, or another
asset type selected during implementation, but it must preserve compiled asset
identity, schema fingerprint, compiler compatibility data, and source/schema
freshness metadata when available.

## Shared Adapter Core

The Unity package should have one shared adapter core used by both facades. It
owns:

- compiled asset identity and freshness checks;
- one-active-session enforcement per declared owner;
- session start/select/acknowledge/end sequencing;
- opaque runtime snapshot handoff for save/load;
- locale and grammatical variant inputs;
- condition-handler dispatch;
- structured effect request emission;
- structured output and error conversion;
- the changed-asset policy.

Facade code may differ in how Unity users interact with it, but not in the
semantics above. If the GameObject facade accepts `UnityEvent` callbacks and
the DOTS facade writes output buffers, those surfaces still receive the same
ordered output categories and machine-readable error categories.

## GameObject and OO Facade

The GameObject/OO path should target ordinary Unity C# teams and authoring
scenes. A likely MVP shape is:

- `ReciteDialogueAsset`: imported compiled Recite asset reference.
- `ReciteDialogueService`: plain C# service with `Start`, `SelectChoice`,
  `AcknowledgeEffect`, `Snapshot`, and `Restore` methods.
- `ReciteDialogueRunner`: `MonoBehaviour` wrapper for scene wiring.
- C# events for structured output and errors.
- Optional `UnityEvent` hooks for users who prefer inspector wiring, with the
  structured C# value still available.

The service should be the core owner. The `MonoBehaviour` should be convenience
glue, not the semantic implementation. Starting a second active session on the
same service or runner must return or emit a structured one-active-session
error.

## DOTS Facade

The DOTS path should target Entities users without changing Recite semantics.
The design should preserve a staged implementation path: DOTS can follow the
GameObject MVP, but the first design must not preclude it.

A likely DOTS shape is:

- baked references from authoring components to `ReciteDialogueAsset` identity;
- a singleton or explicitly keyed session owner component for v1;
- request components or buffers for start/select/acknowledge operations;
- output buffers or event entities for lines, prompts, effects, end events, and
  errors;
- systems that translate ECS requests into calls on the shared adapter core.

DOTS may optimize memory layout or event delivery, but it must not reimplement
traversal, alter choice IDs, silently reload active assets, reinterpret
localisation, or serialize a hand-picked subset of Recite session state.

## Schema Export Strategy

Unity schema export is editor-only. It may gather schema declarations and
resource-backed snapshots from Unity-native sources such as:

- C# attributes on condition, effect, enum, registry, and projection types;
- explicit C# builder registration in an editor export entry point;
- `ScriptableObject` registries and settings assets;
- asset importers and asset labels;
- GUID-addressed assets;
- Addressables groups;
- project settings.

All producer inputs lower into the canonical generated Recite schema manifest
from spec §10.2 and adapter contract §7. The generated manifest is the only
schema surface consumed by `recite compile`, `recite validate`, `recite watch`,
and `recite-lsp`.

The exporter should provide two explicit editor commands:

- export schema manifest: scan Unity-side declarations and write the generated
  manifest to the configured project path;
- check schema freshness: compare the existing manifest against Unity-side
  producer fingerprints and report stale or malformed producer state.

Unity may also call those commands from CI or editor automation, but validation
inside Recite tooling must still work from the manifest alone. Compiler and LSP
code must never load a Unity project, query Unity's asset database, reflect over
Unity assemblies, or execute Unity game/editor code.

The exporter should snapshot stable symbols and fingerprints. For Unity assets,
stable Recite symbols should be explicit project IDs where possible; Unity GUIDs
may be provenance or fallback identity, but dialogue-facing IDs should not be
opaque editor implementation details unless the project deliberately chooses
that convention.

## Authoring Loop

The expected Unity authoring loop is:

1. Edit Unity-side schema declarations or dialogue source.
2. Run the Unity schema export command when schema declarations or Unity-backed
   registries change.
3. Let `recite validate` or the LSP validate dialogue against the generated
   manifest.
4. Let `recite watch <project-root>` rebuild compiled assets when dialogue,
   project, or schema inputs change.
5. Let Unity import or reimport rebuilt compiled assets through the adapter's
   importer or explicit refresh command.
6. Restart the dialogue session or follow the adapter's active-session policy.

The Unity editor may improve this with menu items, project settings, import
hooks, and status UI, but the core loop should remain explicit. `recite watch`
does not imply mid-session patch reload, and Unity import does not authorize
silent mutation of an active runtime session.

## Active-Session Policy

The Unity adapter should default to `reload_for_next_session_only`.

When a compiled asset changes during play mode:

- Unity may import the new compiled asset into its asset cache.
- Any active session continues using the compiled asset identity it started
  with.
- Save/load snapshots continue to compare against the original active asset
  identity.
- The next session started from the asset uses the newly imported identity.
- The adapter surfaces a structured notification or status that the active
  session is using an older loaded identity when Unity can report that cleanly.

This policy supports a fast editor import loop without breaking deterministic
traversal, previous-prompt choice validation, blocking-effect acknowledgement,
or save/load identity. Mid-session migration is explicitly out of scope for v1.

## Conditions and Effects

Condition handlers are pure C# queries registered through the Unity adapter.
They may read caller-provided game state, but must not mutate game state,
advance time, emit effects, or depend on nondeterministic ordering. Missing
handlers, evaluation failures, and result-type mismatches must surface as
structured adapter errors matching the shared contract categories.

Effects are typed requests emitted to Unity host code. The adapter must not
execute game-side mutation inside the Recite runtime or native bridge.

The facade may expose:

- generic structured effect request events for all effects;
- optional typed C# wrappers generated or hand-written from schema;
- `UnityEvent` convenience hooks for inspector-driven projects;
- DOTS event entities or buffers for ECS systems.

All wrappers must preserve the original effect request ID, function name, mode,
arguments, and acknowledgement requirement. Blocking effects must pause until
the host acknowledges the exact pending `EffectRequestId`.

## Localisation and Save/Load

Unity sessions should start with an explicit locale, a project-configured
locale, or source-text fallback. The adapter must not silently derive dialogue
locale or grammatical variant from the operating system, editor language, or
Unity player settings.

Save/load should treat the Recite runtime session snapshot as opaque. Unity save
systems may store that snapshot beside game save data, but must not serialize
Unity game state into the Recite snapshot or rebuild the snapshot from selected
fields. Restoring a session with a pending blocking effect must preserve the
same pending effect ID required for acknowledgement.

## Import and Asset Identity

The Unity importer should preserve both compatibility identity and authoring
freshness metadata:

- compatibility identity for session start and save/load resume;
- source/schema freshness information for editor diagnostics and import status;
- schema fingerprint and compiler compatibility version;
- asset provenance useful for user-facing diagnostics.

The importer should reject malformed compiled assets with structured adapter or
import errors. Stale assets should not start a session as if current when the
adapter has enough source/schema visibility to check freshness.

## Test Strategy

The follow-up implementation should include Unity EditMode and PlayMode tests,
plus host-independent adapter conformance coverage where possible.

EditMode tests should cover:

- schema export from attributes, builder registration, and `ScriptableObject`
  or asset-backed registries without running Recite compiler/LSP in Unity;
- deterministic manifest ordering and stable fingerprints;
- stale schema manifest reporting;
- compiled asset import, decode errors, and identity/freshness metadata;
- `reload_for_next_session_only` import behavior while a session is active.

PlayMode tests should cover:

- GameObject/OO start/select/acknowledge flow;
- DOTS start/select/acknowledge flow once the DOTS facade exists;
- one-active-session rejection;
- pure condition registration and missing/evaluation/result-type errors;
- immediate, blocking, and deferred effect requests;
- blocking-effect save/load with the same pending effect ID;
- locale fallback behavior;
- save/load handoff with pending prompts and pending blocking effects.

Where Unity test runners cannot exercise the native bridge directly in CI,
adapter conformance scenarios should still be runnable against a headless core
or a thin test host so expected Recite traces stay shared with other adapters.

## MVP and Follow-Up Sequencing

The implementation sequence should keep both Unity facades in view while
allowing staged delivery:

1. Unity schema exporter design implementation: editor-only producer from C#
   declarations, `ScriptableObject` registries, and asset-backed snapshots into
   the canonical generated manifest.
2. Unity GameObject/OO MVP: UPM package, imported compiled asset, managed native
   bridge, `ReciteDialogueService`, `ReciteDialogueRunner`, structured outputs,
   conditions, effects, save/load, and `reload_for_next_session_only`.
3. Unity DOTS facade: ECS authoring/baking, request/output components or
   buffers, and systems backed by the shared adapter core.
4. Unity editor refresh workflow: importer/status UI, schema freshness checks,
   `recite watch` integration, and play-mode asset-change reporting.
5. Unity conformance and documentation: adapter conformance fixtures, EditMode
   and PlayMode coverage, setup guide, troubleshooting, and known limitations.

Issue #108 should use this design as input for the Unity MVP. Issue #122 should
use the same shared-core and active-session decisions for Unity refresh work.
If the first implementation only ships the GameObject/OO facade, it must still
leave the shared adapter core and asset/session model suitable for the DOTS
facade.

## Known Limitations and Validation Gaps

- The native bridge ABI, binary distribution, and supported Unity platforms are
  not selected here.
- The exact `ScriptableObject`, importer, settings, and C# attribute shapes are
  illustrative.
- DOTS delivery may be staged after the GameObject/OO MVP.
- Generated C# bindings are deferred; hand-written or generic structured values
  are acceptable for the MVP.
- Unity CI coverage for native plugins may require platform-specific runners.
- Rich mid-session patch reload is out of scope for v1.
- Unity asset GUIDs are useful provenance, but projects still need a stable
  dialogue-facing symbol policy for schema values.
