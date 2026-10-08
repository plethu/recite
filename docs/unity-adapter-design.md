# Unity adapter boundaries

The [package README](../Packages/com.recite.dialogue/README.md) owns installation, APIs, upgrades
and tested host support. The [adapter contract](engine-adapter-contract.md) owns runtime semantics;
this guide records Unity-specific ownership decisions.

## Ownership

The GameObject/OO package uses a plain C# session service over `recite-ffi`; no Entities dependency
is required. Native Recite owns traversal, schema validation, snapshots and catalogue resolution.
Managed code owns handles, AOT callback storage, byte decoding and Unity events. It does not
maintain a second condition, plural-rule or localisation implementation.

Construct the service in the runner's `Awake`, not a field initializer that Unity may run on a
loading thread. Use and dispose native handles on that owner thread. The runner ends on disable and
disposes on destruction. It queues a complete output batch before notifying listeners, so a nested
runner operation cannot interleave output ahead of the current batch. This post-operation event
delivery is distinct from prohibited re-entry inside a native condition callback.

## Authoring and refresh

`ReciteSchemaRegistration` owns Unity declarations. Its stable asset GUID is the producer identity;
temporary TOML transports declarations to the canonical CLI exporter. Failed validation preserves
the old manifest.

The compiled importer validates candidates through native Recite. Its GUID-keyed last-valid cache
under `Library/Recite/CompiledCache` lets a rejected import retain a validated revision and report
its error. Cache-write failure rejects the candidate. Clearing `Library` removes this derived
fallback and requires valid source again. Active sessions retain their loaded revision;
compiled-only imports do not establish source/schema freshness.

Legacy `TextAsset` scene references need the explicit migration described in the package guide.
Preserve `.meta` GUIDs and inspect backed-up scene/prefab changes before adopting them.

## Localisation and package boundary

PO imports validate a complete candidate before attaching its native catalogue revision. An attached
revision remains owned until explicitly replaced or cleared, including across restore preparation.
The C ABI retains callbacks for other hosts; the Unity package uses the native PO path.

Ship one matching native library, header/binding contract and managed package. Allocations must be
freed through the library that created them. Platform plugins and import metadata belong to the
package, not an undocumented consumer build step.

## Verification limits

Headless managed tests exercise real native calls but do not prove Unity import, scene
serialization, Mono/IL2CPP, play-mode lifecycle or player behavior. Run the package's
installed-Editor tests and selected player backend for the claimed profile. CoreCLR probes remain
experimental. Other native platforms require their own plugin and host evidence.
