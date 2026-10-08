---
title: Authoring dialogue
description: Work from source edits through diagnostics, compilation and engine refresh.
template: splash
---

Use the [complete example](/examples/headless-cli/) for a project you can copy. Keep `.recite`
source, your schema producer, PO catalogues and runtime fixtures in version control. Compiled assets
and generated schema manifests are outputs; edit the owning source rather than patching their
contents.

## Source and schema

A block groups statements. A line or choice has an editable label and a stable anchor
(`label@11111111111111111111`). Conditions query host state through the schema; effects request
typed host work. Metadata carries presentation data without making the runtime execute it. The
[source reference](/reference/source-format/) defines the syntax.

In the example, `src/arrival.recite` shows all three concerns. The schema declares `object_seen` and
`trust_at_least`, the `open_interface` blocking effect, and the metadata used on the lines and
choices. The runtime fixture supplies their test values. Change the schema producer when your game
API changes, regenerate its manifest, then compile with `--schema`.

## Edit, check, preview

Use a configured [text editor](https://github.com/plethu/recite/tree/main/editors) for
language-server diagnostics and stable-ID actions, or open the source in Recite Writer. Keep
existing anchors when changing text. Fix source and schema diagnostics before testing the engine.

From the copied example:

```sh
recite validate src
recite check-ids src
recite compile src --schema schema/realistic.schema.json -o build/dialogue.recitec
recite validate-project .
recite run build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
recite trace build/dialogue.recitec --block arrival --fixture runtime-fixture.toml
```

Use [runtime fixtures](/guides/testing-dialogue/) for repeatable decisions and
[PO catalogues](/guides/localisation/) for translated preview. Source-only runs omit the fixture's
dialogue locale/catalogue configuration; enabling a locale makes catalogue resolution part of the
run.

## Watch and restart

`recite watch .` rebuilds saved inputs. A failed build reports diagnostics and retains the last
valid asset. Check `recite check-fresh .` after a successful build, then follow your engine's
[refresh instructions](/adapters/authoring/). All three companions retain the original revision in
an active session; the refreshed asset is used by a new session.

Save/load restores runtime traversal, not game state. Your game owns its state and effect handling.
Restore against compatible compiled content; stable IDs alone do not make a snapshot compatible with
an edited asset. Keep the original asset when finishing a saved session against its old revision.

The complete example's automated checks cover these command and runtime paths. They do not establish
keyboard, screen-reader, IME or interactive editor acceptance; those checks belong to the authoring
application's platform matrix.
