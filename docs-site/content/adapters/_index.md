---
title: Engine adapters
description: Use Recite dialogue in Bevy, Godot, or Unity.
---

Recite compiles dialogue to `.recitec` assets. The adapters load those assets, deliver lines and
choices, and pass effect requests to your game. Conditions query game state; game code carries out
effects.

Choose the [Bevy](/adapters/bevy/), [Godot](/adapters/godot/), or [Unity](/adapters/unity/) guide
for setup. The [authoring walkthrough](/adapters/authoring/) covers editing, diagnostics, watch
builds, and refreshing an asset while a dialogue is running.

All three adapters keep an active dialogue on the revision it started with. A successful asset
refresh is available to the next session. Compiled assets and saved sessions carry Recite
identities; engine paths and asset handles do not replace those compatibility checks.

Recite is preparing for its first release. Package checks run locally before publication. The
[workflow evidence](https://github.com/plethu/recite/blob/main/docs/engine-authoring-workflows.md)
records the tested engines, platforms, package checks, and limits.

For implementation details, see the
[companion architecture](https://github.com/plethu/recite/blob/main/docs/engine-companions-design.md),
[adapter contract](https://github.com/plethu/recite/blob/main/docs/engine-adapter-contract.md), and
[acceptance checklist](https://github.com/plethu/recite/blob/main/docs/adapter-acceptance-matrix.md).
