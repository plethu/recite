---
title: Choosing dialogue tools
description: Compare authoring and integration approaches, then test a small part of your own project.
template: splash
---

A dialogue tool affects how people write, how a game reads their work, and what must change when
either side changes. Choose around the work your team needs to do.

The comparisons here concern authoring and integration models. They are qualitative assessments, not
performance measurements or claims that the tools are interchangeable. Recite's v1 scope includes
the CLI, language server, supported text editors, standalone Writer, localisation workflow, and
engine adapters. Release preparation is still in progress; check
[packages and release status](/guides/distribution/) before planning adoption.

## When to consider Recite

Recite fits a project that wants plain-text source, deterministic traversal, and checks that can run
without the game engine. A project schema declares the conditions, effects, and metadata available
to writers. The runtime returns structured dialogue and effect requests; game code owns presentation
and state changes.

This gives writers and programmers a shared contract to review. It also leaves integration work with
the game team. You will need to provide condition answers, handle effects, display dialogue, and
save game state alongside the Recite session.

A general visual node editor is outside v1. If your workflow depends on one, or on a tool's existing
presentation and engine integrations, include that work in the cost of changing tools.

## Approaches worth comparing

### Ink

[Ink](https://www.inklestudios.com/ink/) combines a narrative language with Inky’s write-and-play
workflow. Examine how your story uses its flow and state features before planning a move.

Recite imports a small knot, divert, and choice subset. Weave, variables, expressions, and other
unsupported constructs need redesign. Read the [Ink migration notes](/migration/ink/).

### Yarn Spinner

Try [Yarn Spinner’s](https://docs.yarnspinner.dev/) script authoring and engine workflow with a
representative conversation from your game.

Recite’s importer handles documented plain nodes, lines, and static jumps. Options, commands,
variables, and history-dependent behaviour need manual work. Read the
[Yarn migration notes](/migration/yarn-spinner/).

### Dialogue Manager

[Dialogue Manager](https://dialogue.nathanhoad.net/) provides script authoring within Godot, native
translation support, and a stateless runtime that leaves game state with the host.

A move to Recite requires reviewing conditions and mutations against its schema and effect model.
Godot presentation and project setup stay separate. Read the
[Dialogue Manager migration notes](/migration/dialogue-manager/).

### Dialogic

[Dialogic](https://docs.dialogic.pro/) provides timeline events, visual and text editors,
characters, and presentation tools for Godot. Include the timeline behaviour and presentation you
use when assessing the work involved in changing tools.

Recite has no automatic Dialogic importer. The [Dialogic migration notes](/migration/dialogic/)
describe how to map dialogue and identify the parts that need manual work.

### Dialogue System for Unity

For
[Dialogue System for Unity](https://www.pixelcrushers.com/dialogue_system/manual2x/html/getting_started.html),
inventory the conversations, UI, sequencing, quests, and Unity integrations your project uses before
replacing dialogue traversal.

Recite does not import the Unity project or reproduce those systems. Read the
[Dialogue System migration notes](/migration/dialogue-system-for-unity/) to plan the integration
work.

### Clyde

[Clyde](https://github.com/viniciusgerevini/clyde) provides a dialogue language with branching,
translations, and game integration through variables and events. Compare those behaviours with
Recite’s conditions, typed effects, and localisation model.

Migration is manual. Recite v1 includes neither a Clyde importer nor a compatibility runtime. Read
the [Clyde migration notes](/migration/clyde/).

For passage-based work, the [Twee and Twine notes](/migration/twee/) describe the limited source
forms Recite can import. A published Twine game, its story format, and its presentation are separate
from that source subset.

## Try one conversation

Choose a conversation that contains the parts of your project most likely to make a move difficult:
a conditional choice, a game action, a save point, and some translated text. Keep the original
project available while you work.

Write down the behaviour you need to preserve. Convert the supported source, inspect every importer
report, and represent game operations as declared effects. Then run the scene against fixtures and
compare its outputs with the intended paths.

Test it in the engine too. A headless trace can check traversal and effect requests; it cannot
establish that your dialogue UI, input, timing, or save flow works for players.

Compare the cost of editing that scene, finding a mistake, reviewing a change, and integrating it
into the game. Those observations will be more useful than a feature count.

## Performance and release evidence

Recite’s [benchmark reference](/reference/benchmarks/) documents the maintained workloads and
regression policy. Smoke runs establish buildability and execution, not comparative speed or memory
use. A useful comparison needs the same workload, named versions and hardware, and a clear account
of what was measured.

Check the [distribution guide](/guides/distribution/) and relevant [engine adapter](/adapters/) for
package and platform evidence. Build and fixture results do not establish desktop accessibility,
engine compatibility or suitability for your project.
