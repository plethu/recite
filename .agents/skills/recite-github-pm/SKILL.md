---
name: recite-github-pm
description: Use for Recite issue and milestone planning or authorized GitHub delivery.
---

# Recite GitHub workflow

[CONTRIBUTING.md](../../../CONTRIBUTING.md#change-and-review-workflow) owns branch, commit and PR
conventions. Use `gh` with `--repo plethu/recite`; Projects commands use `--owner plethu` and an
explicit project number. GitHub owns task state; do not create a parallel Markdown ledger.

Read the linked issue before changing scope. For release or deferral decisions, consult
[release gates](../../../docs/spec/release.md) §22–23 and identify remaining consumer evidence.

Inspect the current PR head, diff, findings and required checks before authorized delivery. GitHub
branch protection owns merge enforcement; use the native commands in the contributor workflow. After
an authorized merge, verify the linked issue and milestone state.
