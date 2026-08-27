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
- List focused checks appropriate to the changed surface.
- Use `mise run verify` for broad or high-risk code changes.

## Spec References
- `docs/recite-production-spec.md` §<section>
```

Recite issues are human-directed co-work. A delegated implementer may own a
bounded issue or vertical slice through only the stages explicitly named in the
authorized delivery target: local edits, commit, push, and PR updates are
separate authorizations and must not be inferred from one another. Product
direction, subjective decisions, integration review, and final acceptance
remain with the coordinating maintainer. Keep task packets compact. Delegated
slices use isolated purpose-first branches or worktrees and do not open
issue-slice pull requests. The coordinator reviews each slice, returns findings
to its implementer for correction, and mechanically integrates accepted
commits; the coordinator does not patch implementer work except for mechanical
conflict resolution.

## Milestone integration workflow

For a milestone pass, the coordinator creates one purpose-first
`integration/<short-kebab-topic>` branch from `main`. Each bounded slice is
assigned to an isolated normal purpose-first branch or worktree based on that
integration branch. The packet names the
slice, base revision, write scope, acceptance checks, stop-and-ask categories,
and authorized delivery stages. A worker may commit or push only when those
stages are explicitly authorized, but does not open a pull request for the
slice.

The coordinator reviews the slice diff and its focused checks. Findings go back
to the owning implementer for a correction pass. Once accepted, the coordinator
cherry-picks the worker's commits mechanically into the integration branch; use
`--ff-only` only when the accepted branch is a direct fast-forward. Do not use
a default non-fast-forward merge, which creates a `Merge branch ...` commit
that fails the commit policy. If an exceptional merge commit is unavoidable,
the coordinator must review it and give it an explicit policy-compliant
`[REC-N] <type>: <subject>` message; prefer cherry-picking instead. Do not
rewrite implementation work in the coordinator's worktree.
Workers keep the normal `[REC-N]` conventional commit subject and attribution
policy.

At a stable checkpoint, the coordinator opens exactly one protected integration
pull request from the integration branch to `main`. Apply the
`workflow/integration` label and use an `integration/<short-kebab-topic>` head
branch targeting `main`; CI requires all three before enabling integration mode.
Use the milestone tracking issue in the PR title. The final integration PR may
contain multiple valid `[REC-N]` issue codes; its title code identifies the
milestone tracking issue. The protected GitHub checks and review gate apply to
this PR. After it merges, verify live GitHub state and refresh
`docs/roadmap.md` on `main`.

## Review and protected merge

Recite's protected `main` policy and the read-only helper are the repository
sources of truth. Before merging, inspect the pull request's current head,
standard GitHub reviews and threads, resolve or explicitly reject each review
comment, and run checks appropriate to the changed surface. Use focused checks
for documentation or instruction-only changes and `mise run verify` for broad
or high-risk code changes. Required GitHub CI and branch protection remain
authoritative at merge. The helper below is for a standalone PR or the one
coordinator-owned integration PR targeting protected `main`; delegated slices
are reviewed and integrated without issue-slice PRs. Then pass:

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
gh pr merge <pr> --repo plethu/recite --squash --delete-branch
```

Human maintainer approval remains authoritative. Codex Code Review is advisory
and asynchronous:
it does not replace human approval, branch protection, required checks, or
tests. The current solo-maintainer policy permits the allowlisted maintainer's
self-review; once another human maintainer exists, require their independent
standard GitHub approval. The gate requires the exact current head SHA and no
unresolved review threads.

For the official Codex GitHub integration, including automatic review setup,
see the [official Codex GitHub review documentation](https://learn.chatgpt.com/docs/third-party/github).
For review details, read `references/github-merge-details.md`. Do not parse
custom review comments, bot usernames, or marker blocks.

After merging, verify the linked issue/PR and refresh `docs/roadmap.md` against
live GitHub state. If the merge closed, unblocked, or superseded a roadmap item,
update the roadmap on `main` and complete its own policy-checked follow-up.
