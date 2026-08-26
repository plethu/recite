---
name: recite-github-pm
description: "Use for Recite-specific GitHub project management: labels, milestones, issue shape, review gates, pull requests, and repo helper scripts."
---

# Recite GitHub Project Management

## Why

Use the GitHub CLI (`gh`) for Recite project management. Always pass
`--repo plethu/recite` so a stale local remote cannot send an operation to the
wrong repository. This skill adds Recite-specific labels, issue shape,
milestone/spec routing, and review gates.

## Recite Preflight

For single-issue work, prefer the lightweight checker and a targeted issue read:

```bash
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh quick
gh issue view 17 --repo plethu/recite --json number,title,state,milestone,labels,url
```

Use `recite-pm-check.sh full` for broad planning or label/milestone audits, not
as a routine before every mutation. If a command would create or edit many
remote objects, write an idempotent script that checks current state first and
skips existing objects.

For detailed issue and PR command examples, read
`references/issue-pr-examples.md`. For multi-agent roadmap or milestone
orchestration, use `.agents/skills/recite-parallel-issue-orchestration/SKILL.md`.

## Labels

Use these labels when useful:

| Category | Values |
| --- | --- |
| Status | `status/ready`, `status/design-needed`, `status/in-progress`, `status/review`, `status/blocked` |
| Area | `area/parser`, `area/ast`, `area/compiler`, `area/runtime`, `area/cli`, `area/lsp`, `area/localisation`, `area/schema`, `area/bevy`, `area/editor`, `area/tests`, `area/docs` |
| Kind | `kind/design`, `kind/implementation`, `kind/tests`, `kind/refactor`, `kind/docs`, `kind/bug` |
| Size | `size/s`, `size/m`, `size/l` |
| Risk | `risk/high`, `risk/cross-cutting` |

Recite issues are Mari + agent co-work by default. Do not encode assumptions
that an issue will be fully autonomous.

## Milestones

Use the milestone names from `docs/recite-production-spec.md` §22. The serious
v1 boundary is defined by §23 and is broad: it covers the core runtime, CLI,
LSP, scale proof, the engine-adapter contract, production-quality Godot, Unity,
and Bevy adapters, and adoption/migration docs. Do not treat adapter,
performance, or editor work as automatically post-v1; defer to §23 and the
issue's milestone.

## Issue Shape

Use this body structure for implementation issues:

```markdown
## Goal
One concrete outcome.

## Scope
What behavior, crate, or surface is in bounds.

## Known Decisions
Project decisions that should not be reopened in this issue.

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

## Suggested Branch
`<kind>/<short-kebab-topic>`

Use one of `feat`, `fix`, `refactor`, `perf`, `ci`, `docs`, `test`, `build`,
`chore`, `spike`, `release`, or `security` for `kind`; do not put the issue
number at the start of a branch. Commit subjects begin with `[REC-N]` followed
by a concise conventional-commit-style subject. Keep commit bodies to at most
one explanatory sentence and do not add `Co-Authored-By:` or other
agent-attribution trailers. Run `scripts/check-git-policy.sh` before handoff.
```

## Review And Merge Pipeline

Recite requires signed commits and explicit review gates. Treat GitHub branch
protection and required checks as the repository-level source of truth. The
protected `main` policy requires a pull request, the aggregate project check,
linear history, and signed commits. Recite is currently solo-maintained, so the
temporary review path permits maintainer self-review; once another human
maintainer exists, require independent approval. Before merging, run the
read-only gate, inspect the standard GitHub reviews, resolve or explicitly
reject every review comment, and run `mise run verify` (or `scripts/verify.sh`).
Human maintainer approval remains authoritative; Codex Code Review is advisory
and does not replace human approval, branch protection, required checks, or
tests. For Rust changes, run the
`.agents/skills/recite-rust-quality/SKILL.md` quick audit and include its
size-triggered split/cohesion/follow-up handoff in review notes.

Normal read-only gate and GitHub merge commands are:

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
gh pr merge <pr> --repo plethu/recite --squash --delete-branch
```

Do not merge until the gate and local checks pass. Use GitHub's protected pull
request path so the configured review policy, checks, linear-history, and
commit-signing requirements remain visible and enforceable. For standard review
and approval details, read `references/github-merge-details.md`.

## API Courtesy Rules

- Use `gh` with an explicit `--repo plethu/recite` for every remote mutation.
- Keep remote mutations sequential and check current state before creating or
  editing issues, labels, milestones, or pull requests.
- Use `.agents/skills/recite-github-pm/scripts/recite-pm-check.sh issue
  <number>` after a single-issue mutation.
- Use `.agents/skills/recite-github-pm/scripts/recite-pm-check.sh full`
  sparingly for planning or project-wide
  audits. Full mode caches labels and milestones under `/tmp/recite-pm-cache`
  for 30 minutes by default; adjust with `RECITE_PM_CACHE_DIR` and
  `RECITE_PM_CACHE_TTL_SECONDS` if needed.
- GitHub rate-limit or server failures are authoritative. Stop the current
  remote-mutation pass, surface the failure, and resume only after checking
  the response and waiting for any server-provided retry window.

## Verification

After a single issue mutation, verify only that issue:

```bash
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh issue 17
```

After broad label, milestone, or planning work, run the full audit once:

```bash
.agents/skills/recite-github-pm/scripts/recite-pm-check.sh full
```
