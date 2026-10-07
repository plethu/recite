---
title: Testing dialogue
description: Verify source, host decisions, effects and saved sessions without rendering a game.
template: splash
---

Use a runtime fixture to make the game's answers explicit. In the
[complete example](/examples/headless-cli/), `runtime-fixture.toml` records boolean and enum
condition results, selected choice anchors, and automatic blocking-effect acknowledgement. It also
selects the test PO catalogue.

```sh
recite validate src
recite compile src --schema schema/realistic.schema.json -o build/dialogue.recitec
recite run build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite trace build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite check-fresh .
```

Use choice anchors in fixtures so editing the displayed text does not change the selection. Supply
every condition and choice needed by the path. A fixture that cannot answer a prompt fails rather
than inventing a game decision.

Trace records let you assert lines, choices, metadata, effect order and termination. Compare
structured fields or deterministic JSON, rather than terminal layout. The sample's smoke check runs
the same trace twice and requires equality. Add fixtures for different answers when they exercise
materially different behavior.

For save/load, test the engine or runtime session API. Save at a choice prompt and at a blocking
effect, restore against the original compiled revision, and verify that the same pending effect is
re-emitted. The game must acknowledge the right request and handle its own persistence. The Bevy
workflow test covers the blocking boundary and refresh behavior using the same source project.

## CI entry points

From the Recite checkout:

```sh
mise exec -- just engines workflow
cargo test --locked -p recite-import
cargo test --locked -p recite-cli --test migration
```

`just engines workflow` creates a fresh copy and verifies source/schema checks, extraction,
run/trace, stale-build recovery and Bevy save/refresh. The repository's normal project gate includes
it. A project's own CI can run the documented `recite` commands after building the CLI at a pinned
revision.

Use the existing [package checks](/guides/distribution/) for installation and upgrade evidence.
Rendering, input feel, assistive technology and editor interaction require their own host checks; a
passing headless trace does not cover them.
