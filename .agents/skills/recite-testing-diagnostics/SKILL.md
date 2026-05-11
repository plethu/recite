---
name: recite-testing-diagnostics
description: Use for Recite fixtures, snapshot-style tests, diagnostics, CLI validation commands, LSP behavior, and headless runtime test workflows.
---

# Recite Testing and Diagnostics

## Why

Recite should make dialogue testable without running a game. Tests and diagnostics should be deterministic, structured, and useful to both CLI and editor surfaces.

The repo is currently at the initial single-crate stage. CLI, LSP, fixture harnesses, and snapshot conventions are aspirational until implemented; use this skill to shape those surfaces as they land.

## Spec Routing

Read these sections of `docs/recite-production-spec.md` when relevant:

- CLI: §13
- LSP/editor: §14-15
- Tests: §17
- Diagnostics: §18
- Performance/benchmarks: §19
- v1 gate: §23

## Testing Principles

- Prefer fixture-driven tests for language behavior.
- Assert structured outputs instead of formatted prose when possible.
- Keep output ordering deterministic: source order or explicit sorting.
- Share fixture expectations between compiler, CLI, and LSP tests where practical.
- Runtime tests should run headlessly without engine runtime.
- Benchmarks are not part of the v1 acceptance gate unless the issue is explicitly milestone 6.

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

Before handoff:

- Tests are deterministic and do not depend on host paths or wall-clock time.
- Structured outputs are asserted directly where possible.
- Stable IDs are included where required.
- Effect order is asserted when effects are present.
- Locale fallback and markup preservation are tested when touched.
- `cargo test` or the relevant crate/workspace tests were run, or the blocker is stated.
