---
title: Dialogue System for Unity
description: Migration notes for Pixel Crushers Dialogue System for Unity projects.
---

Pixel Crushers' Dialogue System for Unity centers authoring around a Dialogue Manager, dialogue databases, conversations, dialogue entries, triggers, Lua conditions, and sequencer or event integrations. Recite does not replace the Unity scene, UI, save system, or third-party integration layer; it replaces the dialogue traversal contract when you want a portable, deterministic core.

Terminology checked against current official Pixel Crushers 2.x manual/API pages for [getting started](https://www.pixelcrushers.com/dialogue_system/manual2x/html/getting_started.html), [quick start](https://www.pixelcrushers.com/dialogue_system/manual2x/html/quick_start.html), [trigger conditions](https://www.pixelcrushers.com/dialogue_system/manual2x/html/trigger_conditions.html), and [sequencer commands](https://www.pixelcrushers.com/dialogue_system/manual2x/html/_sequencer_command_animation_8cs.html).

## Concept map

| Dialogue System for Unity | Recite |
| --- | --- |
| Dialogue database | Project source files plus schema |
| Conversation | Block or group of blocks |
| Dialogue entry text | Line body |
| Actor/conversant fields | Structured `speaker=` and metadata |
| Player response | Choice with stable choice ID |
| Lua condition | Pure condition function |
| Sequence, Lua event, or trigger-side operation | Typed effect request |
| Quest fields and custom fields | Schema-backed metadata or host state queried by conditions |

## Clean migrations

- Linear conversations with speaker text map directly to blocks and lines.
- Player responses map to choices with explicit targets.
- Actor, portrait, mood, bark category, and similar fields can become metadata when they are descriptive.
- Simple Lua conditions that only read state can become condition calls such as `has_quest(hero_ring)`.

## Lossy migrations

- Sequencer commands lose Unity-specific timing unless you model them as effects or adapter metadata.
- Dialogue System UI setup does not move into Recite.
- Quest state that is both stored and mutated by Dialogue System needs a host-owned state model.
- Database fields with project-specific meanings need a schema decision before import.

## Manual work

- Write effect declarations for operations that previously happened through sequencer commands, Lua events, or trigger components.
- Decide whether Unity remains the host adapter and how it will acknowledge blocking effects.
- Rebuild save/load around Recite session snapshots plus host game state.

## Not imported or replaced

- Dialogue Manager prefab setup, Unity components, colliders, scene triggers, cameras, UI prefabs, save-system components, and third-party integration packages.
- Lua execution from dialogue.
- Automatic parity with Dialogue System database behavior.

## Before

```text
Conversation: Gate
Guard: Papers?
Player Response: Here they are.
Condition: Variable["HasPass"] == true
Sequence: SetActive(Gate,true)
```

## After

```text
:: gate_check default
> gate_001@8f6939290fcd3122d120 speaker=guard
  Papers?

? gate_show_pass@646ec5b28069a5b31d62 if has_pass(player)
  Here they are.
  -> open_gate

:: open_gate
! blocking set_gate_open(town_gate)
> gate_002@bf3f381355fb355f3970 speaker=guard
  Go on.
-> END
```

Next workflow:

```bash
recite validate dialogue/gate_check.recite
recite compile --output build/dialogue.recitec dialogue/gate_check.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Engine Adapter Contract](https://github.com/plethu/recite/blob/main/docs/engine-adapter-contract.md)
- [Production specification](https://github.com/plethu/recite/blob/main/docs/recite-production-spec.md)
- [CLI](/reference/cli/)
