---
title: Yarn Spinner
description: Migration notes for Yarn Spinner projects.
---

Yarn Spinner scripts are organized into nodes containing lines, options, commands, variables, flow control, tags, and metadata. Recite maps well from explicit nodes and options, but it is stricter about IDs, host effects, and schema validation.

Terminology checked against current official Yarn Spinner docs for [nodes and lines](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/lines-nodes-and-options), [flow control](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/flow-control), [commands](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/commands), [tags and metadata](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/tags-metadata), [line groups](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/line-groups), [node groups](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/node-groups), [saliency](https://docs.yarnspinner.dev/write-yarn-scripts/advanced-scripting/saliency), [once](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/once), and [functions](https://docs.yarnspinner.dev/write-yarn-scripts/scripting-fundamentals/functions).

## Concept map

| Yarn Spinner | Recite |
| --- | --- |
| Node title | Block ID |
| Line | Line body with stable line ID |
| Character prefix | Structured `speaker=` field |
| Option | Choice with stable choice ID |
| Jump or detour target | Target block |
| Command | Typed effect request |
| Variable condition | Pure condition call |
| Tags and line metadata | Ordered metadata |
| Line group or node group selection | Manual design or host selection policy |

## Clean migrations

- Node titles map to blocks.
- Basic lines and options map to lines and choices.
- Tags such as line IDs, speaker hints, portrait hints, and barks can become metadata.
- Commands that mean "tell the game to do something" can become effects.

## Lossy migrations

- Yarn variables and expressions do not automatically become Recite state.
- Line groups, node groups, saliency strategies, detours, `once`, and visit-tracking behavior need explicit design.
- Built-in command timing may differ after conversion.

## Manual work

- Decide whether Yarn line tags remain metadata, become Recite IDs, or are preserved as source provenance.
- Replace custom commands with effect declarations.
- Replace variable reads with schema-declared condition functions.
- Review every branch that depends on `once`, `visited()`, visit counts, or saliency selection.

## Not imported or replaced

- Yarn Spinner runtime, variable storage, Dialogue Runner setup, Unity/Godot presenters, command registration code, and editor extensions.
- Automatic compatibility for saliency, line groups, node groups, `once`, or visit-tracking logic.

## Before

```text
title: Dock
---
HarborMaster: Boat leaves at dawn. #mood:busy
-> Ask about cargo
    <<log_cargo_question>>
    <<jump Cargo>>
===
```

## After

```text
:: dock default
> dock_001 speaker=harbor_master mood=busy
  Boat leaves at dawn.
? dock_ask_cargo
  Ask about cargo.
  -> cargo

:: cargo
! immediate log_cargo_question()
> cargo_001 speaker=harbor_master
  Crates first, passengers second.
-> END
```

Next workflow:

```bash
recite validate dialogue/dock.recite
recite compile --output build/dialogue.recitec dialogue/dock.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Source Format](/reference/source-format/)
- [Schema](/reference/schema/)
- [Localisation](/guides/localisation/)
