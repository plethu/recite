# Rust lint-suppression policy

Recite treats Clippy and compiler warnings as design signals. A suppression is
therefore an explicit, reviewable exception rather than a way to make a gate
quiet. `scripts/check-lint-suppressions.sh` inventories `#[allow(...)]` and
`#[expect(...)]` attributes in handwritten Rust and compares the checked-out
head with the supplied Git base.

## What the gate checks

The default invocation is diff-aware. An attribute is considered baseline when
its path, category, lint names, kind, target, and item/module scope still match
the base. Moving such an attribute as code shifts does not create a false
positive, but moving it to another file or category is new at the destination.
Adding a lint, changing an item into a broader scope, removing an existing
reason, or adding a new attribute is reported as an expanded/new suppression.
Narrowing a list of lints is retained in the inventory as a changed record;
its current reason and scope still have to satisfy the policy. Only an
unchanged baseline record bypasses current-reason validation. Matching never
crosses paths or categories, and lint-list changes must retain the same
structural owner.

Discovery is delegated to the pinned ast-grep Rust parser in
`scripts/lint_suppression_ast.py`. Its real syntax nodes provide attributes,
declaration ancestry, blocks, closures, macros, comments, and parse errors.
The helper keeps only a bounded interpreter for suppression metadata and
`cfg_attr`; it does not lex Rust declarations or infer ownership from
delimiters. Configured owners and their configured ancestors are deliberately
unstable, so conditional siblings cannot consume one another's baseline.
Named modules, items, impl headers, and use trees are matchable. Anonymous,
macro, closure, malformed, or otherwise ambiguous scopes are reported as
`owner=unstable` and never consume a baseline, which is intentionally
fail-closed. ast-grep is supplied by the repository's `maintainability` mise
environment. Its `ERROR` and incomplete-node results fail the file closed;
this is structural validity evidence, not a replacement for rustc semantics.

The gate does not claim to verify whether a lint exists or whether a reason is
true; Cargo/rustc and human review retain those responsibilities.

New or expanded handwritten production suppressions must:

- be item-scoped; crate- and module-wide production `#[allow]`/`#[expect]`
  attributes are rejected;
- contain a non-empty literal `reason = "..."` argument;
- keep the lint list narrow enough that a reviewer can identify the local
  ownership boundary.

Reason literals containing escapes are rejected conservatively; write the
human-readable rationale directly in the literal.

`expect` is still subject to the reason requirement. A production
`#[expect]` may be broad only when the compatibility boundary itself is the
thing being documented; a broad `#[allow]` is never the default escape hatch.

## Scoped exceptional categories

The path classification is deliberately visible in the inventory:

| Category | Scope | New reason requirement |
| --- | --- | --- |
| `tests`, `fixtures` | Test/support inputs | No mandatory reason; keep the exception local to the fixture. |
| `benchmarks` | Benchmark targets and support | No mandatory reason; benchmark-only APIs should not leak into production. |
| `generated` | Exact paths in `scripts/generated-rust-allowlist.txt` | Only allowlisted generated output is exempt; path/name conventions alone never exempt a file. |
| `ffi` | `recite-ffi` or an explicit `ffi` path | `reason = "ffi: ..."` describing the boundary/ownership contract. |
| `compatibility` | Compatibility-named paths or an adjacent marker | `reason = "compatibility: ..."` naming the preserved public contract. |

An adjacent marker is a single comment containing
`recite-lint-suppression: compatibility` or `recite-lint-suppression: ffi`.
The marker is recognized from ast-grep comment trivia, so the same text in a
string, raw string, or commented-out old attribute is not classification
evidence. It is only a classification hint; it is not semantic or
cryptographic proof.
Generated exemptions use the exact repository-owned allowlist, whose entries
must be relative `.rs` paths; `generated.rs`, `target/`, or a generated
directory name is not evidence by itself. Existing exceptions in any category
remain baseline debt and are not blanket failed by this gate. A compatibility
`#[expect]` may be module-wide only when its scoped `compatibility:` reason
documents the preserved boundary; `#[allow]` remains item-scoped.

## Reading and remediating the inventory

The output distinguishes `baseline`, `new`, `expanded`, `narrowed`, and
`reason-*` records. Each record includes `scope`, structural `owner`, category,
literal `reason`, `rationale` status (`missing`, `present`, or `scoped`), and
`baseline_status`, plus whether the structural owner is stable and matchable.
Use `--full` when reviewing the current debt inventory;
full mode is reporting-only and marks records as `current`. The normal range
is the pull request base to the actual source head, not a synthetic merge
commit. Owner keys preserve module/item ancestry and the structural impl/use
header, so similarly named declarations do not collide.

```text
scripts/check-lint-suppressions.sh origin/main HEAD
scripts/check-lint-suppressions.sh --full
```

The trusted pull-request policy supplies `--policy-revision` for repository
policy files. This keeps the generated allowlist owned by the already-reviewed
base checkout: changing the allowlist and adding a generated-looking file in
one pull request cannot grant that file an exemption. The allowlist change
becomes effective only after a later base review.

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
