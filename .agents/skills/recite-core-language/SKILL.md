---
name: recite-core-language
description: Use for Recite parser, AST, compiler, runtime, schema, effects, localisation ID, and deterministic dialogue semantics work.
---

# Recite Core Language

Use this overlay for changes to Recite's language and execution semantics. Load
the relevant section of `docs/recite-production-spec.md` before implementation;
the section map is in `AGENTS.md`. When available, load the global
`language-tooling` and `rust-quality` skills for general guidance.

## Recite invariants

- Conditions are evaluated through caller-provided context.
- Source-backed diagnostics carry spans.
- Metadata preserves repeated keys and source order.
- Blocking effects pause traversal and resume with the same effect ID after
  save/load.
- Runtime traversal remains deterministic, emits structured events, and never
  performs game-side effects.

## Ownership boundaries

- Parser code describes syntax and spans, not runtime policy.
- AST and model types represent source structure without performing execution.
- Compiler code validates references, IDs, schema use, and deterministic
  compiled output.
- Runtime code consumes compiled structures and exposes structured events.
- Keep parser, AST/model, compiler/validation, runtime traversal,
  serialisation, and host-facing tooling responsibilities separate.

## Blocking effects

When touching blocking effects, verify that:

- the runtime emits a structured effect event;
- traversal pauses until acknowledgement;
- serialised session state records the pending effect;
- deserialising and resuming re-emits the same effect ID; and
- acknowledging the wrong or missing effect returns a structured error.

Semantic changes require tests unless the issue is explicitly exploratory. Keep
source order, stable IDs, spans, diagnostics, serialisation, and public API
compatibility visible in the review.
