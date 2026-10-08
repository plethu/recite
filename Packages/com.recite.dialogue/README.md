# Recite Dialogue for Unity

This Unity Package Manager package declares Unity 2022.3 as its minimum for best-effort
compatibility. **Unity 6.7+ is the primary development and runtime modernization target**, with a
deliberately small Linux x86_64 host test matrix. Mono remains a best-effort backend for the pinned
2022.3 lane. The current bundle contains a Linux x86_64 native plugin for the Editor and standalone
player. Other platforms need a matching `recite-ffi` build and PluginImporter configuration. The
base package has no Entities dependency; a DOTS facade is not included.

| Host lane                       | Scripting back ends and purpose                                                                            | Evidence status                                                                                 |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Unity 6.7.0b2, Linux x86_64     | Primary Editor import, EditMode/PlayMode (Editor Mono), IL2CPP player; separate experimental CoreCLR probe | Editor EditMode 3/3 and PlayMode 3/3; IL2CPP player 1/1; experimental CoreCLR player 1/1 passed |
| Unity 2022.3.62f3, Linux x86_64 | Older minimum, best-effort Editor import, EditMode/PlayMode, and Mono player                               | Editor EditMode 3/3 and PlayMode 3/3; Mono player 1/1 passed                                    |

Unity's
[6.7 scripting documentation](https://docs.unity.com/en-us/engine/6000.7/manual/scripting/compilation-and-code-reload/script-compilation/backends/coreclr)
states that the Editor still runs Mono and the desktop CoreCLR player is an **experimental technical
preview**, unsuitable for production. CoreCLR experiments are separate from the IL2CPP
production-backend check above. Unity's
[June 2026 update](https://discussions.unity.com/t/coreclr-scripting-and-serialization-update-june-2026/1723299)
targets supported CoreCLR Editor/player delivery in Unity 7.0; that roadmap is not a current support
claim for this package.

## Install and build

From the Recite checkout, build a reproducible package archive:

```bash
CARGO_TARGET_DIR=/path/on/disk/recite-target scripts/unity/build-upm.sh
```

The script prints `com.recite.dialogue-0.1.0-linux-x86_64.tgz`. Add that archive with Unity Package
Manager's **Add package from tarball** action. The archive contains
`Runtime/Plugins/x86_64/librecite_ffi.so`, the runtime/editor assembly definitions, tests, and the
Basic Dialogue sample. The native plugin exports Recite FFI 0.7.0. Source-tree installs of
`Packages/com.recite.dialogue` require the same native library under that package's
`Runtime/Plugins/x86_64` folder.

Import the Basic Dialogue sample from Package Manager, then run **Tools > Recite > Migrate Legacy
Compiled Asset References** to bind its scene to the imported `.recitec` object's actual Unity local
ID. Its scene uses `ReciteDialogueRunner` and an imported `.recitec` asset. Register conditions from
a component's `Start` or later, after the runner's `Awake` has created its thread-affine service.
The runner ends a session on disable and disposes it on destruction; re-enabling permits another
start. Calls and disposal must occur on the service's owner thread. UnityEvents expose structured
output and errors. A listener may call another runner operation; the runner delivers the current
batch fully before a nested operation's output.

## Runtime

`ReciteDialogueService` is a plain C# one-session owner. Its `Start`, `SelectChoice`,
`AcknowledgeEffect`, `Snapshot`, and `Restore` methods use native Recite traversal. Conditions are
synchronous queries. Effects are requests for the game to perform and acknowledge, never game
mutations inside Recite. Pass stable choice/effect IDs from structured output. Save the opaque
snapshot beside game state. A restored pending blocking effect may be emitted with the same request
ID, so hosts should reconcile their game operation before acknowledging.

```csharp
using var service = new ReciteDialogueService();
service.RegisterCondition("has_key", args => inventory.Has((string)args[0]));
service.SetInterpolationValues(new[] { ReciteInterpolationValue.String("name", "Ada") });
service.SetPoCatalog(new[] { new RecitePoDocument("fr", File.ReadAllBytes("fr.po")) });
var first = service.Start(compiledAsset.ToDialogueAsset(), locale: "fr-CA");
```

`SetPoCatalog` accepts all PO files for a revision, including multiple files per locale. The native
parser validates gettext plural rules, entries, conflicts, and placeholders. A failed candidate
leaves the previous catalogue in place. A successful refresh applies to the current session's next
traversal; previously returned output is unchanged. `SetPoCatalog(null)` removes the provider for
the active and later sessions. Source text is used where no translation matches. `SetLocaleVariant`
can change the active grammatical variant. Restore passes the owned catalogue before the first
output drain. The C ABI still offers callbacks for non-C# hosts; this package's built-in
localisation path is PO only.

`ActiveAssetInfo` reports the native-validated revision retained by an active session. The imported
resource reports the latest available revision and whether it came from the last-valid cache.
Compare their canonical content fingerprints to show an active/available difference after reimport;
equal asset names alone do not establish equal content. Native start/restore enforce compatibility.
Editor import of `.recitec` has no source/project/schema inputs, so it does **not** assert source
freshness.

## Schema and authoring

Create a `ReciteSchemaRegistration` asset with game-owned condition/effect signatures and any
referenced enum, registry, or speaker declarations. Set its output to an `Assets/*.json` path, then
use **Tools > Recite > Export Schema**. The editor lowers declarations to a temporary TOML transport
and runs `recite export-schema` with the asset's stable Unity GUID as producer ID. Recite's core
validates and writes the canonical manifest and fingerprint; Unity does not implement schema
semantics or hashing. A malformed export retains the prior manifest and logs structured CLI
diagnostics. The CLI must be available as `recite` or at the registration's configured executable
path. Advanced schema domains can be authored as a standalone Recite schema source.

Build dialogue with the CLI or `recite watch`, then let Unity import the resulting `.recitec`. The
`ScriptedImporter` validates compiled bytes through native Recite before publishing
`ReciteCompiledAsset`. It stores a last-valid copy under `Library/Recite/CompiledCache/<asset
GUID>.recitec`. A failed candidate logs an import error and republishes the validated last-good
bytes with `IsUsingRetainedRevision`, `ImportStatus`, and `ImportMessage` exposed. An existing
active session keeps its original revision; the next start uses the latest valid available revision.
A successful reimport clears retained status. This cache is derived local state: deleting `Library`
loses last-good fallback and requires a valid source rebuild. Cache write failure fails import
rather than claiming a durable last-good revision.

## Upgrading old scenes

Before this importer, `.recitec` was a `TextAsset` with local file ID `4900000`. Existing scene and
prefab references need migration to the imported `ReciteCompiledAsset`. Keep the original
`.recitec.meta` GUID, import the file, then run **Tools > Recite > Migrate Legacy Compiled Asset
References**. The command updates only resolvable `ReciteDialogueRunner` references and saves
original YAML under `Library/Recite/MigrationBackups`. Commit scenes and meta files together after
inspecting the result. A sample scene and EditMode fixture exercise this path.

## Verification

```bash
CARGO_TARGET_DIR=/path/on/disk/recite-target scripts/check-unity-adapter.sh
CARGO_TARGET_DIR=/path/on/disk/recite-target scripts/unity/perf-probe.sh
UNITY_EDITOR=/path/to/Unity scripts/unity/run-unity-tests.sh
UNITY_EDITOR=/path/to/Unity RECITE_UNITY_PLAYER_MODE=mono scripts/unity/run-unity-tests.sh
UNITY_EDITOR=/path/to/Unity RECITE_UNITY_PLAYER_MODE=il2cpp scripts/unity/run-unity-tests.sh
UNITY_EDITOR=/path/to/Unity-6.7 RECITE_UNITY_PLAYER_MODE=coreclr scripts/unity/run-unity-tests.sh
```

The first check builds the native library and CLI, tests the managed service through real FFI, and
checks schema export. Its [managed suite](Tests~/Headless/ReciteUnityNativeCases.cs) observes
transactional batches, not every reference advance. The
[shared scenarios](https://github.com/plethu/recite/blob/main/fixtures/adapter-conformance/v1/scenarios.json)
own required observations. Projection is unsupported, and isolated compiled assets cannot establish
source/schema freshness. The larger managed suite is distinct from the single imported-resource
player smoke test. The informational `.NET 8` probe prints iteration counts and total time for
native load/metadata decode, managed line/effect conversion, condition traversal, and inactive
service access; it has no pass threshold. With Unity Editor installed, the host runner extracts the
built UPM archive into a temporary clean consumer project and executes EditMode and PlayMode tests.
The optional Linux standalone player modes use Unity Test Framework's `testSettingsFile` to select
Mono or IL2CPP; the matching Linux build support and IL2CPP module must be installed. The CoreCLR
mode separately probes whether the Editor offers that enum, selects it explicitly, and runs an
experimental player test; it is not a production backend claim. Linux desktop test players run with
`-batchmode -nographics` and a 90-second runtime limit after launch. Set `TMPDIR=/var/tmp` to place
the clean consumer project and default result directory there when the host requires a different
temporary-file location. Set `RECITE_UNITY_UPM_BUNDLE` to test a specific archive and
`RECITE_UNITY_TEST_RESULTS` to retain logs/XML at a chosen location. Headless .NET results alone are
not evidence of Mono, IL2CPP, ScriptedImporter, serialization, or Unity scene behavior. In clean
Linux x86_64 consumers, both Editor suites passed 3 EditMode and 3 PlayMode tests. The 2022.3.62f3
Mono player and 6.7.0b2 IL2CPP player each passed the imported-resource PlayMode test (1/1); the
6.7.0b2 CoreCLR preview player also passed that test (1/1).
