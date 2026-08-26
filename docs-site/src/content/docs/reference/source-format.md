---
title: Source Format
description: Author-facing reference for Recite source syntax.
---

Recite source is a line-oriented format for named dialogue blocks, localisable
lines, choices, conditions, and effect requests. Headers carry structured data.
Indented body text carries dialogue prose.

## Minimal scene

```text
:: gate_check default speaker=guard

> gate_001@ae703fc627755242ebc7 mood=impatient
  Papers?

:if has_pass(player)
  ? gate_show_pass@7f2eb1f55623952e6434
    Here they are.
    -> open_gate

:: open_gate speaker=guard

! blocking set_gate_open(town_gate)

> gate_002@7cfd0149209d8e2f5359
  Go on.

-> END
```

The example defines a default block, one structurally conditional choice, a
typed blocking effect request, and an `END` target.

## Statements

Recite uses a small statement vocabulary:

```text
:: block_name default speaker=guard  # block
> line_001@335096a87620c0ccbf2f speaker=guard             # line
? choice_001@63e10f410e48a6773297                         # choice
! blocking set_gate_open(town_gate)  # effect
-> open_gate                         # target
:if has_pass(player)                 # conditional branch
:else                                # else branch
:match gate_state(town_gate)         # enum dispatch
:case open                           # match arm
# comment                            # comment
```

Statement headers can carry fields and metadata. Statement bodies are indented
below the header. Blank lines inside a prose body are preserved as paragraph
breaks.

## Lines and speakers

Dialogue prose is not a quoted string. It is the indented body owned by a line
header:

```text
> gate_003@68e651c6d898d7f39f00 speaker=guard portrait=neutral
  Move along.
```

Speaker is structured data on the header, not text parsed from the body. Write
`speaker=guard` instead of `Guard: Move along.` so validation, localisation,
runtime output, and adapters all see the same speaker value.

## Block default speaker

A block can provide a default speaker for lines that do not name one:

```text
:: gate_check default speaker=guard

> gate_001@258bfad65f45bd3661bc
  Papers?

> gate_002@60ba4b9486ed20ce6735 speaker=captain
  Let them through.

> gate_003@b13991cde6630c24fe5b mood=impatient
  Well?
```

Here `gate_001` and `gate_003` inherit `guard`. `gate_002` explicitly overrides
the speaker with `captain`. The default speaker is speaker context only; it is
not general metadata inheritance.

## Metadata

Metadata entries are ordered and schema-validated. Repeated keys are allowed
when the project schema permits them.

```text
:: gate_check default speaker=guard location=town_gate

> gate_004@3b654fa7ef8715d05428 portrait=neutral caption="Door closes"
  Last warning.
```

Bare values are symbols or references, such as `portrait=neutral` and
`location=town_gate`. Quoted values are literal strings, such as
`caption="Door closes"`.

Ordinary block metadata describes the block itself. It is not inherited by
lines or choices. Put presentation cues, tags, or translator context on the
statement that owns them unless a documented statement field has specific
defaulting behavior.

## Choices and targets

Choices are localisable records with stable IDs, body text, metadata, and a
target:

```text
? gate_show_pass@191a4a3ecf68db47f96a tone=polite
  Here they are.
  -> open_gate

? gate_leave@a8f0209d81e33a0c858f
  I'll come back later.
  -> END
```

Targets jump to another block or end traversal with `END`. Unknown targets are
validation errors.

Use `requires=(...)` to keep a choice visible while making its availability
conditional, with `reason=...` for the schema-owned player-facing explanation.
Use `:if` when the choice should be structurally absent instead. The distinction
was settled in [#110](https://github.com/plethu/recite/issues/110),
[#111](https://github.com/plethu/recite/issues/111), and
[#112](https://github.com/plethu/recite/issues/112).

Choices may be nested under a line to model a prompt:

```text
> gate_prompt_001@15585b913a65e0892edb
  What do you show the guard?

  :if has_pass(player)
    ? gate_show_pass@456751c5fc90888fe004
      The signed pass.
      -> open_gate

  ? gate_back_away@b861d38fafdf6ceab3f0
    Nothing.
    -> END
```

## Conditions and effects

Conditions are pure queries over host-provided state. They decide whether
structural branches are included; they do not mutate the game:

```text
:if reputation_at_least(town_guard, 3)
  > gate_known_001@84cad7e66e4e1882cff4
    I know you. Go on.
:else
  > gate_unknown_001@b2fac71bc5030b065f3c
    Papers?
```

Use `:match` and `:case` for schema-declared enum state:

```text
:match gate_state(town_gate)
  :case open
    -> open_gate
  :case closed
    > gate_closed_001@a4bdc33d4c6caa81e951
      Not today.
  :case _
    -> END
```

Effects are typed requests emitted to the host. Recite reports the request; the
game decides what mutation, animation, sound, inventory change, or scene action
actually happens.

```text
! immediate play_sfx(gate_unlock)
! deferred mark_thread(gate_check, completed)
! blocking set_gate_open(town_gate)
```

Blocking effects require the host to acknowledge the effect before traversal
continues.

## Stable IDs

The current compiler contract requires every line and choice to have a stable
ID before compilation:

```text
> gate_005@79ab5f91ee4e42b3da54
  Keep moving.

? gate_ask_news@70c6b181e0398ce66cfb
  Any news from the road?
  -> road_news
```

The planned editor-assisted workflow is for LSP or on-save tooling to insert
missing line and choice IDs. That insertion workflow is not documented here as
available until the relevant editor tooling has shipped.

Once an ID is written, it is frozen. Tooling must not silently rewrite existing
IDs because localisation, fixtures, traces, and save-compatible dialogue state
depend on stable identifiers.

## Related docs

- [Production specification](https://github.com/plethu/recite/blob/main/docs/recite-production-spec.md)
- [CLI reference](/reference/cli/)
- [Migration overview](/migration/)
