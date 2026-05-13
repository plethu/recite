# recite

> dialogue tooling for narrative-heavy games.

> **pre-release.** apis, surfaces, and edges are in flux while the v1 shape
> settles. external code contributions are not being accepted yet — see
> [`CONTRIBUTING.md`](CONTRIBUTING.md). issues, questions, and design
> feedback are welcome, especially from real authoring, localisation,
> runtime, or tooling needs.

recite is a dialogue compiler, runtime, and toolchain for games with a lot to
say: localisable, testable, predictable dialogue without tying it to one
engine, one asset store, or one editor-shaped way of thinking.

it is written in rust. godot and bevy are first-class adapter targets.

the basic idea: dialogue should describe the conversation. game logic should
stay in the game.

authored text is a small deterministic protocol. content names what narrative
systems need, and the runtime reports structured events back to the caller.

## why this exists

recite, and the games i develop, are inspired by games with narrative
ambition: disco elysium, citizen sleeper, persona, final fantasy, planescape:
torment, pillars of eternity, night in the woods, and larian's reactive
worlds.

the tooling for that kind of work has familiar frustrations:

- one-off mini-languages whose ideas don't travel beyond the tool;
- dialogue that calls directly into engine scripting;
- asset-store paywalls around basic features;
- localisation and content validation treated as afterthoughts.

it can leave you feeling like you're getting better at a specific product,
rather than building a portable way of thinking about narrative systems.

recite still has its own small language — intentionally. dialogue has
structure, and a format for dialogue should name that structure while keeping
it close to the text. the boundary stays simple: conditions are pure queries,
effects are typed requests, and the game stays the place where game logic
happens.

the contract is deliberately small:

- deterministic traversal across replay, save/load, and tests;
- stable ids, useful extraction paths, and validation before runtime;
- editor tooling that helps authors without owning the whole workflow;
- structured runtime output, headless tests, traces, and fixtures.

the writing can be strange, tender, sprawling, or funny. the machinery should
be boring in the best possible way.

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
format, schema, compiler, runtime, cli, editor tooling, and engine adapters
that keep the core dialogue contract intact.

## ai usage disclosure

agentic ai tools are part of the development workflow for drafting,
implementation assistance, refactoring, and review prompts. project direction,
architecture, taste, and final review remain human-led.

recite is not pursuing ai-authored dialogue features; it is a deterministic
dialogue toolchain.
