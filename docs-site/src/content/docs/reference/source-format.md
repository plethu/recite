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

> gate_001 mood=impatient
  Papers?

:if has_pass(player)
  ? gate_show_pass
    Here they are.
    -> open_gate

:: open_gate speaker=guard

! blocking set_gate_open(town_gate)

> gate_002
  Go on.

-> END
```

The example defines a default block, one structurally conditional choice, a
typed blocking effect request, and an `END` target.

## Statements

Recite uses a small statement vocabulary:

```text
:: block_name default speaker=guard  # block
> line_001 speaker=guard             # line
? choice_001                         # choice
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
> gate_003 speaker=guard portrait=neutral
  Move along.
```

Speaker is structured data on the header, not text parsed from the body. Write
`speaker=guard` instead of `Guard: Move along.` so validation, localisation,
runtime output, and adapters all see the same speaker value.

## Block default speaker

A block can provide a default speaker for lines that do not name one:

```text
:: gate_check default speaker=guard

> gate_001
  Papers?

> gate_002 speaker=captain
  Let them through.

> gate_003 mood=impatient
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

> gate_004 portrait=neutral caption="Door closes"
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
? gate_show_pass tone=polite
  Here they are.
  -> open_gate

? gate_leave
  I'll come back later.
  -> END
```

Targets jump to another block or end traversal with `END`. Unknown targets are
validation errors.

Visible-but-unavailable choices are v1 scope and are tracked by
[#170](https://codeberg.org/plethu/recite/issues/170),
[#171](https://codeberg.org/plethu/recite/issues/171), and
[#172](https://codeberg.org/plethu/recite/issues/172). Until the final
availability syntax lands, use `:if` for structural omission instead of a
trailing choice `if`.

Choices may be nested under a line to model a prompt:

```text
> gate_prompt_001
  What do you show the guard?

  :if has_pass(player)
    ? gate_show_pass
      The signed pass.
      -> open_gate

  ? gate_back_away
    Nothing.
    -> END
```

## Conditions and effects

Conditions are pure queries over host-provided state. They decide whether
structural branches are included; they do not mutate the game:

```text
:if reputation_at_least(town_guard, 3)
  > gate_known_001
    I know you. Go on.
:else
  > gate_unknown_001
    Papers?
```

Use `:match` and `:case` for schema-declared enum state:

```text
:match gate_state(town_gate)
  :case open
    -> open_gate
  :case closed
    > gate_closed_001
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
> gate_005
  Keep moving.

? gate_ask_news
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

- [Production specification](https://codeberg.org/plethu/recite/src/branch/main/docs/recite-production-spec.md)
- [Schema reference](/reference/schema/)
- [CLI reference](/reference/cli/)
- [Authoring loop](/guides/authoring-loop/)
- [Localisation guide](/guides/localisation/)
- [Migration overview](/migration/)
