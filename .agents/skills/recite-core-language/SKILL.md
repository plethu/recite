---
name: recite-core-language
description: Use for Recite parser, AST, compiler, runtime, schema, effects, localisation ID, and deterministic dialogue semantics work.
---

# Recite Core Language

Use this overlay for changes to Recite's language and execution semantics. Load only the affected
contract chapter and subsections before implementation; the
[production specification](../../../docs/recite-production-spec.md) routes to their owners.

Use [product invariants](../../../docs/spec/product.md#2-core-invariants) and the affected subsystem
contract. Parser/lowering, validation, traversal, serialization and host-facing tooling keep
separate owners; a client must not grow its own validator.

## Blocking effects

When touching blocking effects, verify that:

- the runtime emits a structured effect event;
- traversal pauses until acknowledgement;
- serialised session state records the pending effect;
- deserialising and resuming re-emits the same effect ID; and
- acknowledging the wrong or missing effect returns a structured error.

Semantic changes require tests unless the issue is explicitly exploratory. Keep source order, stable
IDs, spans, diagnostics, serialisation, and public API compatibility visible in the review.
