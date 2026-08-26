---
name: recite-github-pm
description: "Use for Recite-specific GitHub project management: labels, milestones, issue shape, review gates, pull requests, and repo helper scripts."
---

# Recite GitHub Project Management

Use the GitHub CLI for Recite project management. Pass `--repo plethu/recite`
to issue, pull-request, review, label, and milestone commands so a stale local
remote cannot direct a mutation elsewhere. GitHub Projects commands use
`--owner plethu` and the explicit project number instead. This skill contains
only Recite's project shape and protected-merge requirements; use the global
Git and review skills for general workflow guidance.

## Preflight and verification

For single-issue work, run the lightweight checker and read the target issue:

```bash
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh quick
gh issue view 17 --repo plethu/recite --json number,title,state,milestone,labels,url
```

Use `recite-pm-check.sh full` for broad planning or label/milestone audits, not
before every mutation. After a single issue mutation, verify that issue:

```bash
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh issue 17
```

The helper scripts are read-only checks. Check current state before creating or
editing remote objects, keep remote mutations sequential, and make broad
mutations idempotent. Stop a mutation pass on a GitHub rate-limit or server
failure and respect the server-provided retry window.

## Recite project shape

Use these labels when useful:

| Category | Values |
| --- | --- |
| Status | `status/ready`, `status/design-needed`, `status/in-progress`, `status/review`, `status/blocked` |
| Area | `area/parser`, `area/ast`, `area/compiler`, `area/runtime`, `area/cli`, `area/lsp`, `area/localisation`, `area/schema`, `area/bevy`, `area/editor`, `area/tests`, `area/docs` |
| Kind | `kind/design`, `kind/implementation`, `kind/tests`, `kind/refactor`, `kind/docs`, `kind/bug` |
| Size | `size/s`, `size/m`, `size/l` |
| Risk | `risk/high`, `risk/cross-cutting` |

Use milestone names from `docs/recite-production-spec.md` §22. The serious v1
boundary is §23; do not automatically defer adapter, performance, or editor
work without checking that section and the issue milestone.

Implementation issues should state:

```markdown
## Goal
One concrete outcome.

## Scope
What behavior, crate, or surface is in bounds.

## Known Decisions
Decisions that should not be reopened in this issue.

## Open Questions
Questions that must be answered during co-work.

## Acceptance Criteria
- Observable result.
- Required error behavior or invariant.
- Required tests/checks.

## Out of Scope
Nearby work not included.

## Test/Check Commands
- `mise run verify`

## Spec References
- `docs/recite-production-spec.md` §<section>
```

Recite issues are maintainer and agent co-work by default; do not encode an
assumption that an issue will be fully autonomous.

## Review and protected merge

Recite's protected `main` policy and the read-only helper are the repository
sources of truth. Before merging, inspect the pull request's current head,
standard GitHub reviews and threads, resolve or explicitly reject each review
comment, run `mise run verify`, and pass:

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
gh pr merge <pr> --repo plethu/recite --squash --delete-branch
```

Human maintainer approval remains authoritative. Codex Code Review is advisory:
it does not replace human approval, branch protection, required checks, or
tests. The current solo-maintainer policy permits the allowlisted maintainer's
self-review; once another human maintainer exists, require their independent
standard GitHub approval. The gate requires the exact current head SHA and no
unresolved review threads.

For the official Codex GitHub integration, including automatic review setup,
see the [official Codex GitHub review documentation](https://learn.chatgpt.com/docs/third-party/github).
For review details and patient polling behavior, read
`references/github-merge-details.md`. Do not parse custom review comments,
bot usernames, or marker blocks.

After merging, verify the linked issue/PR and refresh `docs/roadmap.md` against
live GitHub state. If the merge closed, unblocked, or superseded a roadmap item,
update the roadmap on `main` and complete its own policy-checked follow-up.
