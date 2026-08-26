# recite

> Words are events, they do things, change things.
>
> — Ursula K. Le Guin, "Telling Is Listening," *The Wave in the Mind* (2004)

`recite` is a dialogue language and toolchain for games. You write scenes as
plain text, check them locally or in CI, compile them into assets, and feed
those assets to a small runtime. The runtime produces lines, choices, and
typed effect requests; the game decides how to present them and what those
effects mean.

> [!WARNING]
> Recite is pre-release (`0.0.1`) and is not published to crates.io. The source
> format, compiled assets, and public APIs may change before v1. External code
> contributions are not open yet, but issues, questions, and design feedback
> are welcome. See [`CONTRIBUTING.md`](CONTRIBUTING.md) and the
> [current roadmap](docs/roadmap.md).

## Source

Scenes are organised into named blocks. Lines and choices carry stable,
author-visible IDs, while speaker data, metadata, conditions, and effects stay
structured rather than being inferred from prose.

```text
:: which_way default

> alice_way_001@7701ceab59d2adfa057a speaker=alice
  Would you tell me, please, which way I ought to go from here?

> cat_way_001@e26ae3e6834c21c1b716 speaker=cheshire_cat portrait=grin
  That depends a good deal on where you want to get to.

? answer_anywhere@a6f46c2edbe8466b9bfd
  I don't much care where.
  -> anywhere

:: anywhere

> cat_anywhere_001@a11b4b64dceda892c08e speaker=cheshire_cat
  Then it doesn't matter which way you go.

! deferred mark_thread(alice_crossroads, direction_unsettled)
-> END
```

This adapts a public-domain exchange from [*Alice's Adventures in
Wonderland*](https://www.gutenberg.org/ebooks/11). `mark_thread` is a
schema-declared effect request. The runtime returns it when the scene ends; it
does not execute it.

## Runtime boundary

The core runtime has no engine dependency or game-side effects. Traversal is
deterministic across replay, save/load, and tests. Conditions are caller-
provided pure queries. Effects are typed, schema-checked requests, and runtime
state is serialisable without game state. Validation can catch malformed
content before an engine runs.

Recite is designed for an IDE-first authoring workflow. Today, `recite-lsp`
provides diagnostics, completion, hover, navigation, rename, and stable-ID code
actions to compatible LSP clients; the VS Code extension and documented Neovim
setup are still in progress. The CLI provides validation, compilation,
localisation extraction, interactive `play`, and fixture-driven `run` and
`trace` commands for local checks and CI.

`recite watch` rebuilds compiled assets when project inputs change. It is an
authoring and build loop, not a universal mid-session hot-reload contract;
each engine adapter owns its refresh policy.

## Try it

The CLI currently installs from a checkout:

```sh
git clone https://github.com/plethu/recite
cd recite
cargo install --path crates/recite-cli
```

Install the language server as well when your editor supports LSP:

```sh
cargo install --path crates/recite-lsp
```

The [first-scene guide](docs-site/src/content/docs/getting-started/first-scene.md)
takes a scene through validation, compilation, interactive play, a headless
run, and localisation extraction. The
[install guide](docs-site/src/content/docs/getting-started/install.md) covers
installing from Git without a clone.

## Why I built it

I built Recite while authoring dialogue for my own game. I wanted prose that
could live in Git, IDs that would survive localisation, validation before
starting the game, and an explicit boundary between dialogue and game logic.
The language and toolchain follow from those requirements.

## Documentation

- [`docs/recite-production-spec.md`](docs/recite-production-spec.md) — source
  format, schema, compiler, runtime, CLI, LSP, and adapter contracts
- [`docs/roadmap.md`](docs/roadmap.md) — current v1 work and dependencies
- [`docs-site/`](docs-site) — the developer-facing manual source

Licensed under `MIT OR Apache-2.0`.
