---
name: recite-testing-diagnostics
description: Use for Recite fixtures, snapshot-style tests, diagnostics, CLI validation commands, LSP behavior, and headless runtime test workflows.
---

# Recite Testing and Diagnostics

## Why

Recite should make dialogue testable without running a game. Tests and diagnostics should be deterministic, structured, and useful to compiler, runtime, CLI, and editor surfaces.

## Spec Routing

Read these sections of `docs/recite-production-spec.md` when relevant:

- CLI: §13
- LSP/editor: §14-15
- Tests: §17
- Diagnostics: §18
- Performance/benchmarks: §19
- v1 gate: §23

## Testing Principles

| Concern | Expectation |
| --- | --- |
| Language behavior | Prefer fixture-driven tests and share fixture expectations between compiler, CLI, and LSP where practical. |
| Assertions | Assert structured outputs instead of formatted prose when possible. |
| Ordering | Keep output deterministic: source order or explicit sorting. |
| Runtime | Run headlessly without engine runtime. |
| Benchmarks | Exclude from the v1 acceptance gate unless the issue is explicitly milestone 6. |
| Large tests | Use `.agents/skills/recite-rust-quality/SKILL.md` to review cohesion and helper extraction. |

## Rust Test Organization

Use a small number of predictable test locations:

- Put externally observable crate behavior in `crates/<crate>/tests/**`. This is the default for parser, compiler, runtime, CLI, fixture, snapshot, diagnostic, and public model behavior.
- Put private unit tests in module-local `src/**/tests.rs` sidecars only when the test needs private internals that should not become public API.
- Do not put `#[test]` bodies inline in production source files.
- Do not use source-side `*_test.rs` or `*_tests.rs` files.
- Keep shared cross-crate fixtures under top-level `tests/support`.
- Run `.agents/skills/recite-codeberg-pm/scripts/check-test-organization.sh` before handoff when test files move or new tests are added.

## Fixture Shape Example

Use a fixture layout like this once the harness exists:

```text
fixtures/
  parser/
    block_headers.recite
    block_headers.expected.ron
    block_headers.diagnostics.ron
  runtime/
    blocking_effect.recite
    blocking_effect.trace.ron
```

A fixture should include:

- Source input.
- Expected structured output or diagnostic.
- Stable IDs where required.
- Effect order when effects are present.
- Locale fallback or markup preservation when relevant.

## Diagnostic Expectation Example

```ron
Diagnostic(
  code: "mixed-indent",
  severity: Error,
  span: Span(file: "dialogue/example.recite", start: 42, end: 46),
  message: "mixed indentation inside block body",
)
```

Diagnostics should be stable enough for snapshots. Avoid messages that include nondeterministic ordering, host paths, or debug-only formatting.

## Runtime Trace Assertion Example

```rust
#[test]
fn deferred_effects_are_source_ordered() {
    let trace = run_fixture("effects/deferred_order.recite");

    assert_eq!(
        trace.deferred_effects.iter().map(|effect| &effect.name).collect::<Vec<_>>(),
        ["advance_thread", "record_relationship_interaction"]
    );
}
```

## CLI and LSP Guidance

When these surfaces exist:

- CLI validation should emit machine-readable diagnostics where possible.
- LSP diagnostics should use the same diagnostic codes as compiler/CLI validation.
- LSP code actions should preserve stable IDs and avoid rewriting existing IDs.
- CLI and LSP tests should reuse fixture inputs rather than maintaining divergent examples.

## Quality Gate

Before handoff, state blockers or confirm:

| Concern | Required check |
| --- | --- |
| Determinism | Tests avoid host paths and wall-clock time. |
| Structured output | Direct assertions are used where possible. |
| Stable IDs/effects/locales | Stable IDs, effect order, locale fallback, and markup preservation are asserted when touched. |
| Commands | `.agents/skills/recite-codeberg-pm/scripts/check-test-organization.sh`, `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings`. |
