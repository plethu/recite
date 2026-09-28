# Unity Adapter Design and Delivery State

This document applies the [engine adapter contract](engine-adapter-contract.md)
to the Unity companion. The production spec remains normative for dialogue,
schema, localisation, save/load, diagnostics, and performance (§16–19). The
first implementation is a GameObject/OO package. Unity 6.7+ is the primary
development and runtime modernization target; the declared Unity 2022.3
minimum is a pinned, best-effort compatibility lane. The deliberately small
host matrix is Linux x86_64. The package ships a native plugin for that
platform; other editor/player platforms and DOTS are future work. No Entities
package is required by the base UPM package.

## Ownership

`ReciteDialogueService` is a plain C# one-session owner. `NativeSession` owns
native asset/session handles and captures the construction thread. The runner
constructs the service in `Awake`, since a Unity field initializer can run on
a loading thread. `ConditionCallbacks` owns static AOT callback dispatch and
its GCHandle/pinned response lifetime. `NativePoCatalog` owns a native PO
catalogue handle. `OutputDecoder` converts native MessagePack batches into
structured C# output. Native Recite owns traversal, schema validation,
compatibility, plural rules, locale fallback, and errors. The C# runtime does
not implement a second dialogue or catalogue policy.

The service accepts immutable copied compiled bytes and loads them through
`recite-ffi`. Native `recite_asset_info` returns canonical metadata for
presentation and active/available revision comparison. It does not replace
native start/restore compatibility checks. One active session is enforced.
`Start`, `SelectChoice`, and `AcknowledgeEffect` drain native output
transactionally. Conditions are pure host queries; effects are structured
requests for game-side work. Snapshots are opaque native bytes. A pending
blocking request is preserved across restore and must be reconciled by the
host before acknowledging it.

The GameObject runner is a scene facade. It emits structured `UnityEvent`
output/errors, ends its session on disable, and disposes on destruction. It
queues whole batches during event delivery, so a handler reentering the runner
cannot interleave nested output before the current batch is finished. The
service and its native handles must be used/disposed on their owner thread.
Editor play exit and domain reload call Unity disable/destroy lifecycles;
Enter Play Mode configurations still need a real Editor host check.

## Authoring and refresh

`ReciteSchemaRegistration` is an explicit Unity-owned declaration asset for
conditions, effects, and referenced enums, registries, and speakers. An
editor-only TOML transport carries those declarations to
`recite export-schema --producer-kind unity --producer-id <registration GUID>`.
The CLI/core validate and emit the canonical manifest with its producer
fingerprint. The GUID stays stable when an asset is renamed. Export stages the
output and publishes only after a valid structured success response. Invalid
declarations leave the prior manifest in place. Unity does not duplicate
schema validation or hashing. Advanced domains may use standalone Recite
schema input.

`ReciteCompiledImporter` validates each `.recitec` candidate through the
native loader before publishing `ReciteCompiledAsset`. A valid candidate is
also written atomically to a GUID-keyed last-valid cache under
`Library/Recite/CompiledCache`. If a later candidate is rejected, the importer
revalidates and republishes that cached revision, marks it retained, and logs
structured status/message. If no safe cache exists it publishes no asset. A
cache write failure rejects the candidate, preserving the previous valid
cache/resource rather than claiming guaranteed retention. A subsequent valid
candidate clears the retained/error state. Removing `Library` removes the
derived fallback cache and requires valid source rebuilding.

The changed-asset policy is `reload_for_next_session_only`: active native
sessions own their original revision; next start uses the latest valid
available revision. Active `ReciteAssetInfo` and imported resource metadata
permit a host to compare canonical content fingerprints. A `.recitec` import
has no source/project/schema visibility, so freshness relative to authored
inputs is unavailable there. Content identity is not a freshness claim.

Before this importer, `.recitec` references pointed to a `TextAsset` local file
ID. `ReciteLegacyReferenceMigration` updates resolvable runner references in
scenes/prefabs to the imported object's actual local ID, preserving asset GUIDs
and backing up original YAML under `Library/Recite/MigrationBackups`. The
Editor fixture starts from the previous scene/meta representation and checks
that the migrated runner resolves its compiled asset.

## Localisation and package boundary

`SetPoCatalog` accepts all PO documents for one candidate revision. Native
Recite parses and validates them, merging identical entries and rejecting
conflicts. The candidate replaces the current catalogue only after success.
`SetPoCatalog(null)` clears the provider for current and future sessions.
Existing sessions retain their attached native catalogue revision until a new
one is explicitly attached. `Restore` attaches the owned catalogue before the
first output drain. The C ABI callback functions remain for other hosts, but
the built-in C# localisation API has only the native PO path. Variant changes
are validated and applied directly to the native session.

The reproducible bundle script packages the runtime/editor assemblies, sample,
tests, and Linux x86_64 native plugin with PluginImporter metadata. Unity
2022.3 remains the declared minimum for best-effort compatibility. No compiled
DLL for other platforms is advertised. The source package alone requires a
matching native plugin to be placed under `Runtime/Plugins/x86_64`.

## Verification limits

`scripts/check-unity-adapter.sh` builds the native library and CLI, compiles a
headless managed subset, exercises real FFI paths, and checks canonical schema
export. `scripts/unity/perf-probe.sh` reports named informational .NET 8 timings
with iteration counts and no thresholds. `scripts/unity/build-upm.sh` builds
and inspects the package tarball. EditMode/PlayMode runners exist at
`scripts/unity/run-unity-tests.sh`; they require an installed Unity Editor.
Headless .NET results do not establish Unity serialization, Mono/IL2CPP,
ScriptedImporter or player behavior. Clean-consumer Unity 2022.3.62f3 and
6.7.0b2 Editor runs each passed 3 EditMode and 3 PlayMode tests. The bounded
player matrix is 2022.3.62f3 Mono and 6.7.0b2 IL2CPP, with a separate
experimental 6.7.0b2 CoreCLR probe. The runner builds a Linux desktop player
and bounds its runtime to 90 seconds. Each player passed the imported-resource
PlayMode test (1/1) in a clean consumer. Mono remains a best-effort 2022.3
backend; IL2CPP is the primary 6.7 player backend.
Unity's [6.7 scripting documentation](https://docs.unity.com/en-us/engine/6000.7/manual/scripting/compilation-and-code-reload/script-compilation/backends/coreclr)
says its Editor still uses Mono and desktop CoreCLR is an experimental technical
preview, unsuitable for production. The [June 2026 CoreCLR update](https://discussions.unity.com/t/coreclr-scripting-and-serialization-update-june-2026/1723299)
targets supported Editor/player CoreCLR in Unity 7.0. That target does not
establish current 6.7 CoreCLR support for this package.
