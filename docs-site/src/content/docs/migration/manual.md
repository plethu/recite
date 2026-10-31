---
title: Manual migrations
description: Map existing Unity, Godot and Clyde dialogue without an automatic importer.
---

Dialogue System for Unity, Dialogue Manager, Dialogic and Clyde have no automatic Recite importer.
Keep the original project and migrate one conversation first. Map entries to blocks, lines and
choices; preserve identifiers or record their old-to-new mapping. Put state queries in declared
conditions, game actions in typed effects, and descriptive presentation data in metadata.

Matching text does not establish matching traversal, timing or saves. Engine UI, resources and
integration code remain host-owned. Use the [source reference](/reference/source-format/) and
[testing workflow](/guides/testing-dialogue/) to validate each intended path, then check it in the
engine. The [migration overview](/migration/) owns the common adoption steps.

## Dialogue System for Unity

Conversations become blocks; actor/conversant fields become structured speakers and metadata.
Quest/custom fields need a decision between descriptive metadata and queried host state. Pure Lua
conditions need schema declarations; sequencer commands and Lua/trigger actions need effects with
explicit host timing and blocking acknowledgement. Reconcile existing quest state and save data with
host-owned game state and Recite snapshots.

Unity prefabs, components, triggers, camera/UI setup and third-party integrations do not transfer.
These mappings follow the Pixel Crushers 2.x
[getting-started](https://www.pixelcrushers.com/dialogue_system/manual2x/html/getting_started.html),
[quick-start](https://www.pixelcrushers.com/dialogue_system/manual2x/html/quick_start.html),
[condition](https://www.pixelcrushers.com/dialogue_system/manual2x/html/trigger_conditions.html) and
[sequencer](https://www.pixelcrushers.com/dialogue_system/manual2x/html/_sequencer_command_animation_8cs.html)
references.

### Before

```text
Conversation: Gate
Guard: Papers?
Player Response: Here they are.
Condition: Variable["HasPass"] == true
Sequence: SetActive(Gate,true)
```

### After

```text
:: gate_check default
> speaker=guard @gate_001
  Papers?

? requires=(has_pass(player)) @gate_show_pass
  Here they are.
  -> open_gate

:: open_gate
! blocking set_gate_open(town_gate)
> speaker=guard @gate_002
  Go on.
-> END
```

## Dialogue Manager

Cues become blocks, responses become choices, jumps become targets and tags become metadata. Replace
mutations with declared effects; game code owns the state change. Randomised lines need separate
candidates with explicit host selection. Inline waits, speed/text effects and concurrent lines need
metadata or host presentation behavior; mutation timing and randomisation are not implicitly
preserved. Balloons, scenes, plugin/autoload settings and node/method references stay in Godot.

See the official [overview](https://dialogue.nathanhoad.net/),
[basic dialogue](https://github.com/nathanhoad/godot_dialogue_manager/blob/main/docs/Basic_Dialogue.md)
and
[conditions/mutations](https://github.com/nathanhoad/godot_dialogue_manager/blob/main/docs/Conditions_Mutations.md)
references.

### Before

```text
~ start
Guide: Take this lantern.
$> Inventory.add_item("lantern")
- Thanks => end

~ end
Guide: Keep it lit.
=> END
```

### After

```text
:: start default
> speaker=guide @start_001
  Take this lantern.
! blocking grant_item(lantern)
? @start_thanks
  Thanks.
  -> end

:: end
> speaker=guide @end_001
  Keep it lit.
-> END
```

## Dialogic

Timeline labels/jumps become blocks/targets, character text becomes lines with `speaker=`, and
choices become Recite choices. Review timeline indentation and event grouping by hand. Character
staging, signal events and variable writes need decisions between metadata, effects and host state;
move signal listeners and actual calls into Godot code. Editor data, character resources, portrait
animation, scenes/autoloads and presentation remain outside Recite. Custom and built-in events have
no automatic compatibility layer.

See Dialogic 2's [timeline syntax](https://docs.dialogic.pro/timeline-text-syntax.html),
[variables](https://docs.dialogic.pro/variables.html) and
[signals](https://docs.dialogic.pro/dialogic-signals.html).

### Before

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

### After

```text
:: start default
> speaker=mira @lift_001
  The lift is offline.
? @lift_try_switch
  Try the switch.
  -> check

:: check
! blocking try_switch(power_panel)
:if power_online()
  > speaker=mira @lift_002
    That worked.
-> END
```

## Clyde

Migrate by hand against the native source and schema contracts. Review state-dependent traversal,
localisation, engine bindings and saved data separately; no runtime compatibility is promised.
