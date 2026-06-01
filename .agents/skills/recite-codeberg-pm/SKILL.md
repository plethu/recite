---
name: recite-codeberg-pm
description: "Use for Recite Codeberg/Forgejo project management with tea: issues, milestones, labels, pull requests, issue planning, co-work status labels, and issue branch workflow."
---

# Recite Codeberg Project Management

## Why

Recite uses Codeberg for public project management. Codeberg is free shared infrastructure, so prefer correctness, idempotence, and courtesy over throughput. Do not use GitHub workflows for this project.

## Current Tooling Assumptions

- Use the `tea` CLI for Codeberg operations.
- Assume `tea` batching is unavailable for this workflow. Use sequential commands.
- This workflow was written against `tea 0.14.0`.
- If the installed `tea` version differs, re-check command syntax and whether rate-limit headers are exposed before trusting wrapper defaults.
- Run `tea --version` and the relevant `tea <subcommand> --help` when command syntax matters.

## Before Remote Mutation

Confirm the target repo and only the current state needed for the operation. For single-issue work, prefer the lightweight checker and a targeted issue read:

```bash
.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh quick
tea issues 17 --fields index,title,state,milestone,labels,url
```

Use `recite-pm-check.sh full` for broad planning or label/milestone audits, not as a routine before every mutation. If a command would create or edit many remote objects, write an idempotent script that checks current state first and skips existing objects.

For detailed issue and PR command examples, read `references/issue-pr-examples.md`.
For multi-agent roadmap or milestone orchestration, use `.agents/skills/recite-parallel-issue-orchestration/SKILL.md`.

## Labels

Use these labels when useful:

| Category | Values |
| --- | --- |
| Status | `status/ready`, `status/design-needed`, `status/in-progress`, `status/review`, `status/blocked` |
| Area | `area/parser`, `area/ast`, `area/compiler`, `area/runtime`, `area/cli`, `area/lsp`, `area/localisation`, `area/schema`, `area/bevy`, `area/editor`, `area/tests`, `area/docs` |
| Kind | `kind/design`, `kind/implementation`, `kind/tests`, `kind/refactor`, `kind/docs`, `kind/bug` |
| Size | `size/s`, `size/m`, `size/l` |
| Risk | `risk/high`, `risk/cross-cutting` |

Recite issues are Mari + agent co-work by default. Do not encode assumptions that an issue will be fully autonomous.

## Milestones

Use the milestone names from `docs/recite-production-spec.md` §22. The serious v1 boundary is defined by §23 and is broad: it covers the core runtime, CLI, LSP, scale proof, the engine-adapter contract, at least one production adapter, and adoption/migration docs. Do not treat adapter, performance, or editor work as automatically post-v1; defer to §23 and the issue's milestone.

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
- `scripts/check-project-gates.sh`

## Spec References
- `docs/recite-production-spec.md` §<section>

## Suggested Branch
`issue-<number>-<short-topic>`
```

## Review And Signed Merge Pipeline

Recite requires signed commits and explicit review gates. Do not merge pull requests with the Codeberg web UI or any forge-side merge command, because those paths can create unsigned merge commits and bypass local guardrails.

| Stage | Required actions |
| --- | --- |
| Review | Keep the issue in `status/review`; confirm the PR targets `main` from the expected short-lived branch; review beyond acceptance criteria for correctness, maintainability, extensibility, public API shape, invariant preservation, and validation ownership; run `scripts/check-project-gates.sh`; for Rust changes, run the `.agents/skills/recite-rust-quality/SKILL.md` quick audit and include its size-triggered split/cohesion/follow-up handoff in review notes; require maintainer approval and a current clean-context agent review; resolve or explicitly reject every review comment. |
| Signed merge | Start from a clean worktree; run the review gate helper; fetch `origin/main` and the PR head branch; verify every PR commit signature with `git verify-commit`; stage a local no-ff merge, run checks, commit with `git commit -S`, push `main`, mark the PR as `manually-merged`, and verify the PR plus linked issue. |

Do not push to `main` or close the PR until review gates pass. Trusted maintainer author self-review is only a temporary single-maintainer exception.

Normal read-only gate and signed merge commands (replace the PR number, branch, and base with your values):

```bash
.agents/skills/recite-codeberg-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
.agents/skills/recite-codeberg-pm/scripts/merge-pr-signed.sh <pr> <branch> main
```

For clean-context review comment shape, manual merge recording details, and maintainer approval commands, read `references/signed-merge-details.md`.

## API Courtesy Rules

- Do read-only preflight before mutation.
- Keep preflight and verification targeted to the work at hand. Do not run broad issue-list audits when a single issue lookup is enough.
- Never parallelize remote-mutating `tea` commands.
- Use `scripts/tea-rate-limit.sh` for mutating issue, PR, label, and milestone commands. It has best-effort 5xx detection; treat a non-zero `tea` exit code as authoritative.
- Use `scripts/recite-pm-check.sh issue <number>` after a single-issue mutation.
- Use `scripts/recite-pm-check.sh full` sparingly for planning or project-wide audits. Full mode caches labels and milestones under `/tmp/recite-pm-cache` for 30 minutes by default; adjust with `RECITE_PM_CACHE_DIR` and `RECITE_PM_CACHE_TTL_SECONDS` if needed.
- The wrapper defaults to at least 75 seconds between issue/PR mutations and at least 10 seconds between label/milestone mutations.
- The wrapper does not auto-retry or auto-sleep after a rate-limit failure. It surfaces the limit and exits; stop the current remote-mutation pass until the user explicitly resumes or the wait window has passed. If `tea` prints a rate-limit wait, honor it; otherwise wait at least 15 minutes before resuming.
- On a single 5xx-like Forgejo/Codeberg failure, stop the current remote-mutation pass, surface the failure, and do not silently retry. Treat a second 5xx during the same pass as repeated failure and wait for user direction.
- The wrapper lock prevents concurrent agent sessions or terminals from mutating Codeberg at the same time through this project workflow.

## Verification

After a single issue mutation, verify only that issue:

```bash
.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh issue 17
```

After broad label, milestone, or planning work, run the full audit once:

```bash
.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh full
```
