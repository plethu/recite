---
title: First Scene
description: Write, validate, compile, play, and headlessly run a minimal Recite scene.
---

This walkthrough takes one scene from source to a deterministic headless run
using only the CLI — no engine, no adapter. Every command below is the real
invocation against the file shown.

## Write the scene

Create `dialogue/crossroads.recite`:

```text
:: which_way default

> alice_way_001@7701ceab59d2adfa057a speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001@e26ae3e6834c21c1b716 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

? answer_anywhere@a6f46c2edbe8466b9bfd
  I don't much care where.
  -> anywhere

:: anywhere

> cat_anywhere_001@a11b4b64dceda892c08e speaker=cheshire_cat
  Then it doesn't matter which way you go.

! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

Each line and choice header is a `label@anchor` pair: the label
(`alice_way_001`) is editable context for you, and the 20-hex anchor is the
frozen ID that localisation and saves key on. When you author with the LSP you
write headers without anchors and an on-save code action fills them in; when
hand-writing, any unique 20-hex value works. The `! deferred` statement is a
typed effect request — the runtime never executes it, it hands it to your game
when the scene ends.

## Validate and compile

```bash
recite validate dialogue/crossroads.recite
recite compile dialogue/crossroads.recite -o crossroads.recitec
```

`validate` exits non-zero with structured diagnostics if the scene is
malformed — wrong indentation, duplicate anchors, a divert to a missing block.
`compile` writes the deterministic MessagePack asset the runtime consumes.

## Play it interactively

```bash
recite play crossroads.recitec --block which_way
```

`play` is the writer's REPL: it renders lines and prompts in a TUI (or plain
mode with `--ui plain`) and lets you pick choices.

## Run it headlessly

Deterministic runs use a TOML fixture that answers every prompt. Create
`fixture.toml`:

```toml
[choices]
"which_way" = "a6f46c2edbe8466b9bfd"
```

```bash
recite run crossroads.recitec --block which_way --fixture fixture.toml
```

```text
line 7701ceab59d2adfa057a: Would you tell me, please, which way I ought to go from here?
line e26ae3e6834c21c1b716: That depends a good deal on where you want to get to.
prompt which_way
  [1] a6f46c2edbe8466b9bfd: I don't much care where.
selected choice a6f46c2edbe8466b9bfd
line a11b4b64dceda892c08e: Then it doesn't matter which way you go.
end
deferred effects:
  mark_thread (alice_crossroads, direction_unsettled)
```

If the fixture is missing an answer for a prompt, `run` tells you the exact
key it expected. `recite trace` emits the same run as structured JSON for
snapshot tests and CI.

## Extract localisation entries

```bash
recite extract dialogue/crossroads.recite
```

This produces gettext POT entries whose `msgctxt` is the anchor — so editing
prose or labels later never invalidates a translation.

From here: the
[source format reference](/reference/source-format/) covers the full statement
vocabulary, and the [CLI reference](/reference/cli/) covers every command.
