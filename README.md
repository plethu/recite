# Recite

> Dialogue tooling for narrative-driven games.

> **Pre-release.** APIs and on-disk formats are in flux while the v1 shape
> settles; external code contributions aren't open yet — see
> [`CONTRIBUTING.md`](CONTRIBUTING.md). Issues, questions, and design feedback
> are welcome, especially from real authoring, localisation, runtime, or tooling
> work.

## Repository

The canonical repository, issues, and pull requests live on
[Codeberg](https://codeberg.org/plethu/recite). The
[GitHub mirror](https://github.com/plethu/recite) is read-only, for
discoverability.

## What it is

Recite is a dialogue compiler, runtime, and toolchain for narrative-driven
games. It keeps authored conversation separate from game logic, then hands the
game structured output — lines, choices, and typed effect requests — to observe
and handle. It's written in Rust, with Godot and Bevy as the first adapter
targets and an engine-independent dialogue contract underneath.

A scene is made of prose, speakers, choices, guards, fallthrough, localisation
IDs, and effect requests. Usually those are scattered across editor state, engine
scripts, and prose conventions; Recite keeps them in one plain-text format you
own and can inspect, validate, translate, and test.

What the core gives you:

- deterministic traversal across replay, save/load, and tests;
- pure conditions and typed effect requests — never game-side mutation inside the runtime;
- stable IDs, localisation extraction, and validation before runtime;
- editor tooling that helps authors without owning the workflow;
- structured runtime output, with headless tests, traces, and fixtures.

## Why

There are already plenty of dialogue tools — Yarn Spinner, ink, and a variety of
engine-native options. Most of them ask you to accept some sort of trade-off: a
scripting language that can mutate game state; lock-in to a proprietary editor;
localisation that needs to be bolted on or worked around; and, most commonly,
absolutely no type safety or correctness guarantees to catch issues before
runtime. Most of these offerings predate what we now expect from software:
testing, validation, robust tooling.

Recite treats your narrative like any other part of your game: validated before
it runs, and integrated through typed requests rather than direct access to game
state. It's the tool I wanted while authoring dialogue for my own game (and tried
to hack together from what exists), so now I'm building it properly here.

Inspired by the narrative ambition on show in games like 1000xRESIST, Disco
Elysium, Citizen Sleeper, Planescape: Torment, TES III: Morrowind, and Pillars of
Eternity.

## Example

The source format reads like prose with rails:

```text
# A referenceable block of prose.
:: which_way default

# A line with a stable ID for localisation, plus speaker metadata.
> alice_way_001 speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

# A choice — a branch in the dialogue — with the stable ID answer_anywhere.
? answer_anywhere
  I don't much care where.
  -> anywhere

:: anywhere

> cat_anywhere_001 speaker=cheshire_cat
  Then it doesn't matter which way you go.

# Scene's done. Flag a quest thread, deferred until the scene finishes.
! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

Metadata values use source-aware scalar syntax: bare identifier-like tokens are
symbols (`portrait=grin`, `sfx=door_close`), while quoted values are string
literals (`caption="Door closes"`).

This adapts a public-domain exchange from [*Alice's Adventures in
Wonderland*](https://www.gutenberg.org/ebooks/11).

`mark_thread` is never executed by the runtime — it's a schema-declared effect
request for the game to observe and handle.

## Direction

The production spec —
[`docs/recite-production-spec.md`](docs/recite-production-spec.md) — covers the
source format, schema, compiler, runtime, CLI, editor tooling, and the engine
adapters that keep the core dialogue contract intact.

## AI usage

Agentic AI tools are part of the development workflow: drafting, implementation,
refactoring, and review. Direction, architecture, taste, and final review stay
human-led. Recite is a deterministic dialogue toolchain — it isn't pursuing
AI-authored dialogue features.
