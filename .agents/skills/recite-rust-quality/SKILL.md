---
name: recite-rust-quality
description: Use for Recite Rust maintainability review: module boundaries, validation ownership, deterministic surfaces, diagnostics, FFI, and file-size triggers.
---

# Recite Rust Quality

This skill records Recite's maintainability and compatibility checks. Repository configurations and
the documented quality gate supply the shared Rust baseline.

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

- Use the affected subsystem contract for semantic ownership and public compatibility.
- Keep observable ordering explicit with source order or stable sorting.
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
