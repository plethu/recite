# Prose checking decisions

[Tooling §14.1](spec/tooling.md#141-prose-checking) owns the behaviour. This note records why Recite
embeds a checker behind parser-owned prose regions, what that costs, and when to revisit it.

## Requirements

Authors want existing spelling and grammar tools on dialogue and choice prose without findings on
headers, IDs, speakers, conditions, effects, diverts or comments. Placeholders and markup must not
be flagged or break sentence context, fixes must not edit Recite syntax, and the same checks must
run in VS Code, Neovim, Zed, Writer and CI. Projects need a language, dictionaries for invented
names and suppression for intentional dialogue grammar. Checking must not need a network service.

## Candidates

| Route                                    | Grammar | Local and offline        | Recite-aware ranges                | Clients reached                | Licence and runtime            |
| ---------------------------------------- | ------- | ------------------------ | ---------------------------------- | ------------------------------ | ------------------------------ |
| Harper (`harper-core`) in Recite         | yes     | yes; nothing leaves host | yes, through prose regions         | every LSP client, CLI, Writer  | Apache-2.0; Rust, in process   |
| `harper-ls` beside the Recite server     | yes     | yes                      | no; checks whole file or comments  | per-editor setup               | Apache-2.0; second server      |
| LanguageTool through LTeX+ or HTTP       | yes     | only with local server   | no, unless Recite masks the text   | per-editor extension           | LGPL-2.1; Java, server process |
| cspell (Code Spell Checker)              | no      | yes                      | regex ignore patterns only         | VS Code; CLI through Node      | MIT; Node                      |
| Vale                                     | style   | yes                      | no Recite parser                   | CLI, some editors              | MIT; Go binary, style packs    |
| Editor spelling (Neovim `spell`, others) | no      | yes                      | Tree-sitter `@spell` captures only | Neovim; none in VS Code or Zed | host built-in                  |

LanguageTool has the broadest grammar coverage, but its public API sends prose off the machine and
its local mode needs Java and a running server. cspell and editor spelling miss grammar. Generic
servers cannot tell a speaker or stable ID from a word, so each would need its own Recite masking
and still disagree with the parser on nesting and incomplete files.

## Decision

`recite-parser` exposes prose regions and masked text with a byte-exact source map; `recite-prose`
owns `[prose]` settings, dictionaries, a small engine trait and the Harper adapter; the LSP, CLI and
Writer only display findings and apply the fixes they carry. Harper runs in process, offline and
deterministically, with English dialects (`en-US`, `en-GB`, `en-AU`, `en-CA`, `en-IN`). The engine
trait leaves room for another engine, such as a project-run LanguageTool server, without changing
regions, codes or clients.

The CLI and LSP enable the `prose` Cargo feature by default so official binaries check prose; a
packager can build without it, and checks then report `RECITE_PROSE103`. Editor checking stays off
until a project sets `[prose] enabled = true`, so the cost is opt-in at runtime.

## Costs

Measured on Linux x86_64 with `harper-core` 3.2.0, default features off and `concurrent` on:

- about 210 crates join the active graph (about 280 lockfile entries across platforms), including
  `burn` (part-of-speech tagging), `ammonia`, `pulldown-cmark` and `zip`. `log`, `serde_json` and
  `unicode-width` move to newer patch releases;
- a cold optimised build of that graph takes about two minutes on three cores, and a stripped probe
  binary is about 11 MB larger;
- building a checker takes about 0.45 s optimised and 3.5 s unoptimised, so clients keep one per
  project; checking a scene takes milliseconds after that;
- `colored`, `cssparser` and `dtoa-short` are MPL-2.0 and need per-crate exceptions in
  `.cargo/deny.toml`, like the existing Godot ones, before the licence check passes;
- `paste` (RUSTSEC-2024-0436) and `bincode` (RUSTSEC-2025-0141) are transitive unmaintained crates
  for the release dependency review;
- `harper-core` 3.2.0 pins `zip` 8.6, which has extraction and decoder vulnerability advisories
  (RUSTSEC-2026-0367 to 0369, 0429 and 0545 to 0548). Harper only uses `zip` in its Weirpack API,
  which Recite never calls, but the supply-chain gate fails on any vulnerability in the graph.
  Accepting these advisories is a maintainer decision and is not part of this change.

## Revisit when

- `harper-core` moves to `zip` 9.0.5 or later (required before the supply-chain gate passes without
  an exception), or drops `burn`;
- authors need a language Harper lacks; add another `ProseEngine` rather than a second region model;
- per-document checking dominates LSP publication latency; cache findings per region instead of per
  document text.
