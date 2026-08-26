# recite

> Words are events, they do things, change things.
>
> — Ursula K. Le Guin, "Telling Is Listening," *The Wave in the Mind* (2004)

`recite` is a small dialogue language and toolchain for games. I started
building it while writing my own game: I wanted plain-text scenes in Git,
localisation IDs that survived nearby edits, and broken dialogue to fail before
I opened the engine.

> [!WARNING]
> Recite is pre-release (`0.0.1`). An old `recite` snapshot exists on crates.io;
> the current `recite-cli` and `recite-lsp` packages are not published. Install
> them from this repository for now. The language, compiled assets, and Rust
> APIs may change before v1. Code contributions are not open yet, but issues,
> questions, and design feedback are welcome.

## A scene

Scenes are plain text, organised into named blocks:

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
Wonderland*](https://www.gutenberg.org/ebooks/11). The text after each `@` is a
stable, author-visible ID used by localisation. `speaker` is the line's dedicated
speaker field; `portrait` remains metadata. `mark_thread` must be declared in
the project schema; the runtime queues it and returns it when the scene ends
without executing it.

Recite sits between scene files and the game engine. The compiler turns those
files into assets. The runtime asks the game for condition values and returns
lines, choices, and typed effect requests; presentation and game state remain
in game code. Given the same asset, session state, and condition answers,
traversal order is deterministic across tests, save/load, and replay.

Most authoring should happen in your editor. Today, `recite-lsp` reports
diagnostics and provides completion, hover, definition navigation, block
rename, and code actions for missing stable IDs through compatible LSP clients.
The packaged VS Code extension and documented Neovim setup are still in
progress.

`recite watch` rebuilds when project inputs change. Engine adapters own how a
running game imports or refreshes those assets. In CI, the CLI can validate and
compile without an engine, then run scenes against repeatable fixtures. You do
not need to start the game to find a malformed scene.

## Try it

Install the current CLI and language server from a checkout:

```sh
git clone https://github.com/plethu/recite
cd recite
cargo install --path crates/recite-cli
cargo install --path crates/recite-lsp
```

Start with the [first-scene
guide](docs-site/src/content/docs/getting-started/first-scene.md). The [install
guide](docs-site/src/content/docs/getting-started/install.md) also covers
installing directly from Git.

The [production spec](docs/recite-production-spec.md) is the detailed contract,
and the [v1 roadmap](docs/roadmap.md) records what is still missing.
[`CONTRIBUTING.md`](CONTRIBUTING.md) has the current contribution status and
project notes.

Licensed under `MIT OR Apache-2.0`.
