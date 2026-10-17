---
title: Complete CLI workflow
description: Copy the maintained project and validate, compile, localise, run and trace it.
---

This example uses the five-scene project in `fixtures/realistic/v1-pack`. It contains a schema,
stable line and choice IDs, conditions, three effect modes, metadata, an English PO catalogue and a
deterministic runtime fixture.

## Copy and build

From a Recite checkout, build the CLI and copy the project to a new directory. The commands below
use a temporary directory so you can experiment without editing the repository's fixtures.

```sh
cargo build --locked -p recite-cli
export PATH="$PWD/target/debug:$PATH"
work=$(mktemp -d)
cp -R fixtures/realistic/v1-pack "$work/dialogue"
cd "$work/dialogue"
mkdir build
recite validate src
recite check-ids src
recite check-metadata src --schema schema/realistic.schema.json
recite check-markup src --schema schema/realistic.schema.json
recite compile src --schema schema/realistic.schema.json -o build/dialogue.recitec
recite validate-project .
recite check-fresh .
```

`recite.project.toml` declares the schema, compiled asset, entry block and scene participants.
Project validation checks that the declared asset exists and is fresh, so compile before running it.
Individual source checks work before an asset exists.

## Extract and run

```sh
recite extract src --schema schema/realistic.schema.json -o dialogue.pot
recite run build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite trace build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
```

The POT contains stable contexts such as `11111111111111111111`. The runtime fixture selects choices
by those anchors, supplies condition results and acknowledges blocking effects. Its `[dialogue]`
section selects the included `en-US` catalogue. The run prints the LACUNA conversation; trace
returns the structured events and decisions. Running the same fixture again produces the same trace.

An effect in the trace is a request to the host. This headless fixture supplies acknowledgements for
testing; Recite does not perform the requested game work.

## Edit and rebuild

```sh
recite watch .
```

Wait for a successful build. In `src/arrival.recite`, replace `-> src/effects.recite::effects` with
`-> missing_block` and save. Watch reports the missing target and keeps the previous compiled bytes.
Restore the target, change `The case narrows` to `The case turns`, and save again. The next
successful build replaces the asset. Run `recite check-fresh .` in another terminal.

Keep the anchors after `@` when editing text. The [authoring guide](/guides/authoring-loop/) covers
editor diagnostics and [engine refresh](/adapters/authoring/).

## Repeat the check in CI

From the repository root:

```sh
mise exec -- just engines workflow
```

This copies the fixture, runs the commands above, compares two traces and checks watch
failure/recovery. It also runs the same project through a headless Bevy App: a blocking-effect
snapshot restores the pending request, a refresh keeps the active revision, and a new session sees
the changed text.

The
[Bevy workflow test](https://github.com/plethu/recite/blob/main/crates/recite-bevy/tests/workflow_project.rs)
is the host integration reference for this project. The packaged [Bevy](/adapters/bevy/),
[Godot](/adapters/godot/) and [Unity](/adapters/unity/) examples cover their respective setup and
package workflows. Interactive editor and accessibility checks remain separate from this headless
test.
