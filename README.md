# recite

> dialogue tooling for narrative-heavy games.

recite is a dialogue compiler, runtime, and tooling project for games with a
lot to say.

it is for projects where dialogue needs to be localisable, testable,
predictable, and pleasant to work on without tying the dialogue model to one
engine's scripting language, one asset store, or one editor-shaped way of
thinking.

the basic idea is that dialogue should describe the conversation. game logic
should stay in the game.

recite treats authored text as a small deterministic protocol. content names
the things narrative systems need: lines, choices, conditions, metadata, stable
ids, and effect requests. the runtime walks that content predictably and
reports structured events back to the caller.

## why this exists

recite, and the games i develop, are inspired by my love for games with a lot
of narrative ambition: disco elysium, citizen sleeper, persona, final fantasy,
planescape: torment, pillars of eternity, night in the woods, and larian's big
reactive worlds.

the tooling for that kind of work can be frustrating in very practical ways:
one-off mini-languages whose ideas do not travel much beyond the tool, dialogue
that calls directly into engine scripting, asset-store paywalls, and
localisation or content validation treated as afterthoughts.

it can leave you feeling like you are getting better at a specific product,
rather than building a portable way of thinking about narrative systems.

recite still has its own small language. that is intentional. dialogue has
structure, and a format for dialogue should give that structure names while
keeping it close to the text.

the boundary is deliberately simple: conditions are pure queries. effects are
typed requests. the game remains the place where game logic happens.

recite's contract is deliberately small:

- traversal should be deterministic across replay, save/load, and tests;
- localisation should have stable ids and useful extraction paths;
- content mistakes should be caught before the game is running;
- editor tooling should help authors without owning the whole workflow;
- runtime output should be structured data, not prose conventions;
- headless tests, traces, and fixtures should be normal.

the writing can be strange, tender, sprawling, or funny. the machinery should be
boring in the best possible way.

## a small sketch

the source format is meant to feel like prose with rails:

```text
:: small_talk default

> rhea_001 speaker=rhea portrait=flat
  You again.

? ask_news if familiarity_gte(hazel, rhea, 3)
  Any news?
  -> news

? leave
  Never mind.
  -> END

:: news

> rhea_news_001 speaker=rhea
  The bridge is out. The gossip is not.

! deferred advance_thread(rhea_small_talk, news_heard)
-> END
```

in that example, `advance_thread` is not executed by the dialogue runtime. it
is a request for the game to handle.

## what recite is aiming for

the production direction is described in
[`docs/recite-production-spec.md`](docs/recite-production-spec.md).

| surface | purpose |
| --- | --- |
| source format | human-readable scenes with blocks, lines, choices, effects, metadata, and branches. |
| schema | project-owned contracts for conditions, effects, speakers, metadata, markup, and registries. |
| compiler | validation, stable id checks, deterministic compiled assets, and source maps. |
| runtime | engine-independent traversal with serialisable session state and structured events. |
| cli | validation, compilation, extraction, headless runs, traces, and project checks. |
| editor tooling | diagnostics, completions, renames, hovers, and authoring feedback. |
| adapters | engine integrations that keep the core dialogue contract intact. |

## status

> **pre-release**
>
> recite is early public work. expect changing apis, incomplete surfaces, and
> rough edges while the v1 shape settles.

## contributions

external code contributions are not being accepted until the v1 shape has
stabilised. see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the current policy.

issues, questions, and design feedback are welcome, especially when they come
from real authoring, localisation, runtime, or tooling needs.

## ai usage disclosure

agentic ai tools are part of the development workflow for drafting,
implementation assistance, refactoring passes, and review prompts.

project direction, architecture, taste, and final review remain human-led.
recite is not pursuing ai-authored dialogue features; it is a deterministic
dialogue toolchain.
