---
name: recite-testing-diagnostics
description: Use for Recite fixtures, snapshot-style tests, diagnostics, CLI validation commands, LSP behavior, and headless runtime test workflows.
---

# Recite Testing and Diagnostics

Load the affected contract chapter from the section map in `AGENTS.md`; `docs/spec/quality.md` owns
testing and performance policy. This overlay records Recite's fixture, diagnostic, and test-layout
conventions. Load the global `language-tooling` and `rust-quality` skills for general testing
guidance when they are available.

## Test placement

- Put externally observable crate behavior in `crates/<crate>/tests/**`. This is the default for
  parser, compiler, runtime, CLI, fixture, snapshot, diagnostic, and public-model behavior.
- Put private unit tests in module-local `src/**/tests.rs` sidecars only when a test needs private
  internals that should not become public API.
- Do not put `#[test]` bodies inline in production source files, or use source- side
  `*_test.rs`/`*_tests.rs` files.
- Keep shared cross-crate fixtures under top-level `tests/support`.
- Run `scripts/check-test-organization.sh` when test files move or new tests are added.

## Fixtures and snapshots

Keep shared `.recite` inputs under `fixtures/recite/`, split into `valid/` and `invalid/`; keep
schema inputs under `fixtures/schema/valid/` and `fixtures/schema/invalid/`. Store expectations as
`insta` snapshots under each crate's `tests/snapshots/`, using `tests/support/fixtures.rs` where
applicable. Do not add `.expected.ron` or `.diagnostics.ron` sidecars.

When relevant, fixture expectations must cover structured output or diagnostics, stable IDs, effect
order, locale fallback, and markup preservation. Keep snapshots deterministic: use stable diagnostic
codes and avoid host paths, wall-clock values, nondeterministic ordering, and debug-only formatting.

## Diagnostics and host tooling

- CLI validation should emit machine-readable diagnostics where possible.
- LSP diagnostics use the same diagnostic codes as compiler and CLI validation.
- LSP code actions preserve stable IDs and do not rewrite existing IDs.
- CLI and LSP tests reuse fixture inputs rather than maintaining divergent examples.
- Runtime tests execute headlessly without an engine and assert structured events, deterministic
  order, serialisation/recovery, and effects when those surfaces are involved.
- Benchmark and scale evidence support the serious-v1 release outcome (GitHub milestone 25). Issue
  #109 owns the release benchmark baseline. Require benchmark smoke for affected changes and
  complete CI runs as defined in production spec §19.8, including the existing paired LSP gate.
  Broader numeric budgets remain evidence until that baseline is established.

## Performance investigations

For performance requests, read `docs/profiling-and-optimisation.md` before choosing probes. Its
broad-pass workflow requires actual timing measurements, CPU and allocation discovery, and final
implementation evidence; smoke, estimated model size and RSS alone do not satisfy it. Use a narrow
hypothesis and focused checks for an individual fix rather than repeating the broad pass.
