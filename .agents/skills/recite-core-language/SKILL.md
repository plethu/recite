---
name: recite-core-language
description: Use for Recite parser, AST, compiler, runtime, schema, effects, localisation ID, and deterministic dialogue semantics work.
---

# Recite Core Language Work

## Why

Core language work defines Recite's durable semantics. Prefer small changes that preserve deterministic behavior and leave game-specific meaning outside the core runtime.

The repo is early and not yet split into the production workspace crates. Apply this guidance to the current crate now, and to `recite-parser`, `recite-core`, `recite-compiler`, `recite-runtime`, and related crates as they land.

## Spec Routing

Read the relevant section of `docs/recite-production-spec.md` before implementation:

- Source format/parser: §5
- Conditions: §6
- Effects: §7
- Runtime: §8
- Localisation and stable IDs: §9
- Schema: §10
- Manifests/compiler: §11-12
- Milestones and v1 gate: §22-23

## Core Invariants

- Runtime traversal is deterministic.
- Runtime code does not perform game-side effects.
- Effects are typed requests emitted to the caller.
- Conditions are evaluated through caller-provided context.
- Source-backed diagnostics carry spans.
- Metadata preserves repeated keys and source order.
- Stable line and choice IDs are author-visible and should not be rewritten implicitly.
- Blocking effects pause traversal and must resume with the same effect ID after save/load.

## Implementation Guidance

- Keep parser, AST, compiler, and runtime responsibilities separate.
- Parser code should describe syntax and spans, not runtime policy.
- Compiler code should validate references, IDs, schema use, and deterministic compiled output.
- Runtime code should consume compiled structures and expose structured events.
- Avoid adding public API surface that the current issue does not need.
- Prefer explicit structured types over strings that callers must parse.

## Parser/AST Issue Example

Good acceptance criteria:

```markdown
## Acceptance Criteria
- Parses named blocks and default block markers.
- Preserves source spans for block names and malformed headers.
- Rejects mixed indentation inside a block body.
- Adds parser tests for valid and invalid examples.
```

## Runtime Test Shape Example

```rust
#[test]
fn traversal_is_deterministic() {
    let asset = fixture_asset("simple_choice");
    let first = run_trace(&asset, ["choice_a"]);
    let second = run_trace(&asset, ["choice_a"]);

    assert_eq!(first.events, second.events);
    assert_eq!(first.deferred_effects, second.deferred_effects);
}
```

## Blocking Effect Checklist

When touching blocking effects, verify:

- The runtime emits a structured effect event.
- Traversal pauses until acknowledgement.
- Serialized session state records the pending effect.
- Deserializing and resuming re-emits the same effect ID.
- Acknowledging the wrong or missing effect returns a structured error.

## Quality Gate

Before handoff:

- Acceptance criteria are implemented or explicitly called out.
- New public model types and constructors have a clear validation policy: invalid states are prevented, represented explicitly, or deliberately deferred to a named validation pass or issue.
- Public API changes are reviewed for long-term correctness, maintainability, extensibility, and preservation of Recite's core invariants, not only for immediate issue scope.
- Semantic changes include tests unless the issue is exploratory.
- Error paths are covered for malformed source or invalid compiled content.
- Source-backed errors include spans.
- Source-order behavior is asserted where meaningful.
- `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings` were run, or the blocker is stated.
