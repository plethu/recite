---
title: Source Format
description: Author-facing reference for Recite source syntax.
---

Recite source is a line-oriented format for named dialogue blocks, localisable lines, choices,
conditions, and effect requests. Headers carry structured data. Indented body text carries dialogue
prose.

## Minimal scene

```text
:: gate_check default speaker=guard

> mood=impatient @b1w0tg8
  Papers?

:if has_pass(player)
  ? @w388zqy
    Here they are.
    -> open_gate

:: open_gate speaker=guard

! blocking set_gate_open(town_gate)

> @xg4jb1d
  Go on.

-> END
```

The example defines a default block, one structurally conditional choice, a typed blocking effect
request, and an `END` target.

## Statements

Recite uses a small statement vocabulary:

```text
:: block_name default speaker=guard  # block
> speaker=guard             # line @a6p6x73
?                         # choice @m8tz49s
! blocking set_gate_open(town_gate)  # effect
-> open_gate                         # target
:if has_pass(player)                 # conditional branch
:else                                # else branch
:match gate_state(town_gate)         # enum dispatch
:case open                           # match arm
# comment                            # comment
```

Statement headers can carry fields and metadata. Statement bodies are indented below the header.
Blank lines inside a prose body are preserved as paragraph breaks.

## Lines and speakers

Dialogue prose is not a quoted string. It is the indented body owned by a line header:

```text
> speaker=guard portrait=neutral @qekwjc5
  Move along.
```

Speaker is structured data on the header, not text parsed from the body. Write `speaker=guard`
instead of `Guard: Move along.` so validation, localisation, runtime output, and adapters all see
the same speaker value.

## Block default speaker

A block can provide a default speaker for lines that do not name one:

```text
:: gate_check default speaker=guard

> @gnrhw2n
  Papers?

> speaker=captain @e9c132f
  Let them through.

> mood=impatient @kje99zw
  Well?
```

Here `gate_001` and `gate_003` inherit `guard`. `gate_002` explicitly overrides the speaker with
`captain`. The default speaker is speaker context only; it is not general metadata inheritance.

## Metadata

Metadata entries are ordered and schema-validated. Repeated keys are allowed when the project schema
permits them.

```text
:: gate_check default speaker=guard location=town_gate

> portrait=neutral caption="Door closes" @gate_004
  Last warning.
```

Bare values are symbols or references, such as `portrait=neutral` and `location=town_gate`. Quoted
values are literal strings, such as `caption="Door closes"`.

Ordinary block metadata describes the block itself. It is not inherited by lines or choices. Put
presentation cues, tags, or translator context on the statement that owns them unless a documented
statement field has specific defaulting behavior.

## Choices and targets

Choices are localisable records with stable IDs, body text, metadata, and a target:

```text
? tone=polite @njnwr3m
  Here they are.
  -> open_gate

? @gate_leave
  I'll come back later.
  -> END
```

Targets jump to another block or end traversal with `END`. Unknown targets are validation errors.

Use `requires=(...)` to keep a choice visible while making its availability conditional, with
`reason=...` for the schema-owned player-facing explanation. Use `:if` when the choice should be
structurally absent instead. Unavailable choices remain structured prompt items; hidden choices are
absent.

Choices may be nested under a line to model a prompt:

```text
> @gate_prompt_001
  What do you show the guard?

  :if has_pass(player)
    ? @hewxgyh
      The signed pass.
      -> open_gate

  ? @gate_back_away
    Nothing.
    -> END
```

## Conditions and effects

Conditions are pure queries over host-provided state. They decide whether structural branches are
included; they do not mutate the game:

```text
:if reputation_at_least(town_guard, 3)
  > @gate_known_001
    I know you. Go on.
:else
  > @gate_unknown_001
    Papers?
```

Use `:match` and `:case` for schema-declared enum state:

```text
:match gate_state(town_gate)
  :case open
    -> open_gate
  :case closed
    > @gate_closed_001
      Not today.
  :case _
    -> END
```

Effects are typed requests emitted to the host. Recite reports the request; the game decides what
mutation, animation, sound, inventory change, or scene action actually happens.

```text
! immediate play_sfx(gate_unlock)
! deferred mark_thread(gate_check, completed)
! blocking set_gate_open(town_gate)
```

Blocking effects require the host to acknowledge the effect before traversal continues.

## Stable IDs

The current compiler contract requires every line and choice to have a stable ID before compilation:

```text
> @gate_005
  Keep moving.

? @gate_ask_news
  Any news from the road?
  -> road_news
```

A line or choice header ends with one `@id` field after its metadata. An ID uses lowercase letters,
digits, `_`, `.` and `-`, and starts with a letter or `_`. Write a readable ID yourself, or leave it
out and let `recite id assign` or the LSP's missing-ID action append a generated seven-character ID.
Neither rewrites an existing ID. Changing an ID changes identity, and changed source text requires
catalogue refresh and translation review.

Once an ID is written, it is frozen. Tooling must not silently rewrite existing IDs because
localisation, fixtures, traces, and save-compatible dialogue state depend on stable identifiers. To
change one deliberately, use `recite id rename`, `recite id reassign` or your editor's rename on the
`@id` field: they update the header and the matching `msgctxt` entries in the project's catalogues
together.

## Glossary

Recite Writer names things for the people writing dialogue. The source format and the CLI name the
same things after their syntax. Each Writer term maps to one source or CLI term:

| Writer               | Source format and CLI | Notes                                                                                                      |
| -------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------- |
| scene                | a `.recite` file      | One file of blocks. Writer's sidebar lists scenes.                                                         |
| beat                 | block (`:: name`)     | One named block; a map card in Writer.                                                                     |
| line                 | line (`> … @id`)      | A localisable line of dialogue with an optional speaker.                                                   |
| reply                | choice (`? … @id`)    | An option the player picks; its `->` target is the reply's destination.                                    |
| Try scene, Run scene | `recite play`         | Writer's Try scene and the playground's Run scene both traverse the compiled scene, as `recite play` does. |
| problems             | diagnostics           | Writer's status bar counts the open scene's diagnostics.                                                   |

## Related docs

- [Production specification](https://github.com/plethu/recite/blob/main/docs/recite-production-spec.md)
- [CLI reference](/reference/cli/)
- [Migration overview](/migration/)
