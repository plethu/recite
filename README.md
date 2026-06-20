# Recite

> Dialogue tooling for narrative-driven games.

> **Pre-release.** APIs and on-disk formats are in flux while the v1 shape
> settles. External code contributions aren't open yet; see
> [`CONTRIBUTING.md`](CONTRIBUTING.md). Issues, questions, and design feedback
> are welcome, especially from authoring, localisation, runtime, or tooling
> work.

## What it is

Recite is a dialogue compiler, runtime, and toolchain for narrative-driven
games. It keeps authored conversation separate from game logic, then hands the
game structured output to observe and handle: lines, choices, and typed effect
requests. It's written in Rust, with Godot and Bevy as the first adapter targets
and an engine-independent dialogue contract underneath.

A scene is made of prose, speakers, choices, guards, fallthrough, localisation
IDs, and effect requests. Usually those are scattered across editor state, engine
scripts, and prose conventions; Recite keeps them in one plain-text format you
own and can inspect, validate, translate, and test.

What the core gives you:

- deterministic traversal across replay, save/load, and tests;
- pure conditions and typed effect requests, with no game-side mutation inside the runtime;
- stable IDs, localisation extraction, and validation before runtime;
- editor tooling that helps authors without taking over the workflow;
- structured runtime output, with headless tests, traces, and fixtures.

## Why

Dialogue tools such as Yarn Spinner, ink, and engine-native editors all make
different tradeoffs around scripting, editor ownership, localisation, and runtime
integration. Recite's tradeoff is strict boundaries: dialogue is validated before
runtime, conditions are pure queries, and effects are typed requests handled by
the game.

That gives narrative content the same kind of checks as code without making the
dialogue runtime responsible for game state. It is the tool I wanted while
authoring dialogue for my own game, where stable IDs, localisation, fixtures, and
typed integration mattered more than another embedded scripting layer.

Inspired by the narrative ambition of games like 1000xRESIST, Disco Elysium,
Citizen Sleeper, Planescape: Torment, TES III: Morrowind, and Pillars of
Eternity.

## Example

The source format reads like prose with rails:

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

## Repository

Canonical repo, issues, and pull requests live on
[Codeberg](https://codeberg.org/plethu/recite); the
[GitHub mirror](https://github.com/plethu/recite) is read-only.

## Documentation

The production spec,
[`docs/recite-production-spec.md`](docs/recite-production-spec.md), covers the
source format, schema, compiler, runtime, CLI, editor tooling, and the engine
adapters. The [`docs-site`](docs-site) Astro/Starlight build is the
game-developer-facing manual; Rustdoc remains the Rust API reference. It
currently produces local static output only, with no hosted deployment yet.

## AI usage

Agentic AI tools are part of the development workflow: drafting, implementation,
refactoring, and review. Direction, architecture, taste, and final review stay
human-led. Recite is a deterministic dialogue toolchain, and it isn't pursuing
AI-authored dialogue features.
