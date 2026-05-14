# recite

> dialogue tooling for narrative-driven games.

> **pre-release.** apis, surfaces, and edges are in flux while the v1 shape
> settles. external code contributions are not being accepted yet — see
> [`CONTRIBUTING.md`](CONTRIBUTING.md). issues, questions, and design
> feedback are welcome, especially from real authoring, localisation,
> runtime, or tooling needs.

## remote

the canonical repository, issues, and pull requests live on
[codeberg](https://codeberg.org/plethu/recite). [github](https://github.com/plethu/recite) is a read-only mirror
for discoverability.

## overview

**recite** is a dialogue compiler, runtime, and toolchain for narrative-driven
games. it keeps authored conversation separate from game logic, then gives the
game structured output it can observe and handle.

it's written in rust for performance and portability. [godot](https://godotengine.org/)
and [bevy](https://bevy.org/) are the early adapter targets, but the core
dialogue contract is engine-independent.

the project is shaped by games with a lot of narrative ambition: 1000xRESIST,
disco elysium, citizen sleeper, planescape: torment, persona, and plenty of
others. recite is for work where the writing can be strange and sprawling, but
the dialogue system still needs to stay predictable.

there are already good dialogue tools, from yarn spinner and ink to unity and
godot-native options. recite is for the cases where i want the dialogue source,
validation, localisation ids, runtime traces, and game-facing effect requests to
share one small contract instead of being split across editor state, engine
scripts, and prose conventions.

that contract keeps the important pieces of a scene close together: prose,
speakers, choices, guards, fallthrough, localisation ids, and effect requests.
the goal is dialogue that is pleasant to write and still easy to inspect,
validate, translate, and test.

the core shape is deliberately small:

- deterministic traversal across replay, save/load, and tests;
- pure conditions and typed effect requests;
- stable ids, useful extraction paths, and validation before runtime;
- editor tooling that helps authors without owning the whole workflow;
- structured runtime output, headless tests, traces, and fixtures.

## quick example

the source format is meant to feel like prose with rails:

```text
# this is a referenceable block of prose
:: which_way default

# this is a line within that block, with a stable ID for localisation, and specifying metadata for the speaker
> alice_way_001 speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

# here's a choice, a branch in the dialogue, with the stable id answer_anywhere
? answer_anywhere
  I don't much care where.
  -> anywhere

# ...which is here!
:: anywhere

> cat_anywhere_001 speaker=cheshire_cat
  Then it doesn't matter which way you go.

# alright, our scene's done. let's set a quest thread flag, but that can happen once the scene is fully done, so let's defer it
! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

this sketch adapts a public-domain exchange from
[*alice's adventures in wonderland*](https://www.gutenberg.org/ebooks/11).
`mark_thread` is not executed by the dialogue runtime. it's a request for the
game to observe and handle as a schema-declared effect request.

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
