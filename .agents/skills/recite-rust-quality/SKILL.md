---
name: recite-rust-quality
description: Use for Recite Rust maintainability review: module boundaries, validation ownership, deterministic surfaces, diagnostics, FFI, and file-size triggers.
---

# Recite Rust Quality

When available, load the global `rust-quality` skill for general Rust
implementation and review. This overlay records the Recite-specific
maintainability and compatibility checks required in any environment.

## File-size review

Line count is a triage signal, not an automatic split rule. Inspect whether a
file owns multiple independently changing concerns, data groups, or test
scenarios.

| File kind | Scrutinise above | Split or follow up above |
| --- | ---: | ---: |
| Production Rust | 250 LOC | 400 LOC |
| Test/support Rust | 350 LOC | 500 LOC |

For a touched file above the scrutiny threshold, record one of:

- split now;
- cohesive, with the reason and alternatives considered; or
- follow-up needed, with the issue or handoff note.

For a branch-wide pass, count tracked Rust files. Include staged and untracked
hand-written files when reviewing a working tree; ignore generated output,
lockfiles, and build output. Do not dismiss hand-written data/tag/catalog tables
as “mostly data” without checking their ownership and update pattern. Use
`ast-grep` or an equivalent structural search when a large file has repeated
patterns that make an ownership split difficult to assess.

## Recite checks

- Keep parser, AST/model, compiler/validation, runtime traversal, serialisation,
  CLI/TUI, and LSP responsibilities separate.
- Put validation policy at the boundary that owns the invalid state: a
  constructor, typed model, loader/lowerer, compiler validation, runtime asset
  check, or named future issue.
- Keep deterministic ordering explicit with source order or stable sorting where
  output can be observed.
- Prefer structured types, enums, and diagnostics over string conventions that
  callers must parse.
- Build diagnostics through the shared `recite-core` constructor
  (`Diagnostic::error`) and per-crate code constants. Do not re-create a
  module-local diagnostic helper. Codes are static and namespaced; validate them
  with `DiagnosticCode::new_static` and select/group them by
  `DiagnosticCategory`, not duplicated raw strings.
- Preserve source spans, diagnostic codes, stable IDs, and serialisation
  compatibility when touching those surfaces.
- Add a dependency only when it removes material local complexity, fits Recite's
  determinism and MIT licensing constraints, and crosses a boundary the project
  does not want to own.

## FFI surface

- `unsafe impl Send` for raw pointer wrappers at a C ABI boundary requires
  runtime enforcement, not only a prose contract. For Recite's cdylib session
  model, record the owner thread ID at session creation and reject session
  operations that fire callbacks from a different thread.
- Do not encode structured error categories into a free-form string carried
  through an opaque error type. Store the category as a typed thread-local or
  structured field and recover it from there.

## Handoff

State the size-triggered files and their cohesion/split decision. Run the
repository's documented gate (`mise run verify`) or name the focused checks and
any blocker.
