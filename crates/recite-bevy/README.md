# Recite for Bevy 0.19

`recite-bevy` loads validated `.recitec` assets through Bevy's asset server and drives one dialogue
session in a `ReciteOwner` resource. Its default feature set uses Bevy's app, ECS, and asset APIs
without a renderer, window, audio, or UI.

Add `AssetPlugin` before `RecitePlugin` in a headless `App`; normal Bevy `DefaultPlugins` already
include the asset plugin. Send ordered `ReciteRequest` messages, then read `ReciteOutput` messages
after `ReciteSet::ProcessRequests`. Register pure condition handlers in `ReciteConditions`; execute
emitted effect requests in game systems and acknowledge blocking requests by their stable ID.

The chosen refresh policy is `reload_for_next_session_only`: accepted new compiled assets enter the
Bevy cache, while an active session retains its original compiled revision. A failed asset load
keeps the last accepted cache entry and emits a typed `ReciteAssetImport::Rejected` status with the
attempted path and retained revision. Recite compiled asset IDs, choice IDs, and effect IDs remain
independent of Bevy `AssetId` and `Entity` values. Source/schema freshness cannot be determined from
compiled bytes alone; run `recite check-fresh` in the authoring project.

The Rust schema producer supports fallible typed builders and the `recite_schema!` declaration
macro. Export native declarations with `ReciteSchema::export_json_with_producer` and a stable
`ProducerIdentity` (`kind = "bevy"`) so the generated manifest records truthful ownership. The
lower-level `export_json` emits an unowned canonical manifest.

## Verification

Run `cargo test --locked -p recite-bevy`. The
[conformance tests](https://github.com/plethu/recite/blob/main/crates/recite-bevy/tests/conformance.rs)
exercise real Bevy Apps and transactional output batches; they do not execute every published
reference trace byte for byte. The detailed availability-reason fixture and exact version-99
snapshot mutation remain unrun through Bevy. Immutable session ownership makes asset substitution
during advance unavailable by construction. Projection is unsupported; compiled-only imports have no
source/schema freshness visibility. The
[shared scenarios](https://github.com/plethu/recite/blob/main/fixtures/adapter-conformance/v1/scenarios.json)
own the required observations. See [performance](#performance) for the headless probe.

## Installing and upgrading

Until the Recite crates are published, use this repository as a path dependency.
`scripts/check-bevy-package.sh` prepares real Cargo `.crate` archives for `recite-bevy` and its
unpublished Recite dependencies. It uses temporary Cargo patches for dependency resolution, then
runs the example from the Bevy archive in a separate project using only extracted archives. The
archives and SHA-256 manifest, license texts, and replayable consumer are under
`target/recite-bevy-probe/cargo-packages/`. Run `cargo fetch --locked` from the repository first if
the local Cargo cache is empty. Each archive carries the `MIT OR Apache-2.0` license metadata; the
distribution bundle carries both full license texts. This check does not publish the crates.

When updating, keep `recite-bevy` and the other Recite crates on the same compatible release. Keep
Bevy on the version named above until this adapter declares support for another version. Re-export
the host schema, rebuild the `.recitec` assets, and run `recite check-fresh <project-root>` before
loading them. An active session keeps the compiled revision it started with; start a new session
after an accepted import to use rebuilt content. Preserve authored line, choice, and effect IDs when
editing source, and test restore from any save format your game intends to carry across the upgrade.

## Performance

Run `CARGO_TARGET_DIR=<disk-backed target> scripts/probe-bevy-performance.sh` from the repository
root. The script builds the CPU-only Bevy example once, compiles the published runtime-surface
fixture outside the measured commands, then times separate child processes with Python's external
wall timer and `wait4` resource usage. There are no CI thresholds. The report is retained at
`$CARGO_TARGET_DIR/recite-bevy-probe/performance.txt`.

The
[September 2026 observations](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#bevy-performance)
record one host/profile. Rerun the probe for the candidate under investigation; those values are not
a current baseline or regression budget.
