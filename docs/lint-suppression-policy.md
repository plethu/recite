# Rust lint-suppression policy

Recite treats Clippy and compiler warnings as design signals. A suppression is
therefore an explicit, reviewable exception rather than a way to make a gate
quiet. `scripts/check-lint-suppressions.sh` inventories `#[allow(...)]` and
`#[expect(...)]` attributes in handwritten Rust and compares the checked-out
head with the supplied Git base.

## What the gate checks

The default invocation is diff-aware. An attribute is considered baseline when
its lint names, kind, and item/module scope still match the base. Moving such
an attribute as code shifts does not create a false positive. Adding a lint,
changing an item into a broader scope, removing an existing reason, or adding
a new attribute is reported as an expanded/new suppression. Narrowing a list
of lints is retained in the inventory but does not fail the gate.

The parser is a small deterministic Rust lexer. It handles multiline
attributes, multiple lints, nested delimiters, comments, ordinary strings,
character literals, and raw strings. It does not claim to verify Rust
semantics, whether a lint exists, or whether a reason is true; Cargo/rustc and
human review retain those responsibilities.

New or expanded handwritten production suppressions must:

- be item-scoped when they use `allow`; crate- and module-wide production
  `#[allow]`/`#![allow]` attributes are rejected;
- contain a non-empty literal `reason = "..."` argument;
- keep the lint list narrow enough that a reviewer can identify the local
  ownership boundary.

`expect` is still subject to the reason requirement. A production
`#[expect]` may be broad only when the compatibility boundary itself is the
thing being documented; a broad `#[allow]` is never the default escape hatch.

## Scoped exceptional categories

The path classification is deliberately visible in the inventory:

| Category | Scope | New reason requirement |
| --- | --- | --- |
| `tests`, `fixtures` | Test/support inputs | No mandatory reason; keep the exception local to the fixture. |
| `benchmarks` | Benchmark targets and support | No mandatory reason; benchmark-only APIs should not leak into production. |
| `generated` | Generated output | Not scanned/enforced; fix the generator or its checked-in template. |
| `ffi` | `recite-ffi` or an explicit `ffi` path | `reason = "ffi: ..."` describing the boundary/ownership contract. |
| `compatibility` | Compatibility-named paths or an adjacent marker | `reason = "compatibility: ..."` naming the preserved public contract. |

An adjacent marker is a single comment containing
`recite-lint-suppression: compatibility` or `recite-lint-suppression: ffi`.
It is only a classification hint; it is not semantic or cryptographic proof.
Existing exceptions in any category remain baseline debt and are not blanket
failed by this gate.

## Reading and remediating the inventory

The output distinguishes `baseline`, `new`, `expanded`, `narrowed`, and
`reason-*` records. Use `--full` when reviewing the current debt inventory;
full mode is reporting-only. The normal range is the pull request base to the
actual source head, not a synthetic merge commit:

```text
scripts/check-lint-suppressions.sh origin/main HEAD
scripts/check-lint-suppressions.sh --full
```

For a new production violation, first ask whether the warning indicates a
missing boundary or an overly large function. Prefer extracting a cohesive
helper or introducing a small options/context value. If the warning is a
genuine local exception, keep the attribute on the smallest item, list only
the required lint, and write the reason next to the code. FFI and compatibility
exceptions must use their scoped prefix. Existing baseline debt should be
reduced in focused cleanup work rather than hidden by a baseline refresh.

The complete local gate runs this check through `mise run verify`. CI's
maintainability lane supplies the pull request base and source-branch head
explicitly, so policy results remain reproducible offline after checkout.
