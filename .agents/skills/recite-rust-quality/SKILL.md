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
git diff --name-only -- '*.rs' | xargs -r wc -l | sort -nr
```

When reviewing a committed branch, compare against the branch base:

```bash
git diff --name-only main...HEAD -- '*.rs' | xargs -r wc -l | sort -nr
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

## Architecture Checks

- Keep parser, AST/model, compiler/validation, runtime traversal, serialization, CLI/TUI, and LSP responsibilities separate.
- Put validation policy at the boundary that owns the invalid state: constructors, typed models, loader/lowerer, compiler validation, runtime asset checks, or a named future issue.
- Avoid widening public API, `pub(crate)`, or module visibility just to make a local implementation convenient.
- Keep deterministic ordering explicit with source order or stable sorting where output can be observed.
- Prefer structured types, enums, and diagnostics over string conventions callers must parse.

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
- Add dependencies only when the repo does not already have a small, clear local or standard-library path.

## Handoff

Before handoff, state:

- Size-triggered files and their cohesion or split rationale.
- Checks run, usually `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, and any focused crate/test command.
