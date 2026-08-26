---
title: Importer Boundaries
description: What a Recite migration importer may preserve, flag, or leave to manual review.
---

Recite migration support is a boundary, not a compatibility promise. An importer may help inventory existing content and produce a first Recite draft, but the native Recite model remains blocks, structured line and choice IDs, pure conditions, schema-checked effects, metadata, localisation, and deterministic traversal.

Use this page when a transition guide says "importer boundary" or "migration report".

## Boundary model

| Existing content | Recite boundary |
| --- | --- |
| Dialogue text | Line bodies with stable line IDs |
| Branch choices | Choice records with stable choice IDs and targets |
| Node, cue, knot, timeline, or conversation names | Block IDs or metadata, depending on authoring intent |
| Tags, fields, portraits, mood labels, and line annotations | Ordered metadata with schema validation |
| External commands, mutations, sequencer commands, signals, and scripted actions | Typed effects emitted to the host |
| Variables and conditions | Pure condition calls declared in schema |
| Visual editor layout, engine scene links, and UI setup | Manual adapter or host-game work |

## Importer outputs

A useful importer should produce three artifacts:

- Draft Recite source for clean structural content.
- A migration report listing lossy or manual constructs with source locations.
- A schema todo list for condition functions, effect functions, speakers, registries, and metadata keys.

The importer should not silently guess semantics for host-game state, execute external code, or hide unsupported constructs.

## Tiny Recite target

```text
:: pier_intro default
> pier_001@4bc426982eed8fc98cee speaker=guide mood=calm
  The tide is coming in.

? pier_ask_boat@1fdd852a770b04574d58
  Ask about the boat.
  -> boat

! deferred mark_thread(pier_intro, seen)
-> END

:: boat
> pier_002@96a6b05cd8fb673bdc50 speaker=guide
  It is tied below the old signal lamp.
-> END
```

Next workflow:

```bash
recite validate dialogue/pier_intro.recite
recite compile --output build/dialogue.recitec dialogue/pier_intro.recite
```

## Not imported by default

- Runtime save files from the source tool.
- Editor graph positions, visual-node layout, and plugin UI configuration.
- Engine-specific scene objects, animations, signals, MonoBehaviours, nodes, prefabs, or resources.
- Undeclared script side effects.
- Tool-specific localisation database formats unless a project-specific converter maps them explicitly.

## Related docs

- [Source Format](/reference/source-format/)
- [Production specification](https://github.com/plethu/recite/blob/main/docs/recite-production-spec.md)
- [CLI](/reference/cli/)
