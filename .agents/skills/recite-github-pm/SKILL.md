---
name: recite-github-pm
description: "Use for Recite-specific GitHub project management: labels, milestones, issue shape, review gates, pull requests, and repo helper scripts."
---

# Recite GitHub Workflow

[CONTRIBUTING.md](../../../CONTRIBUTING.md#change-and-review-workflow) owns branch, commit and
pull-request requirements. Use `gh` with explicit `--repo plethu/recite`; GitHub Projects commands
use `--owner plethu` and the explicit project number.

## Planning and verification

For single-issue work, run the lightweight checker and read the issue:

```sh
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh quick
gh issue view 17 --repo plethu/recite --json number,title,state,milestone,labels,url
```

Use `recite-pm-check.sh full` for broad planning or label/milestone audits. After an issue mutation,
verify it with `recite-pm-check.sh issue 17`. These helpers are read-only. Inspect current forge
state before mutating it; keep mutations sequential and respect rate-limit retry windows.

Use existing labels and milestone names from `docs/spec/release.md` §22. Check the issue and serious
v1 boundary (§23) before deferring adapter, editor or performance work. An implementation issue
should name its goal, bounded scope, settled decisions, remaining questions, observable acceptance
criteria, exclusions, check commands and affected contract sections. GitHub owns delivery state; do
not maintain a second status ledger in Markdown.

## Authorized delegation

Ordinary work stays with the primary agent. Delegate only when the session authorizes it; do not
request a fresh reviewer by default.

For milestone slices, use isolated branches/worktrees at the stated integration SHA. Keep the task
packet to outcome, write scope, settled constraints, permitted decisions, stop-and-ask categories,
acceptance evidence and authorized delivery stages. Local edits, commits, pushes and forge updates
are distinct stages. The coordinator owns scope, product decisions, review and final acceptance.

Reviewers remain read-only. Return findings to the implementing worker. Review each slice's diff and
checks before mechanically cherry-picking accepted commits; only mechanical conflict resolution
belongs in the coordinator's worktree. Use `--ff-only` for a direct fast-forward; avoid generated
merge subjects that fail Git policy. Delegated slices do not open pull requests.

If implementation fails, retry once with the concrete error. On a second failure, use a fresh worker
context and diagnose the environment or boundary rather than silently taking over.

## Protected delivery

Follow the contributor workflow for the final standalone or milestone integration PR. Inspect its
current head, reviews, unresolved threads and required checks; passing local checks does not replace
GitHub branch protection.

The [merge reference](references/github-merge-details.md) describes the exact-head helper and
maintainer approval rules. Run that helper before an authorized merge. Optional automated review is
advisory and requested only when authorized. After merging, verify the linked issue, PR and
milestone state.
