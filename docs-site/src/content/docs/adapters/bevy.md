---
title: Bevy
description: Load compiled Recite dialogue and drive a single session in Bevy 0.19.
template: splash
---

`recite-bevy` is a Bevy 0.19.1 companion for compiled `.recitec` assets. It uses Bevy's app, ECS,
asset, and reflection crates. The library brings no renderer, window, audio, or UI dependency; game
code chooses presentation and executes effects. The plugin can run in a headless `App`.

The
[headless example](https://github.com/plethu/recite/blob/main/crates/recite-bevy/examples/headless_dialogue.rs)
demonstrates conditions, every effect mode, locale variants, prompt and blocking effect saves, and a
rebuilt revision. Run it from the repository with `cargo run -p recite-bevy --example
headless_dialogue`. The example uses `recite-compiler` as an authoring-only dev dependency. The
shipped `recite-bevy` library never compiles source at runtime.

## Setup

Add `RecitePlugin` after Bevy's `AssetPlugin`. `DefaultPlugins` includes the asset plugin. A
headless app can use `TaskPoolPlugin`, `AssetPlugin`, and `RecitePlugin` in that order.
`TaskPoolPlugin` is needed when an `AssetServer` reads from disk; manually inserted assets do not
use the IO pool.

```rust
use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::AssetPlugin;
use recite_bevy::RecitePlugin;

let mut app = App::new();
app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default(), RecitePlugin));
```

Load `.recitec` through `AssetServer::load::<ReciteDialogueAsset>(path)`. Wait for
`ReciteAssetStatus` with `ReciteAssetImport::Accepted` before sending a start request. A rejected
load emits a typed `AdapterError`, attempted asset path, and retained revision if one exists. The
accepted revision contains both the authored Recite asset ID and its canonical compiled content
fingerprint; these are separate from Bevy's `AssetId` and `Entity` values. The standalone
`ReciteDialogueAsset::from_bytes` constructor supports headless tests and host-managed bytes.

The singleton `ReciteOwner` resource holds one active dialogue session. Send `ReciteRequest::Start {
asset, block_id, locale, variant }`, then `SelectChoice(ChoiceId)`, `AcknowledgeEffect { effect:
EffectId, ack: EffectAck }`, `Snapshot`, `Restore`, or `End`. Selection and acknowledgement require
stable authored IDs. A second start or an operation without a session emits a
`ReciteOutputValue::Error` with a stable error category. `Snapshot` returns opaque Recite session
bytes; save game state separately and provide caller-owned interpolation values again after restore.

Requests are processed in message order during `Update`. Every operation emits one `ReciteOutput`
with a monotonic sequence and an ordered batch or structured error. The shared driver drains
synchronously through lines and immediate effects until a prompt, blocking effect, end, or error. On
failure it commits no partial traversal batch. `ReciteSet` orders `PrepareContext`,
`ProcessRequests`, and `Output`; register game context producers before `ProcessRequests` and output
consumers in `Output` or after it. An idle frame reads an empty request queue and performs no
dialogue traversal.

Register pure typed Rust condition handlers in `ReciteConditions`. Each handler receives a read-only
`&World` and a `ConditionQuery`; it must return a `ConditionValue` or a typed
`ConditionEvaluationError`. Keep queries deterministic and free of game mutation. Game systems
observe generic `DialogueEvent::Effect` requests, convert them to their own typed effect commands if
useful, execute the game action, then acknowledge blocking effects with `EffectAck::Completed` or
`EffectAck::Failed { reason }`. Recite does not perform the action.

Import gettext PO text with `ReciteCatalog.0.import_po(locale, path, text)`; the shared parser
validates and applies each document transactionally. You can also insert entries directly. The
catalogue is borrowed on every operation, so replacing this resource changes the next operation of
an active session. The session locale is explicit at start, and `None` uses authored source text. A
grammatical variant is supplied at start or restore. `ReciteInterpolation` carries typed caller
values for placeholders; it is separate from the serialized session. Line and choice outputs retain
source text, localized text, stable IDs, metadata, and structured plural provenance. If a catalogue
is absent, the source text remains available. Validate malformed PO input in authoring/import code
before replacing the catalogue.

## Schema and authoring refresh

Declare schema-owned conditions, effects, speakers, and other domains with `ReciteSchema`'s typed
builder or `recite_schema!` macro, then call
`export_json_with_producer(ProducerIdentity::new("bevy", stable_id)?)`. Choose an ID owned by the
registration asset, independent of its filesystem path. `export_json()` is a lower-level unowned
export; it does not stamp native producer identity or input freshness. Both delegate to the
canonical serializer and validator used by Recite core; compiler, CLI, and LSP consume that
generated manifest. A host-owned TOML transport can instead run `recite export-schema --schema
schema.toml --output schema.json --producer-kind bevy --producer-id stable_id`.

The authoring loop is: edit `.recite` source or host schema declarations, check LSP diagnostics and
stable IDs, save, export the canonical schema manifest, let `recite watch <project-root>` rebuild
`.recitec`, then explicitly call `AssetServer::reload(path)` to request Bevy's import. Automatic
file watching is not enabled by this crate's CPU-only default dependencies. A new session uses the
accepted compiled revision. The [shared authoring walkthrough](/adapters/authoring/) follows
diagnostics, rebuild, import, and restart across the three engine companions. The v1 changed-asset
policy is **`reload_for_next_session_only`**: the active owner keeps its original compiled revision,
including while a prompt or blocking effect is pending. A failed refresh retains the last accepted
cache revision and reports `ReciteAssetImport::Rejected`; it does not pretend that the new source
was imported. Wait for an `Accepted` status before starting a session meant to use the new revision.

Source/schema freshness is distinct from compiled revision compatibility. When the game sees only
`.recitec` bytes, `ReciteAssetFreshness::Unavailable` is explicit. Run `recite check-fresh
<project-root>` where current source and schema inputs are visible. A compiled asset that fails
import validation cannot start a session. Active asset and locale catalogue replacement during a
session are separate operations; the compiled dialogue revision never swaps in place.

## Compatibility and evidence

The crate pins Bevy `=0.19.1` and uses its versioned
[asset loader](https://docs.rs/bevy_asset/0.19.1/bevy_asset/trait.AssetLoader.html),
[message](https://docs.rs/bevy_ecs/0.19.1/bevy_ecs/message/index.html), and
[app scheduling](https://docs.rs/bevy_app/0.19.1/bevy_app/) APIs. The adapter declares no
presentation projection capability. Its conformance observation mode is
`transactional_drained_batch`: published reference-driver scenarios use individual `advance` steps,
while this host API reports the equivalent ordered events in one batch and reports an error before
publishing a partial batch. The Bevy test suite executes the two mandatory adapter-runner fixtures
for plural metadata and localisation errors directly, plus published error category observations
through a real App and native AssetServer refresh. Its
[verification scope](https://github.com/plethu/recite/blob/main/crates/recite-bevy/README.md#verification)
distinguishes exact, equivalent, gated, and unrun scenarios. The
[headless performance probe](https://github.com/plethu/recite/blob/main/crates/recite-bevy/README.md#performance)
records load, idle, active, and retained-revision observations without CI timing thresholds. The
[adapter contract](https://github.com/plethu/recite/blob/main/docs/engine-adapter-contract.md)
defines the shared requirements.

Before the Recite crates are published, use the repository as a path dependency.
`scripts/check-bevy-package.sh` prepares seven real Cargo `.crate` archives with temporary local
patches for the unpublished dependencies, then runs the packaged headless example from a separate
project. It checks stable archive bytes across two builds and saves SHA-256 hashes, license texts,
and a replayable consumer in `target/recite-bevy-probe/cargo-packages/`. Run `cargo fetch --locked`
from the repository first on a cold cache. The probe does not publish crates.

For an upgrade, move the Recite dependencies together to a compatible release and retain Bevy 0.19.1
until a newer host version is supported. Re-export the schema, rebuild compiled assets, run `recite
check-fresh <project-root>`, and wait for an accepted Bevy import before starting a new session. An
active session continues on its original revision. Keep authored IDs stable, and test restore with
saves your game needs to preserve. A crates.io release needs the matching `recite-core`,
`recite-runtime`, and `recite-adapter` versions available before `recite-bevy`.
