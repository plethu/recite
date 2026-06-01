---
name: recite-rust-quality
description: Use for Recite Rust code quality review, maintainability cleanup, module boundaries, DRY judgment, Rust best practice, validation ownership, visibility, and file-size review triggers.
---

# Recite Rust Quality

## Why

Recite's Rust code should stay easy to review, deterministic, and boring to extend. Use this skill for non-trivial Rust implementation, refactors, reviews, and any touched Rust file that crosses the file-size triggers below.

## Quick Audit

When reviewing uncommitted Rust changes, inspect the changed files and run a line-count pass over the touched surface:

```bash
git diff --name-only --diff-filter=ACMRT -- '*.rs' | xargs -r wc -l | sort -nr
```

When reviewing a committed branch, compare against the branch base:

```bash
git diff --name-only --diff-filter=ACMRT main...HEAD -- '*.rs' | xargs -r wc -l | sort -nr
```

For branch-wide cleanup or review, use tracked files:

```bash
git ls-files '*.rs' | xargs wc -l | sort -nr | head -50
```

Ignore generated files, lockfiles, docs, build output, and data/tag tables unless they also mix responsibilities.

## File-Size Triggers

Line count is a triage trigger, not an automatic split rule. The real question is whether a file owns multiple independently changing concerns.

| File kind | Cohesion check | Split rationale or follow-up |
| --- | ---: | ---: |
| Production Rust | >250 LOC | >400 LOC |
| Test/support Rust | >350 LOC | >500 LOC |

Accept a large file only when it is cohesive and splitting would make the code harder to understand. Split smaller files when responsibilities are mixed.

A clean Rust implementation review is not complete until every touched production Rust file over 250 LOC and every touched test/support Rust file over 350 LOC is listed with one of:

- Split now.
- Cohesive; keep as-is, with the reason.
- Follow-up needed, with the issue or handoff note.

## Architecture Checks

- Keep parser, AST/model, compiler/validation, runtime traversal, serialization, CLI/TUI, and LSP responsibilities separate.
- Put validation policy at the boundary that owns the invalid state: constructors, typed models, loader/lowerer, compiler validation, runtime asset checks, or a named future issue.
- Avoid widening public API, `pub(crate)`, or module visibility just to make a local implementation convenient.
- Keep deterministic ordering explicit with source order or stable sorting where output can be observed.
- Prefer structured types, enums, and diagnostics over string conventions callers must parse.
- Build diagnostics through the shared `recite-core` constructor (`Diagnostic::error`) and the per-crate code constants; do not re-create a module-local `diagnostic()`/`*_diagnostic()` helper. Codes are static and namespaced: validate them at compile time with `DiagnosticCode::new_static`. Select or group diagnostics by `DiagnosticCategory`, never by matching or duplicating raw code strings across crates.

## DRY Checks

- Remove repeated logic when it is already stable and shared by the same concept.
- Do not create abstractions only because two blocks look similar; wait for shared meaning, ownership, and test pressure.
- Extract repeated test setup into local helpers or support modules when it makes assertions clearer.
- Keep helper modules named for the responsibility they own, not for vague mechanics such as `utils` unless the surrounding code already uses that pattern.

## Rust Practice Checks

- Prefer small private functions and focused modules over long functions with many mode flags.
- Keep ownership clear; avoid needless clones, but do not contort simple code to avoid cheap clones on small values.
- Prefer typed errors/results and explicit variants for observable failure modes.
- Preserve source spans, diagnostic codes, stable IDs, and serialization compatibility when touching those surfaces.
- Add a dependency only when it (1) removes error-prone or voluminous local code, (2) does not weaken a product invariant — determinism, stable IDs/codes, serialization compatibility, MIT licensing — and (3) covers a boundary the project does not want to own. A crate being well-regarded or popular is not itself a reason; reject it when std or a small local path already suffices. Worked judgment: `thiserror` earns its place on the library error enums (it deletes hand-rolled `Display`/`Error`/`From`) but not on `CliError`, whose rendering the Fluent i18n table owns.

## Public API and Extensibility

- Do not grow a signature with stacked optional parameters or `_with_a_and_b` suffixes. At the third knob, take an options/resolution struct and keep a zero-config entry point (precedents: `LocaleResolution` behind `next_with`/`choose_with`, `DialogueSessionOptions` behind `start_scene_with_options`).
- Mark consumer-facing public enums and structs `#[non_exhaustive]` when they may gain variants or fields, so downstream code (e.g. the Bevy adapter) keeps compiling when one is added. Apply it to errors, events, and effect/condition kinds; do not apply it to internal compiled-row enums, where same-crate exhaustive matching is the intended contract and wire compatibility is governed by tag mapping and format version.

## Handoff

Before handoff, state:

- Size-triggered files and their cohesion or split rationale.
- Checks run, usually `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and any focused crate/test command.
