# Engine authoring workflows

The [authoring walkthrough](../docs-site/content/adapters/authoring.md) starts with a small dialogue
and follows a source error, correction, rebuild, engine import, and new session. Engine setup is
covered by the [Bevy](../docs-site/content/adapters/bevy.md),
[Godot](../docs-site/content/adapters/godot.md), and [Unity](../docs-site/content/adapters/unity.md)
guides.

## Shared behavior

The language server reports diagnostics while editing. `recite watch` validates saved inputs and
publishes a compiled asset only when the build succeeds. Existing authored line and choice IDs stay
in the source. A failed build leaves the previous compiled output available.

All three adapters use `reload_for_next_session_only`. The active session keeps its loaded revision.
A valid refresh becomes available to the next session; a rejected import reports an error and
retains the last accepted revision when one exists. A saved session still requires compatible
compiled content. Stable IDs alone do not make a snapshot compatible with an edited asset.

Run `recite check-fresh` where source and schema inputs are available. A runtime that only has
`.recitec` files reports freshness as unavailable. Optional presentation projection is not
implemented by these companions.

## Workflow checks

Run the maintained checks against the candidate and record the exact engine, platform, architecture
and backend. The [adapter acceptance matrix](adapter-acceptance-matrix.md) defines the required
coverage; [historical host observations](archive/delivery-evidence.md#engine-authoring) do not
establish a fresh candidate's support.

| Surface     | Maintained check                                                                                               | Evidence scope                                                                       |
| ----------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Bevy        | `cargo test --locked -p recite-bevy`; [conformance](../crates/recite-bevy/CONFORMANCE.md)                      | Real App/AssetServer, revisions, choices and snapshots                               |
| Godot       | `just engines godot`; [coverage](../tests/godot-host/coverage.md)                                              | Native import, Resource/Node behavior and packaged example                           |
| Unity       | `scripts/unity/run-unity-tests.sh`; [coverage](../Packages/com.recite.dialogue/Tests~/Headless/CONFORMANCE.md) | Installed Editor and selected player backend; managed tests alone do not prove these |
| CLI refresh | `cargo test --locked -p recite-cli --test structured_watch`                                                    | Invalid builds retain compiled bytes; correction publishes fresh output              |

Interactive diagnostic display and walkthrough usability need separate host evidence. Headless
checks do not establish those behaviors.

## Packages before publication

Recite is preparing for its first public release. Local package verification must work before any
Recite dependency is on crates.io or an asset store. Publishing the packages is a release step, not
a prerequisite for these checks.

| Package | Build and consumer check                                         | Scope                                                                                                                                                                          |
| ------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Bevy    | `scripts/check-bevy-package.sh`                                  | Prepare real `.crate` archives with temporary Cargo patches for unpublished dependencies, then run the packaged example in an external consumer using only extracted archives. |
| Godot   | `scripts/package-godot-addon.sh`; `scripts/check-godot-host.sh`  | Bundle the native addon and example; check clean installation, import, and replacement without changing authored project files.                                                |
| Unity   | `scripts/unity/build-upm.sh`; `scripts/unity/run-unity-tests.sh` | Bundle runtime/editor assemblies, Linux native plugin, and sample; check clean installation, import/reimport, legacy reference migration, and the selected player backend.     |

The Bevy check prepares each Cargo archive twice and compares the bytes. It retains the archives,
hashes, license texts, extracted crates, and consumer under
`target/recite-bevy-probe/cargo-packages/`. The consumer confirms the next-session refresh policy
and has no rendering or windowing dependencies.

The Godot host check compares independently staged archives, removes an obsolete file from a prior
package output, and runs the example extracted from the archive. Its watcher must report successful
builds before the editor imports the generated asset. A second consumer replaces an old addon,
verifies that authored source and compiled bytes are unchanged, and starts the example again.

Rerun the package checks for the candidate. The
[recorded reproducibility observations](archive/delivery-evidence.md#engine-package-reproducibility)
do not establish identical native binaries across compilers or system libraries. Native compiler
inputs must be pinned for a release build.

Package replacement checks exercise local development artifacts. They do not imply a previously
published version or a compatibility promise for every development snapshot. Store submission and
crates.io publication are outside these checks.
