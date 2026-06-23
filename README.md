# recite

> Words are events, they do things, change things.
>
> — Ursula K. Le Guin, "Telling Is Listening," *The Wave in the Mind* (2004)

`recite` is a dialogue compiler, runtime, and toolchain for narrative-driven
games. It keeps authored conversation separate from game logic and hands the
game structured output to observe and handle: lines, choices, and typed effect
requests. The core is engine-agnostic, with per-engine adapters layered on top.

> **Pre-release.** APIs and on-disk formats are in flux while v1 settles. Code
> contributions aren't open yet (see [`CONTRIBUTING.md`](CONTRIBUTING.md)), but
> issues, questions, and design feedback are welcome, especially from authoring,
> localisation, runtime, or tooling work.

## Where it fits

Reach for Recite when you author narrative dialogue and want it kept apart from
engine logic. A scene holds prose, speakers, choices, guards,
fallthrough, localisation IDs, and effect requests in one plain-text format you
can inspect, validate, translate, and test, instead of spreading them across
editor state and engine scripts.

The runtime is narrow. Traversal is deterministic across replay, save/load, and
tests; conditions are pure queries; effects are typed, schema-checked requests.
It never mutates game state or executes those effects; it emits them as
structured output for the game to handle, alongside lines and choices.
Validation catches malformed content before any engine runs.

## Example

```text
# A referenceable block of prose.
:: which_way default

# A line with a stable anchor for localisation, plus speaker metadata.
> alice_way_001@7701ceab59d2adfa057a speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001@e26ae3e6834c21c1b716 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

# A choice: a branch in the dialogue with a stable anchor.
? answer_anywhere@a6f46c2edbe8466b9bfd
  I don't much care where.
  -> anywhere

:: anywhere

> cat_anywhere_001@a11b4b64dceda892c08e speaker=cheshire_cat
  Then it doesn't matter which way you go.

# Scene's done. Flag a quest thread, deferred until the scene finishes.
! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

This adapts a public-domain exchange from [*Alice's Adventures in
Wonderland*](https://www.gutenberg.org/ebooks/11).

`mark_thread` is a schema-declared effect request for the game to observe and
handle. The runtime never executes it.

## Why it exists

Yarn Spinner, ink, and engine-native editors each make different tradeoffs
around scripting, editor ownership, localisation, and runtime integration.
Recite's bet is strict boundaries, so narrative content gets the same checks as
code without the dialogue runtime taking on game state.

I built it while authoring dialogue for my own game, where stable IDs,
localisation, fixtures, and typed integration mattered more than another
embedded scripting layer. Inspired by the narrative ambition of games like
1000xRESIST, Disco Elysium, Citizen Sleeper, Planescape: Torment, TES III:
Morrowind, and Pillars of Eternity.

## Repository

Canonical repo, issues, and pull requests live on
[Codeberg](https://codeberg.org/plethu/recite); the
[GitHub mirror](https://github.com/plethu/recite) is read-only.

## Documentation

The production spec,
[`docs/recite-production-spec.md`](docs/recite-production-spec.md), covers the
source format, schema, compiler, runtime, CLI, editor tooling, and engine
adapters. The [`docs-site`](docs-site) Starlight build is the developer-facing
manual (local output only for now); Rustdoc is the API reference.

## AI usage

AI tools assist development: drafting, implementation, and review. Direction,
architecture, and final review stay human-led. Recite does not pursue
AI-authored dialogue features.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
