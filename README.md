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

it's written in rust. godot and bevy are first-class adapter targets.

the core principle is that dialogue should describe a conversation
independently of the game's logic.

authored text is a small deterministic protocol. content names what narrative
systems need, and the runtime reports structured events back to the caller.

## why this exists

recite — and the games i develop — are inspired by games with a lot of
narrative ambition: disco elysium, citizen sleeper, persona, final fantasy,
planescape: torment, pillars of eternity, night in the woods, and larian
studios' works.

the tooling for that kind of work has familiar frustrations:

- one-off mini-languages that only really make sense inside one editor;
- dialogue that calls directly into engine scripting;
- asset-store paywalls around basic features;
- localisation and content validation treated as afterthoughts.

this is the bet behind recite's small language: authoring dialogue works best
when the important pieces stay close together. yes, that means another dsl. i
did try not to. but prose, speakers, choices, guards, fallthrough, localisation
ids, and the requests the game needs to answer all want to live in the same
place. the boundary stays simple: conditions are pure queries, effects are
typed requests, and the game stays the place where game logic happens.

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
:: which_way default

> alice_way_001 speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

? answer_anywhere
  I don't much care where.
  -> anywhere

:: anywhere

> cat_anywhere_001 speaker=cheshire_cat
  Then it doesn't matter which way you go.

! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

this sketch adapts a public-domain exchange from
[*alice's adventures in wonderland*](https://www.gutenberg.org/ebooks/11).
`mark_thread` is not executed by the dialogue runtime. it is a request for the
game to handle.

## direction

the production direction is described in
[`docs/recite-production-spec.md`](docs/recite-production-spec.md): source
format, schema, compiler, runtime, cli, editor tooling, and engine adapters
that keep the core dialogue contract intact.

## ai usage disclosure

agentic ai tools are part of the development workflow for drafting,
implementation assistance, refactoring, and review prompts. project direction,
architecture, taste, and final review remain human-led.

recite is not pursuing ai-authored dialogue features; it's a deterministic
dialogue toolchain.
