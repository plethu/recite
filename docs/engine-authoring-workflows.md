# Engine authoring workflows

The [authoring walkthrough](../docs-site/src/content/docs/adapters/authoring.md) starts with a small
dialogue and follows a source error, correction, rebuild, engine import, and new session. Engine
setup is covered by the [Bevy](../docs-site/src/content/docs/adapters/bevy.md),
[Godot](../docs-site/src/content/docs/adapters/godot.md), and
[Unity](../docs-site/src/content/docs/adapters/unity.md) guides.

## Shared behavior

The [public walkthrough](../docs-site/src/content/docs/adapters/authoring.md) owns edit, validate,
rebuild, import and restart steps. The
[companion architecture](engine-companions-design.md#assets-and-refresh) owns active/available
revision policy; [adapter conformance](engine-adapter-contract.md#13-adapter-conformance-fixtures)
owns compatibility and freshness requirements. Compiled-only hosts cannot establish source/schema
freshness, and these companions do not expose presentation projection.

## Workflow checks

Run the maintained checks against the candidate and record the exact engine, platform, architecture
and backend. The [adapter contract](engine-adapter-contract.md) defines the required coverage;
[historical host observations](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#engine-authoring)
do not establish a fresh candidate's support.

| Surface     | Maintained check                                                                                       | Evidence scope                                                                       |
| ----------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| Bevy        | `cargo test --locked -p recite-bevy`; [conformance](../crates/recite-bevy/README.md#verification)      | Real App/AssetServer, revisions, choices and snapshots                               |
| Godot       | `just engines godot`; [coverage](../addons/recite/README.md#verification)                              | Native import, Resource/Node behavior and packaged example                           |
| Unity       | `scripts/unity/run-unity-tests.sh`; [coverage](../Packages/com.recite.dialogue/README.md#verification) | Installed Editor and selected player backend; managed tests alone do not prove these |
| CLI refresh | `cargo test --locked -p recite-cli --test structured_watch`                                            | Invalid builds retain compiled bytes; correction publishes fresh output              |

Interactive diagnostic display and walkthrough usability need separate host evidence. Headless
checks do not establish those behaviors.

## Packages before publication

Package checks apply to developer previews as well as stable release candidates. Local package
verification must work before any Recite dependency is on crates.io or an asset store. Publishing
the packages is a release step, not a prerequisite for these checks.

| Package | Build and consumer check                                         | Scope                                                                                                                                                                          |
| ------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Bevy    | `scripts/check-bevy-package.sh`                                  | Prepare real `.crate` archives with temporary Cargo patches for unpublished dependencies, then run the packaged example in an external consumer using only extracted archives. |
| Godot   | `scripts/package-godot-addon.sh`; `scripts/check-godot-host.sh`  | Bundle the native addon and example; check clean installation, import, and replacement without changing authored project files.                                                |
| Unity   | `scripts/unity/build-upm.sh`; `scripts/unity/run-unity-tests.sh` | Bundle runtime/editor assemblies, Linux native plugin, and sample; check clean installation, import/reimport, legacy reference migration, and the selected player backend.     |

Rerun the package checks for the candidate. The
[recorded reproducibility observations](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/delivery-evidence.md#engine-package-reproducibility)
do not establish identical native binaries across compilers or system libraries. Native compiler
inputs must be pinned for a release build.

Package replacement checks exercise local development artifacts. They do not imply a previously
published version or a compatibility promise for every development snapshot. Store submission and
crates.io publication are outside these checks.
