# Rust lint-suppression policy

Warnings are design signals. Prefer correcting the ownership or implementation before suppressing
one. A local exception needs the narrowest scope and a reason a reviewer can assess.

```sh
scripts/check-lint-suppressions.sh origin/main HEAD
scripts/check-lint-suppressions.sh --full
```

The first command checks changes against the base; `--full` reports current debt without failing it.
The complete local gate includes the check. The [checker](../scripts/check-lint-suppressions.sh),
[structural parser](../scripts/lint_suppression_ast.py) and their tests own matching and
classification.

## What the gate checks

New or expanded handwritten production suppressions must be item-scoped and have a non-empty literal
`reason`. Keep the lint list narrow. Reason literals with escapes are conservatively rejected; write
the rationale directly. Both `allow` and `expect` participate.

Unchanged baseline records remain debt. Moving across files, widening scope, adding lints or losing
a reason cannot silently inherit baseline status. Anonymous, conditional or ambiguous owners fail
closed. Macro token trees containing lint controls cannot establish a reviewed local scope and are
rejected in production. The gate uses pinned rustfmt and ast-grep; it does not validate whether a
lint exists or whether a reason is true. Rustc and human review retain those responsibilities.

## Scoped exceptional categories

Tests, fixtures and benchmarks may have local exceptions without production reason requirements.
Generated exclusions come only from the exact [allowlist](../scripts/generated-rust-allowlist.txt),
never a filename convention.

FFI reasons begin `ffi:` and name ownership at the boundary. Compatibility reasons begin
`compatibility:` and name the preserved public contract. An adjacent `recite-lint-suppression: ffi`
or `recite-lint-suppression: compatibility` comment can classify a boundary; it is not evidence that
its implementation is correct. Only a compatibility `expect` with a scoped reason may cover a
module. Production `allow` remains item-scoped.

## Reading and remediating the inventory

The report distinguishes baseline, new, expanded, narrowed and invalid-reason records and identifies
the structural owner and category. Inspect the reported item before adding an exception. A cohesive
helper or options value may resolve the warning; moving arbitrary lines to satisfy a threshold does
not improve ownership.

Trusted PR policy reads the allowlist from the base revision. A PR cannot grant its own new file a
generated exemption. Reduce existing debt through focused changes rather than refreshing the
baseline to conceal it.
