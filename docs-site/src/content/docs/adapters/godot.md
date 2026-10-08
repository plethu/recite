---
title: Godot
description: Install the Godot 4 addon and run dialogue through native Resources and Nodes.
template: splash
---

Recite's Godot addon imports compiled `.recitec` files as `ReciteDialogueResource` and runs sessions
through `ReciteDialogueNode`. The Node returns structured operation results and emits ordered output
and error signals. Runtime effects are requests for game code to handle.

Build the installable addon from the repository with `scripts/package-godot-addon.sh`. It creates a
release GDExtension archive containing `addons/recite` and `examples/basic-dialogue`; copy the addon
into a Godot project and enable **Recite** under Project Settings → Plugins. See the
[addon installation and authoring guide](https://github.com/plethu/recite/tree/main/addons/recite)
and the
[basic dialogue example](https://github.com/plethu/recite/tree/main/examples/godot/basic-dialogue).

The example includes `recite.project.toml`: run `recite watch .` from that project to rebuild its
`.recitec` file after an edit. Check diagnostics and IDs, wait for the editor to reimport the
rebuilt file, then start a new session. The [authoring walkthrough](/adapters/authoring/) gives a
reproducible source edit and failed-build exercise. To upgrade, close Godot and replace the entire
`addons/recite` directory with the new archive's copy while leaving authored project files in place;
reopen Godot to import with the new extension.

The editor imports validated compiled assets. A valid edit becomes available to the next session,
while an active session retains its loaded revision. A failed reimport retains the previous valid
imported revision and exposes its structured error. `node.asset_state(asset)` reports active and
available canonical identities separately. Compiled-only assets report source and schema freshness
as unavailable.

`ReciteDialogueCatalogResource` accepts PO catalogues and survives Resource save/load. For native
schema declarations, save a `ReciteSchemaDeclarations` Resource with a stable `producer_id`, then
use **Project → Tools → Export Recite Schema**. The addon invokes the canonical Recite CLI exporter
and preserves the previous manifest if validation fails.

The verified host target is the official Godot 4.6.3 Linux x86_64 standard build. The
[verification scope](https://github.com/plethu/recite/blob/main/addons/recite/README.md#verification)
distinguishes Godot execution from Rust-only checks and unavailable capability gates.
