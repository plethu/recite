# Changelog

Notable changes to Recite are documented here, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Recite is
pre-release: everything sits under Unreleased, there are no compatibility
promises yet, and the v0 compiled-asset mutability stance in
`docs/recite-production-spec.md` §12.2 applies until the first tagged release.

## [Unreleased]

Initial pre-release development of the deterministic dialogue toolchain:

- `recite-core` shared model: source AST, stable IDs, values, metadata,
  diagnostics, schema model, and the v0 compiled-asset tables and reader.
- `recite-parser` rowan-based lossless parser with recovery and lowering.
- `recite-compiler` validation, POT extraction, and deterministic v0
  MessagePack asset output with a compact JSON inspection form.
- `recite-runtime` deterministic, effect-free traversal with structured
  events, choice availability reasons, session snapshots, and save/load
  across blocking effects.
- `recite` CLI: `validate`, `compile`, `extract`, `check-ids`,
  `check-markup`, `check-metadata`, `validate-project`, `check-fresh`,
  `watch`, `run`, `trace`, and the interactive `play` TUI.
- `recite-lsp` authoring support: diagnostics, completion, hover,
  navigation/rename, ID code actions, and availability-aware authoring.
- Benchmark suite with synthetic and realistic fixtures, smoke regression
  policy, and benchmark reports under `docs/benchmark-reports/`.
- `recite-core` source AST and `recite-parser` lowering remain pre-1.0 APIs:
  authoring span and recovery fields must be constructed through the published
  constructors/builders and read through their accessors; direct struct
  literals are not a compatibility promise.
