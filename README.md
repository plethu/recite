# recite

> dialogue tooling for narrative-heavy games.

recite is a dialogue compiler, runtime, and tooling project for games with a
lot to say: localisable, testable, predictable dialogue without tying the model
to one engine's scripting language, one asset store, or one editor-shaped way of
thinking.

the basic idea is that dialogue should describe the conversation. game logic
should stay in the game.

recite treats authored text as a small deterministic protocol. content names
the things narrative systems need, and the runtime reports structured events
back to the caller.

## why this exists

recite, and the games i develop, are inspired by my love for games with a lot
of narrative ambition: disco elysium, citizen sleeper, persona, final fantasy,
planescape: torment, pillars of eternity, night in the woods, and larian's big
reactive worlds.

the tooling for that kind of work can be frustrating in very practical ways:
one-off mini-languages whose ideas do not travel much beyond the tool, dialogue
that calls directly into engine scripting, asset-store paywalls, and
localisation or content validation treated as afterthoughts. it can leave you
feeling like you are getting better at a specific product, rather than building
a portable way of thinking about narrative systems.

recite still has its own small language. that is intentional. dialogue has
structure, and a format for dialogue should give that structure names while
keeping it close to the text. the boundary is deliberately simple: conditions
are pure queries. effects are typed requests. the game remains the place where
game logic happens.

recite's contract is deliberately small:

- deterministic traversal across replay, save/load, and tests;
- stable ids, useful extraction paths, and validation before runtime;
- editor tooling that helps authors without owning the whole workflow;
- structured runtime output, headless tests, traces, and fixtures.

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

:: news

> rhea_news_001 speaker=rhea
  The bridge is out. The gossip is not.

! deferred advance_thread(rhea_small_talk, news_heard)
-> END
```

in that example, `advance_thread` is not executed by the dialogue runtime. it
is a request for the game to handle.

## direction

the production direction is described in
[`docs/recite-production-spec.md`](docs/recite-production-spec.md): source
format, schema, compiler, runtime, cli, editor tooling, and engine adapters that
keep the core dialogue contract intact.

## status

> **pre-release**
>
> recite is early public work. expect changing apis, incomplete surfaces, and
> rough edges while the v1 shape settles.

external code contributions are not being accepted until the v1 shape has
stabilised. see [`CONTRIBUTING.md`](CONTRIBUTING.md) for the current policy.

issues, questions, and design feedback are welcome, especially when they come
from real authoring, localisation, runtime, or tooling needs.

## ai usage disclosure

agentic ai tools are part of the development workflow for drafting,
implementation assistance, refactoring, and review prompts. project direction,
architecture, taste, and final review remain human-led.

recite is not pursuing ai-authored dialogue features; it is a deterministic
dialogue toolchain.
