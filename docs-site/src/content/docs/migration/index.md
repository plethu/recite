---
title: Migration
description: Transition guides for evaluating and moving existing dialogue content into Recite.
---

These guides help teams inventory existing dialogue content, decide what maps cleanly to Recite, and identify work that should remain manual. They are not promises of automatic full compatibility with another dialogue tool.

Recite is useful when the goal is deterministic runtime traversal, structured outputs, schema-checked effects, stable line and choice IDs, and validation outside the game engine. A migration should preserve author intent first, then reshape source into Recite's native model.

## Start here

1. Pick the closest source-tool guide.
2. Make a small migration slice: one conversation, timeline, node, knot, or data file.
3. Convert structure first: blocks, lines, choices, targets.
4. Move presentation and tags into metadata.
5. Move host-game actions into typed effects.
6. Move state checks into pure condition functions declared in schema.
7. Run validation before attempting engine integration.

```bash
recite validate dialogue/migrated/*.recite
recite compile --output build/dialogue.recitec dialogue/migrated/*.recite
```

## Guides

- [Dialogue System for Unity](/migration/dialogue-system-for-unity/)
- [Dialogue Manager](/migration/dialogue-manager/)
- [Dialogic](/migration/dialogic/)
- [Yarn Spinner](/migration/yarn-spinner/)
- [Ink](/migration/ink/)
- [JSON, CSV, and engine-native formats](/migration/json-csv-engine-native/)
- [Importer boundaries](/migration/importer-boundaries/)

## Recite target shape

```text
:: first_migrated_scene default
> scene_001@817fbf427f76e1268554 speaker=mentor
  Keep the example small until validation is boring.

? scene_ask_next@7f43c7ea106707ff048e
  What should I check next?
  -> checklist

:: checklist
> scene_002@a72ac9d10c2fe59c09bc speaker=mentor
  IDs, conditions, effects, metadata, then localisation.
! deferred mark_thread(first_migrated_scene, reviewed)
-> END
```

## What to preserve

- Speaker identity as structured speaker fields, not prose parsing.
- Choice identity as stable choice IDs.
- Line identity as stable line IDs.
- Branch targets as block references.
- Tags, portraits, emotions, camera hints, and source provenance as metadata.
- Game operations as explicit effects.
- State checks as pure condition calls.

## What to review manually

- Any source construct that executes code from dialogue.
- Random, saliency, visited-count, or history-sensitive behavior.
- Visual editor layout and engine object references.
- UI behavior such as typewriter timing, portraits, speaker panels, or input handling.
- Localisation IDs that were generated from source text and may drift after editing.

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Source Format](/reference/source-format/)
- [Schema](/reference/schema/)
- [Testing Dialogue](/guides/testing-dialogue/)
