---
title: First Scene
description: Write, validate, compile, play, and headlessly run a minimal Recite scene.
---

This walkthrough takes one scene from source to a deterministic headless run using only the CLI. It
does not need an engine or adapter. Every command below is the real invocation against the file
shown.

## Write the scene

Create `dialogue/crossroads.recite`:

```text
:: which_way default

> speaker=alice @alice_way_001
  Would you tell me, please, which way I ought to go from here?

> speaker=cheshire_cat portrait=grin @cat_way_001
  That depends a good deal on where you want to get to.

? @answer_anywhere
  I don't much care where.
  -> anywhere

:: anywhere

> speaker=cheshire_cat @cat_anywhere_001
  Then it doesn't matter which way you go.

! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

Each line and choice header ends with its stable ID, such as `@alice_way_001`. Localisation and
saves key on that ID, so keep it when you edit the text. You can write a readable ID yourself, or
leave the header without one and run `recite id assign`; the LSP's missing-ID action does the same
in your editor, and clients may configure it on save. The `! deferred` statement is a typed effect
request. The runtime never executes it; it hands it to your game when the scene ends.

## Validate and compile

```bash
recite validate dialogue/crossroads.recite
recite compile dialogue/crossroads.recite -o crossroads.recitec
```

`validate` exits non-zero with structured diagnostics if the scene is malformed: wrong indentation,
duplicate IDs, or a divert to a missing block. `compile` writes the deterministic MessagePack asset
the runtime consumes.

## Play it interactively

```bash
recite play crossroads.recitec --block which_way
```

`play` is the writer's REPL: it renders lines and prompts in a TUI (or plain mode with `--ui plain`)
and lets you pick choices.

## Run it headlessly

Deterministic runs use a TOML fixture that answers every prompt. Create `fixture.toml`:

```toml
[choices]
"which_way" = "answer_anywhere"
```

```bash
recite run crossroads.recitec --block which_way --fixture fixture.toml
```

```text
line alice_way_001: Would you tell me, please, which way I ought to go from here?
line cat_way_001: That depends a good deal on where you want to get to.
prompt
  [1] answer_anywhere: I don't much care where.
selected choice answer_anywhere
line cat_anywhere_001: Then it doesn't matter which way you go.
end
deferred effects:
  mark_thread (alice_crossroads, direction_unsettled)
```

If the fixture is missing an answer for a prompt, `run` tells you the exact key it expected. `recite
trace` emits the same run as structured JSON for snapshot tests and CI.

## Extract localisation entries

```bash
recite extract dialogue/crossroads.recite
```

This produces gettext POT entries whose `msgctxt` is the stable ID. Prose edits keep that ID, but
gettext matches the source text too: refresh the catalogue and review retained translations after
changing prose.

From here: the [source format reference](/reference/source-format/) covers the full statement
vocabulary, and the [CLI reference](/reference/cli/) covers every command.
