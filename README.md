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

**recite** is a dialogue compiler, runtime, and toolchain for games that need robust tooling:
it's localisable, testable, and fully predictable. it isn't tied to one
engine, a particular proprietary asset, or one editor-shaped way of thinking.

it's written in rust for performance and portability between engines.
as of right now, [godot](https://godotengine.org/) and [bevy](https://bevy.org/) are the first-class adapter targets,
but depending on my availability i'll be looking to bring it to other widely used engines too.

**the core principle of the project** is that dialogue should describe a conversation, or an interactive piece of fiction,
completely independently of the game's actual logic.

authored text is a small deterministic protocol. content names what narrative
systems need, and the runtime reports structured events back to the caller for it to integrate with the game's systems.

## why?

recite—and the games i develop—are inspired by games with a lot of
narrative ambition: 1000xRESIST, disco elysium, citizen sleeper, planescape: torment, persona,
and dozens of other games that use video games to tell big stories.

the tooling for that kind of work, _especially_ for indie devs, has a couple big pain points:

- one-off mini-languages that only really make sense inside one editor, so tooling ecosystems are fragmented and learning resources are sparse;
- dialogue that calls directly into engine scripting;
- asset-store paywalls around basic features with a black box you can't shape into the tool you need;
- localisation and content validation treated as afterthoughts.

this is the bet behind recite's small language: authoring dialogue works best
when the important pieces stay close together. yes, that means another mini-language. i
did try not to, but collocating prose, speakers, choices, guards, fallthrough, localisation
ids, and the effect requests makes for it a lot easier to reason about what the outcome of a scene of dialogue will be.

the boundary stays simple, to avoid making this yet another scripting language: conditions are pure queries, effects are
typed requests, and the game stays the place where game logic happens. the contract is also deliberately small:

- deterministic traversal across replay, save/load, and tests;
- stable ids, useful extraction paths, and validation before runtime;
- editor tooling that helps authors without owning the whole workflow;
- structured runtime output, headless tests, traces, and fixtures.

the writing can be strange and sprawling without making the authoring experience reflect that.

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
