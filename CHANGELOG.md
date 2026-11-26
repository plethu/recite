# Changelog

Notable changes to Recite are documented here, following
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Recite is pre-release: everything sits
under Unreleased. The
[pre-release compiled-format policy](docs/spec/build-cli.md#122-compiled-format) applies until the
first tagged release.

## [Unreleased]

Initial pre-release development of the dialogue compiler/runtime, authoring tools and engine
companions. [The README](README.md) introduces the current toolchain.

- `recite-core` source AST and `recite-parser` lowering remain pre-1.0 APIs: authoring span and
  recovery fields must be constructed through the published constructors/builders and read through
  their accessors; direct struct literals are not a compatibility promise.
- Saves survive dialogue patches. `recite_runtime::resume::resume_session` restores a save against a
  patched asset: it resumes exactly, in an unchanged or text-edited block, or relocated through the
  save's stable IDs, and otherwise returns a structured fallback that carries the saved block,
  deferred effects and pending blocking effect. Snapshot v1 is corrected to carry the resume anchor,
  block digests and complete effect requests, so development snapshots must be regenerated.
  `snapshot_session` and `encode_session_messagepack` now take the session's asset.
- A restored pending prompt is presented again on the next advance, as a pending blocking effect is
  re-emitted, so engines need not cache prompt presentation in their saves.
- Every adapter restores against the current asset and reports the outcome: Bevy `Restore` takes
  `ResumeOptions` and emits `Resumed` or `ResumeFallback`; Godot results gain `restore_outcome()`
  and `restore_with_options()`; the C ABI adds `recite_session_prepare_resume`,
  `recite_session_resume` and `RECITE_RESUME_LEVEL_*` (ABI 0.8); Unity adds
  `ReciteDialogueService.Resume` and the runner's `ResumeFallback` event. Exact-only restores now
  report a schema change as a save/load incompatibility.
- `recite save impact` classifies every save point of a release against a patched build (`--since` a
  git revision or `--baseline` an asset, `--check` for CI), `recite save inspect` reads a save
  without the game, and `recite watch --save-baseline` reports the same after each rebuild.
- A `:match` nested in another `:match` arm compiles to its own arm table. Previously the outer
  match took the inner match's arms and could run the wrong arm, so assets with nested matches must
  be recompiled.
- Validation rejects a repeated `:case <variant>` arm, which traversal never selects
  (`RECITE_VALIDATE052`), and any arm after `:case _`, including a second wildcard
  (`RECITE_VALIDATE053`): the wildcard must be the last arm. A `recite save impact --since` or
  `recite watch --save-baseline` release that contains such arms now fails to build; the report
  names that revision, and the shipped compiled asset still works as the baseline.
- With a schema, validation checks every `:match` against its scrutinee's enum: a `:case` for an
  undeclared variant is `RECITE_VALIDATE054`, with a suggestion when one is close, and a match
  without `:case _` that leaves declared variants uncovered is `RECITE_VALIDATE055`. The synthetic
  fixture generator now ends its matches with `:case _`.
