# Historical delivery and measurement evidence

Extracted from the active guides at `58b8f04965af` on 7 October 2026.
These observations retain their original execution boundaries. Current commands,
requirements and delivery state live in the guides and GitHub.

## Writer packaging

Original owner: `apps/writer/packaging.md`.

### Local preview evidence

On September 23, the Linux release build produced a `.deb` and passed artifact,
license, CLI and runtime-ABI inspection. Its extracted binary and packaged desktop
entry passed `scripts/check-writer-desktop-links.py`: `gio` delivered encoded
Unicode/space-containing links, the running writer acknowledged the second scene,
and a malformed route exited with code 2. Fixture bytes and a deliberately
overridden configuration sentinel stayed unchanged. The test used a private
D-Bus session and temporary desktop association.

The shared project-loader regressions also cover welcome-screen activation,
clean project switches, source/translation draft refusal, navigation history,
and clearing the previous project's clean catalogue on a route-less switch.
Cancellation before handling is tested; the load-completion cancellation and
edit guards were inspected, without a deterministic mid-load UI test.

The local package requires the host's GLIBC 2.44. It is a host-specific preview;
the Ubuntu 24.04 CI baseline, macOS and Windows runs, and package-manager
install/upgrade/uninstall acceptance remain outstanding.

## Bevy performance

Original owner: `crates/recite-bevy/PERFORMANCE.md`.

Observed on 2026-09-28, Rust/Cargo 1.96.0, Bevy 0.19.1, Linux x86_64,
AMD Ryzen AI 7 350, debug profile:

| Probe | Work | Wall time | Peak RSS |
| --- | ---: | ---: | ---: |
| Decode and validate | 10,000 compiled asset conversions, 3,266-byte input | 1.861 s | 13,720 KiB |
| Idle | 10,000 `App::update` calls, no session | 0.504 s | 13,848 KiB |
| Active | 1,000 start/choice/blocking ack/choice/end cycles; 1,000 condition dispatches | 0.426 s | 15,204 KiB |
| Retained revision | One active refresh and next-session start | 0.009 s | 15,092 KiB |

The retained-revision probe observes two distinct compiled revision identities
while the old session and new asset cache coexist. Public APIs do not expose
the underlying allocation or `Arc` counts, so the probe does not claim a
byte-accurate retained-memory figure. Wall/RSS figures include process startup
and vary by host, profile, and fixture.

## Editor parity

Original owner: `docs/editor-parity-contract.md`.

### Milestone 4 reconciliation

The evidence now closes the remaining #53 and #192 acceptance questions for the
named Linux hosts without changing the Milestone 4 exit gate or broadening any
platform claim.

- #53 command availability and lifecycle are covered by the VS Code/VSCodium
  and Neovim structured adapters, including Neovim extract and valid run/trace;
  Zed's supported workflow is the explicit static terminal projection, with
  exact validate/extract/compile argv, cwd, and status plus genuine watch
  Ctrl-C termination. Zed does not parse task records, expose a native task
  cancellation controller, or ship built-in run/trace tasks because their
  asset, block, and fixture inputs are explicit. Those are documented client
  limits, not missing M4 evidence.
- LSP request cancellation now has shared-server stdio and deterministic
  coordinator evidence. Installed-client cancellation remains unclaimed.
  Delivery belongs to #206 as a serious-v1 scheduler/performance capability,
  not M4 command/watch work.
- #192 package, activation, grammar, installed LSP, keyboard, and task
  acceptance is covered on Zed 1.18.1 Linux x86_64. The host sent the
  client-generated post-emoji UTF-16 request, applied the canonical missing-ID
  code action, applied exactly the two returned rename edits, and proved the
  finite task argv/cwd/status boundaries. Stale-version rejection remains a
  lower-level test boundary; response-range conversion, non-Linux hosts,
  accessibility, gallery/distribution, parsed task diagnostics, native task
  cancellation, and built-in run/trace remain unclaimed.
- The keyboard workflow evidence is recorded under closed #202. Package,
  source, and headless checks remain supporting evidence only, and broader
  Milestone 5 accessibility proof is not implied.

## C ABI delivery

Historical packaging issue context (the invariant now lives in buffer ownership):

- **IL2CPP allocator mismatch:** `recite_buffer_free` must call the same
  allocator that allocated the buffer — i.e. Rust's allocator inside
  `recite-ffi`. If a Unity IL2CPP build links against a different copy of
  `recite-ffi` than the one that produced the buffer (e.g. a statically linked
  runtime vs a pre-built `.dll`), the free call goes to the wrong allocator and
  is undefined behaviour. The current Unity refresh and packaging work in
  [#85](https://github.com/plethu/recite/issues/85) and [#133](https://github.com/plethu/recite/issues/133)
  must document that `recite-ffi` is always distributed as a single pre-built
  `.dll`/`.so` that both Mono and IL2CPP P/Invoke load at runtime — never
  recompiled per backend or statically linked into the Unity player separately.


Original owner: `docs/c-abi-boundary-design.md`.

### Follow-Up Issues

The C ABI design and its implementation follow-ups were completed under the
historical `Milestone 8: Engine Adapter Contract`:

1. [#128 Adapters: design the C ABI boundary for non-Rust engine adapters](https://github.com/plethu/recite/issues/128)
   records this design.

2. [#130 FFI: implement recite-ffi crate (extern C surface)](https://github.com/plethu/recite/issues/130)
   delivered the `recite-ffi` workspace member.

3. [#131 FFI: cbindgen header generation and packaging](https://github.com/plethu/recite/issues/131)
   delivered the stable `include/recite.h` header. `pkg-config` or CMake
   find-module support remains out of scope for v1 unless a downstream package
   needs it.

The remaining Unity-facing ABI, refresh, documentation, and packaging work is
owned by the current [Milestone 23: 7 Engine Companions](https://github.com/plethu/recite/milestone/23):

- [#85 Unity: add editor import and refresh workflow](https://github.com/plethu/recite/issues/85)
  applies the ABI and changed-asset policy to the editor workflow.
- [#86 Docs: document engine authoring refresh workflows and reload limits](https://github.com/plethu/recite/issues/86)
  records the supported refresh and ABI distribution boundaries.
- [#133 Unity: prepare Asset Store and UPM distribution package](https://github.com/plethu/recite/issues/133)
  owns the Unity package and native artifact distribution surface.

## Adapter prerequisites

Original owner: `docs/engine-adapter-contract.md`.

This contract unblocks adapter implementation and refresh planning for [#46
host-agnostic conformance tests](https://github.com/plethu/recite/issues/46),
[#47 Godot adapter MVP](https://github.com/plethu/recite/issues/47), [#49 Bevy
adapter MVP](https://github.com/plethu/recite/issues/49), [#72 cross-engine
acceptance matrix](https://github.com/plethu/recite/issues/72), [#73 Unity
adapter MVP](https://github.com/plethu/recite/issues/73), [#83 Godot refresh
workflow](https://github.com/plethu/recite/issues/83), [#84 Bevy refresh
workflow](https://github.com/plethu/recite/issues/84), [#85 Unity refresh
workflow](https://github.com/plethu/recite/issues/85), and [#86 Docs: document
engine authoring refresh workflows and reload limits](https://github.com/plethu/recite/issues/86).

## Engine companions

Original owner: `docs/engine-companions-design.md`.

These are verification targets, not blanket support declarations. Clean
consumer Unity 2022.3.62f3 and 6.7.0b2 Editor suites each passed 3 EditMode
and 3 PlayMode tests. The 2022.3 Mono and 6.7 IL2CPP desktop players each
passed the imported-resource test (1/1); the separate 6.7 experimental
CoreCLR player also passed 1/1. Unity's
[6.7 scripting documentation](https://docs.unity.com/en-us/engine/6000.7/manual/scripting/compilation-and-code-reload/script-compilation/backends/coreclr)
describes the Editor as Mono-based and desktop CoreCLR as an experimental
technical preview unsuitable for production. Its
[June 2026 update](https://discussions.unity.com/t/coreclr-scripting-and-serialization-update-june-2026/1723299)
targets supported Editor/player CoreCLR in Unity 7.0; no current 6.7 CoreCLR
production claim follows. The host's Arch-based OS also requires empirical
results; Unity's
[Ubuntu system requirements](https://docs.unity3d.com/2022.3/Documentation/Manual/system-requirements.html)
do not establish Arch compatibility. Other platforms require their own native
builds and host evidence before inclusion in a support claim.

## Unity adapter

Original owner: `docs/unity-adapter-design.md`.

Clean-consumer Unity 2022.3.62f3 and
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

## Godot performance

The informational headless profile on an AMD Ryzen AI 7 350, Linux x86_64,
official Godot 4.6.3, debug GDExtension measured 25 loads in 7,724 µs,
25 start/end output conversions in 3,247 µs, 25 condition/effect routes in
3,799 µs, and 500 inactive process notifications in 16 µs. These are sample
totals from one run, not release budgets or frame-time guarantees. The host
gate prints a new bounded measurement on each run.


Original owner: `tests/godot-host/coverage.md`.

The same clean host probe with the packaged release GDExtension measured
25 loads in 1,089 µs, 25 start/end routes in 1,573 µs, 25 condition/effect
routes in 1,491 µs, and 500 inactive notifications in 15 µs. The release
bundle also passed native import, rejected refresh retention, and the
packaged example probe.

## Engine authoring

Original owner: `docs/engine-authoring-workflows.md`.

### Workflow and host evidence

These are the tested Linux x86_64 profiles. Other platforms require their own
native builds and host checks. The companion fixtures distinguish exact traces,
equivalent batch observations, structural invariants, and unsupported capabilities;
they do not claim every reference scenario ran unchanged in every engine.

| Engine | Import and next-session behavior | Checks and evidence |
| --- | --- | --- |
| Bevy 0.19.1 | `AssetServer` loads compiled assets; an explicit reload reports accepted/rejected status. An active owner keeps its revision. Default dependencies do not enable automatic file watching or rendering. | `cargo test --locked -p recite-bevy`; [native asset tests](../../crates/recite-bevy/tests/native_asset.rs), [example](../../crates/recite-bevy/examples/headless_dialogue.rs), [conformance](../../crates/recite-bevy/CONFORMANCE.md). These exercise real App/AssetServer behavior, stable choices, prompt/blocking snapshots, and new-session revisions. |
| Godot 4.6.3 official standard build | The editor imports `.recitec` as a Resource. Nodes retain active revisions; failed imports can load the last valid cached revision. Clearing `.godot` removes that fallback. | `mise exec -- just engines godot`; [host script](../../scripts/check-godot-host.sh), [coverage](../../tests/godot-host/coverage.md). The host checks import, resource persistence, conditions/effects, snapshots, rejected refresh, and the packaged example. |
| Unity 6000.7.0b2, primary | `ScriptedImporter` publishes a native-validated resource. Active services retain their revision; rejected imports can use the GUID-keyed last-valid cache. Clearing `Library` removes that fallback. | [host runner](../../scripts/unity/run-unity-tests.sh): 3/3 EditMode and 3/3 PlayMode tests; 1/1 IL2CPP desktop player test; separate experimental CoreCLR player test 1/1. The Editor uses Mono. |
| Unity 2022.3.62f3, best effort | Same importer and session behavior. This is the single older compatibility profile. | The same host runner: 3/3 EditMode, 3/3 PlayMode, and 1/1 Mono desktop player test. [Managed and host coverage](../../Packages/com.recite.dialogue/Tests~/Headless/CONFORMANCE.md) distinguishes the broader .NET/native checks from these host smoke tests. |

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

## Engine package reproducibility

Original owner: `docs/engine-authoring-workflows.md`.

Both debug and release native libraries passed these checks.

The Unity archive was built twice from the same source and native-library
inputs and had the same SHA-256. This checks package assembly; it does not
claim that native binaries built with different compilers or system libraries
are identical. Native compiler inputs must be pinned for a release build.
