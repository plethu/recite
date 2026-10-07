---
name: recite-rust-quality
description: Use for Recite Rust maintainability review: module boundaries, validation ownership, deterministic surfaces, diagnostics, FFI, and file-size triggers.
---

# Recite Rust Quality

When available, load the global `rust-quality` skill for general Rust implementation and review.
This overlay records the Recite-specific maintainability and compatibility checks required in any
environment.

For optimisation requests, read `docs/profiling-and-optimisation.md` before choosing probes or
proposing added runtime complexity; it defines completion evidence for broad passes and focused
fixes.

## File-size review

Line count is a triage signal, not an automatic split rule. Inspect whether a file owns multiple
independently changing concerns, data groups, or test scenarios.

| File kind         | Scrutinise above | Split or follow up above |
| ----------------- | ---------------: | -----------------------: |
| Production Rust   |          250 LOC |                  400 LOC |
| Test/support Rust |          350 LOC |                  500 LOC |

For a touched file above the scrutiny threshold, record one of:

- split now;
- cohesive, with the reason and alternatives considered; or
- follow-up needed, with the issue or handoff note.

For a branch-wide pass, count tracked Rust files. Include staged and untracked hand-written files
when reviewing a working tree; ignore generated output, lockfiles, and build output. Do not dismiss
hand-written data/tag/catalog tables as “mostly data” without checking their ownership and update
pattern. Use `ast-grep` or an equivalent structural search when a large file has repeated patterns
that make an ownership split difficult to assess.

## Recite checks

- Keep parser, AST/model, compiler/validation, runtime traversal, serialisation, CLI/TUI, and LSP
  responsibilities separate.
- Put validation policy at the boundary that owns the invalid state: a constructor, typed model,
  loader/lowerer, compiler validation, runtime asset check, or named future issue.
- Keep deterministic ordering explicit with source order or stable sorting where output can be
  observed.
- Prefer structured types, enums, and diagnostics over string conventions that callers must parse.
- Build diagnostics through the shared `recite-core` constructor (`Diagnostic::error`) and per-crate
  code constants. Do not re-create a module-local diagnostic helper. Codes are static and
  namespaced; validate them with `DiagnosticCode::new_static` and select/group them by
  `DiagnosticCategory`, not duplicated raw strings.
- Preserve source spans, diagnostic codes, stable IDs, and serialisation compatibility when touching
  those surfaces.
- Do not grow public entry points through stacked optional parameters or `_with_a_and_b` variants.
  At the third configuration knob, introduce an options/resolution struct and retain a
  zero-configuration entry point; precedents include `LocaleResolution` and
  `DialogueSessionOptions`.
- Keep consumer-facing structs and enums that may grow `#[non_exhaustive]`, especially errors,
  events, and effect/condition kinds. Do not apply it to internal compiled-row enums, where
  same-crate exhaustive matching is intentional and wire compatibility is governed by format
  mapping/versioning.
- Before substantial custom infrastructure or a broad optimisation pass, compare maintained
  ecosystem alternatives against the code, adapters and tests Recite would still own. Judge net
  maintenance cost, not dependency count or fastest timing alone. A roughly 5% slowdown can be
  acceptable for a substantial simplification; preserve correctness and verify relevant workloads.
  Check maintenance, license and platform fit before adoption. For LSP work, start with
  `docs/lsp-dependency-decisions.md` and its reopening conditions rather than repeating settled
  spikes.
- Before expanding Recite's tooling language or runtime footprint, compare the existing Rust
  ownership with maintained alternatives suited to the concrete job. Include setup, tests, debugging
  and the maintainer's learning cost; existing editor dependencies do not justify expanding Node.
  "It is for CI" does not establish that another language or collection of scripts is the best
  owner.

## FFI surface

- `unsafe impl Send` for raw pointer wrappers at a C ABI boundary requires runtime enforcement, not
  only a prose contract. For Recite's cdylib session model, record the owner thread ID at session
  creation and reject session operations that fire callbacks from a different thread.
- Do not encode structured error categories into a free-form string carried through an opaque error
  type. Store the category as a typed thread-local or structured field and recover it from there.

## Handoff

State the size-triggered files and their cohesion/split decision. Run the repository's documented
gate (`mise run verify`) or name the focused checks and any blocker.

Inspect the final code and callers for ownership, names, indirection and retained experiment
scaffolding. Remove superseded code within scope before handoff; passing lints or satisfying
file-size thresholds does not complete that review.
