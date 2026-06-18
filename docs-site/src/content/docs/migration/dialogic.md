---
title: Dialogic
description: Migration notes for Dialogic 2 timeline projects.
---

Dialogic 2 for Godot organizes dialogue as `.dtl` timelines made of events such as text, character, choice, condition, variable, label, jump, return, and do/call events. Recite migration should preserve the narrative flow and validation surface, while leaving Dialogic-specific editor and presentation behavior in Godot or adapter code.

Terminology checked against official Dialogic 2 docs for [timeline text syntax](https://docs.dialogic.pro/timeline-text-syntax.html), [variables](https://docs.dialogic.pro/variables.html), and [signals](https://docs.dialogic.pro/dialogic-signals.html).

## Concept map

| Dialogic | Recite |
| --- | --- |
| Timeline (`.dtl`) | Recite file or block group |
| Label | Block |
| Text event | Line |
| Choice event | Choice |
| Condition event | `:if` or choice `if` condition |
| Set Variable event | Effect or host-owned state update |
| Do/Call event | Effect request |
| Signal event | Effect request or metadata, depending on semantics |
| Character join/update/leave event | Host presentation effect or metadata |

## Clean migrations

- Text events with characters map to lines with `speaker=`.
- Labels and jumps map to blocks and targets.
- Choice events map to choices.
- Conditions that only read variables map to pure condition functions.

## Lossy migrations

- Character staging events do not have a native Recite UI equivalent.
- Timeline indentation and editor event blocks may need hand review after conversion.
- Dialogic variable writes are not Recite runtime mutations; they must become effects or host state changes.

## Manual work

- Decide which timeline events are narrative metadata and which are game effects.
- Move signal listeners into Godot adapter code.
- Recreate Dialogic UI and visual novel presentation outside Recite.

## Not imported or replaced

- Dialogic editor data, Godot scenes, autoload setup, character resources, portrait animation, timeline UI, and signal connections.
- Automatic support for every built-in or custom Dialogic event.
- Direct execution of `do` calls from Recite runtime.

## Before

```text
label Start
Mira: The lift is offline.
- Try the switch
    do PowerPanel.try_switch()
    jump Check

label Check
if {Power.online}:
    Mira: That worked.
```

## After

```text
:: start default
> lift_001@ad82d453e24d1d9d71d7 speaker=mira
  The lift is offline.
? lift_try_switch@727902127db19ed79d97
  Try the switch.
  -> check

:: check
! blocking try_switch(power_panel)
:if power_online()
  > lift_002@400cd2e9e42c14d8856f speaker=mira
    That worked.
-> END
```

Next workflow:

```bash
recite validate dialogue/lift.recite
recite compile --output build/dialogue.recitec dialogue/lift.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Engine Adapter Contract](https://codeberg.org/plethu/recite/src/branch/main/docs/engine-adapter-contract.md)
- [Production specification](https://codeberg.org/plethu/recite/src/branch/main/docs/recite-production-spec.md)
- [CLI](/reference/cli/)
