---
title: Ink
description: Migration notes for ink projects.
---

ink content is organized around text, choices, knots, stitches, diverts, variables, functions, tags, weave, tunnels, threads, and lists. Recite can preserve a direct subset of explicit structure, but ink's recombining flow and built-in story-state behavior often need manual redesign.

Terminology checked against inkle's official ink documentation for [Writing with ink](https://github.com/inkle/ink/blob/master/Documentation/WritingWithInk.md) and [Running your ink](https://github.com/inkle/ink/blob/master/Documentation/RunningYourInk.md).

## Concept map

| ink | Recite |
| --- | --- |
| Knot | Block |
| Stitch | Block, usually with a prefixed ID |
| Text paragraph | Line body |
| Choice | Choice with stable choice ID |
| Divert | Target |
| Tag | Metadata |
| Global variable read | Pure condition call |
| Variable write or external function side effect | Effect or host state |
| Function used as pure query | Condition function |
| Tunnel, thread, weave, list logic | Manual design review |

## Clean migrations

- Simple knots, choices, and diverts map to blocks, choices, and targets.
- Tags map to metadata.
- External functions that only answer true/false questions can become condition functions.
- External functions that request game work can become effects.

## Lossy migrations

- Weave, gathers, tunnels, threads, sticky choices, list values, and visit counts do not have one automatic Recite equivalent.
- ink's runtime state and variable store do not become Recite session state.
- Suppressed choice text and output-mixed choice text need explicit Recite lines if the selected text should be echoed.

## Manual work

- Flatten or redesign recombining flow into explicit blocks.
- Audit every variable write and visited-count check.
- Decide whether ink tags are metadata, source provenance, or adapter presentation cues.
- Add stable IDs to every migrated line and choice.

## Not imported or replaced

- ink runtime JSON, Inky editor behavior, Unity integration code, save files, external function bindings, and story variable storage.
- Automatic full fidelity for weave, tunnels, threads, lists, or visit-count semantics.

## Before

```text
=== tower ===
Guard: The bell is silent.
* [Ring it]
  -> ring_bell

=== ring_bell ===
~ bell_rung = true
The sound crosses the yard.
-> END
```

## After

```text
:: tower default
> tower_001@72f711ddd5bfbaa55d89 speaker=guard
  The bell is silent.
? tower_ring_bell@70780f732f3234aae59c
  Ring it.
  -> ring_bell

:: ring_bell
! blocking ring_bell(courtyard)
! deferred mark_flag(bell_rung)
> tower_002@f936cfc122e738cf0547
  The sound crosses the yard.
-> END
```

Next workflow:

```bash
recite validate dialogue/tower.recite
recite compile --output build/dialogue.recitec dialogue/tower.recite
```

## Related docs

- [Importer Boundaries](/migration/importer-boundaries/)
- [Source Format](/reference/source-format/)
- [Testing Dialogue](/guides/testing-dialogue/)
- [CLI](/reference/cli/)
