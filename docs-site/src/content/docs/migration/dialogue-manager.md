---
title: Dialogue Manager
description: Migration notes for Nathan Hoad's Dialogue Manager for Godot.
---

Dialogue Manager for Godot uses script-like dialogue files, cues, responses, conditions, mutations, tags, jumps, and a stateless data-provider runtime. Recite has a similar text-first migration path, but it separates pure conditions from emitted effects and requires stable line and choice IDs.

Terminology checked against the official Dialogue Manager site and repository docs for [project overview](https://dialogue.nathanhoad.net/), [basic dialogue](https://github.com/nathanhoad/godot_dialogue_manager/blob/main/docs/Basic_Dialogue.md), and [conditions and mutations](https://github.com/nathanhoad/godot_dialogue_manager/blob/main/docs/Conditions_Mutations.md).

## Concept map

| Dialogue Manager | Recite |
| --- | --- |
| Dialogue file | Recite source file |
| Cue (`~ start`) | Block (`:: start`) |
| Dialogue line | Line with stable ID |
| Response (`- text`) | Choice with stable ID |
| Jump (`=> cue`) | Target (`-> cue`) |
| Condition | Pure condition function |
| Mutation (`$>` / `do`) | Typed effect request |
| Tags (`[#tag]`) | Ordered metadata |
| Example balloon | Host UI, not Recite runtime |

## Clean migrations

- Cue-based branching maps cleanly to blocks and targets.
- Responses map cleanly to choices.
- Tags and tag values map to metadata keys.
- Conditions that only read state map to schema-declared condition functions.

## Lossy migrations

- Randomised lines need an explicit Recite design decision. Preserve candidates as separate blocks or choose in host code.
- Inline waits, speed changes, and text effects should become metadata or presentation effects.
- Concurrent lines need a project-specific representation, such as metadata or parallel host UI events.

## Manual work

- Replace mutations with effects and move actual state changes into Godot code.
- Add stable IDs to every migrated line and choice.
- Decide which Dialogue Manager tags become schema-backed metadata and which should be dropped.

## Not imported or replaced

- Balloons, Godot UI scenes, editor plugin settings, autoload configuration, node references, and GDScript/C# methods.
- Dialogue Manager runtime files or save assumptions.
- Automatic compatibility for mutation timing or randomisation.

## Before

```text
~ start
Guide: Take this lantern.
$> Inventory.add_item("lantern")
- Thanks => end

~ end
Guide: Keep it lit.
=> END
```

## After

```text
:: start default
> start_001@d37069f78b6ac6d910bf speaker=guide
  Take this lantern.
! blocking grant_item(lantern)
? start_thanks@64083accc1d568b8b3c5
  Thanks.
  -> end

:: end
> end_001@9d4a9f6974b38d8e2392 speaker=guide
  Keep it lit.
-> END
```

Next workflow:

```bash
recite validate dialogue/start.recite
recite compile --output build/dialogue.recitec dialogue/start.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Godot Adapter](/adapters/godot/)
- [Source Format](/reference/source-format/)
- [Testing Dialogue](/guides/testing-dialogue/)
