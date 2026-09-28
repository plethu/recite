# Engine authoring workflows

The [authoring walkthrough](../docs-site/src/content/docs/adapters/authoring.md)
starts with a small dialogue and follows a source error, correction, rebuild,
engine import, and new session. Engine setup is covered by the
[Bevy](../docs-site/src/content/docs/adapters/bevy.md),
[Godot](../docs-site/src/content/docs/adapters/godot.md), and
[Unity](../docs-site/src/content/docs/adapters/unity.md) guides.

## Shared behavior

The language server reports diagnostics while editing. `recite watch` validates
saved inputs and publishes a compiled asset only when the build succeeds.
Existing authored line and choice IDs stay in the source. A failed build leaves
the previous compiled output available.

All three adapters use `reload_for_next_session_only`. The active session keeps
its loaded revision. A valid refresh becomes available to the next session;
a rejected import reports an error and retains the last accepted revision when
one exists. A saved session still requires compatible compiled content. Stable
IDs alone do not make a snapshot compatible with an edited asset.

Run `recite check-fresh` where source and schema inputs are available. A runtime
that only has `.recitec` files reports freshness as unavailable. Optional
presentation projection is not implemented by these companions.

## Workflow and host evidence

These are the tested Linux x86_64 profiles. Other platforms require their own
native builds and host checks. The companion fixtures distinguish exact traces,
equivalent batch observations, structural invariants, and unsupported capabilities;
they do not claim every reference scenario ran unchanged in every engine.

| Engine | Import and next-session behavior | Checks and evidence |
| --- | --- | --- |
| Bevy 0.19.1 | `AssetServer` loads compiled assets; an explicit reload reports accepted/rejected status. An active owner keeps its revision. Default dependencies do not enable automatic file watching or rendering. | `cargo test --locked -p recite-bevy`; [native asset tests](../crates/recite-bevy/tests/native_asset.rs), [example](../crates/recite-bevy/examples/headless_dialogue.rs), [conformance](../crates/recite-bevy/CONFORMANCE.md). These exercise real App/AssetServer behavior, stable choices, prompt/blocking snapshots, and new-session revisions. |
| Godot 4.6.3 official standard build | The editor imports `.recitec` as a Resource. Nodes retain active revisions; failed imports can load the last valid cached revision. Clearing `.godot` removes that fallback. | `mise exec -- just test-godot`; [host script](../scripts/check-godot-host.sh), [coverage](../tests/godot-host/coverage.md). The host checks import, resource persistence, conditions/effects, snapshots, rejected refresh, and the packaged example. |
| Unity 6000.7.0b2, primary | `ScriptedImporter` publishes a native-validated resource. Active services retain their revision; rejected imports can use the GUID-keyed last-valid cache. Clearing `Library` removes that fallback. | [host runner](../scripts/unity/run-unity-tests.sh): 3/3 EditMode and 3/3 PlayMode tests; 1/1 IL2CPP desktop player test; separate experimental CoreCLR player test 1/1. The Editor uses Mono. |
| Unity 2022.3.62f3, best effort | Same importer and session behavior. This is the single older compatibility profile. | The same host runner: 3/3 EditMode, 3/3 PlayMode, and 1/1 Mono desktop player test. [Managed and host coverage](../Packages/com.recite.dialogue/Tests~/Headless/CONFORMANCE.md) distinguishes the broader .NET/native checks from these host smoke tests. |

Unity 6.7+ is the primary development target. Its CoreCLR player is experimental;
a successful smoke test does not establish production support. The tested
platform was an Arch-based Linux machine; this is local execution evidence,
not a claim of vendor support for that distribution.

The walkthrough's manifest and dialogue were exercised through an actual
`recite watch` process: the initial build succeeded, a missing jump target
produced a diagnostic without changing the compiled bytes, and the corrected
text produced fresh output with the same authored IDs. The existing CLI suite
can be run with `cargo test --locked -p recite-cli --test structured_watch`.

An additional headless Godot walkthrough ran that edit sequence while a native
Node held the original session. Refreshing its Resource left the active
revision and original choice usable; the next session emitted the edited text.
This was a one-off integration check using the documented source and the
packaged native addon. Bevy and Unity have the separate host checks listed above.

The public walkthrough gives the editor steps. Interactive LSP diagnostic
display and clicking through these steps in all three editors were not part
of the headless acceptance run.

## Packages before publication

Recite is preparing for its first public release. Local package verification
must work before any Recite dependency is on crates.io or an asset store.
Publishing the packages is a release step, not a prerequisite for these checks.

| Package | Build and consumer check | Scope |
| --- | --- | --- |
| Bevy | `scripts/check-bevy-package.sh` | Prepare real `.crate` archives with temporary Cargo patches for unpublished dependencies, then run the packaged example in an external consumer using only extracted archives. |
| Godot | `scripts/package-godot-addon.sh`; `scripts/check-godot-host.sh` | Bundle the native addon and example; check clean installation, import, and replacement without changing authored project files. |
| Unity | `scripts/unity/build-upm.sh`; `scripts/unity/run-unity-tests.sh` | Bundle runtime/editor assemblies, Linux native plugin, and sample; check clean installation, import/reimport, legacy reference migration, and the selected player backend. |

The Bevy check prepares each Cargo archive twice and compares the bytes. It
retains the archives, hashes, license texts, extracted crates, and consumer
under `target/recite-bevy-probe/cargo-packages/`. The consumer confirms the
next-session refresh policy and has no rendering or windowing dependencies.

The Godot host check compares independently staged archives, removes an
obsolete file from a prior package output, and runs the example extracted from
the archive. Its watcher must report successful builds before the editor
imports the generated asset. A second consumer replaces an old addon, verifies
that authored source and compiled bytes are unchanged, and starts the example
again. Both debug and release native libraries passed these checks.

The Unity archive was built twice from the same source and native-library
inputs and had the same SHA-256. This checks package assembly; it does not
claim that native binaries built with different compilers or system libraries
are identical. Native compiler inputs must be pinned for a release build.

Package replacement checks exercise local development artifacts. They do not
imply a previously published version or a compatibility promise for every
development snapshot. Store submission and crates.io publication are outside
these checks.
