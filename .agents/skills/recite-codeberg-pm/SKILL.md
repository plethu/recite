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
- If the installed `tea` version differs, re-check command syntax and whether rate-limit headers are exposed before trusting the wrapper defaults.
- `tea 0.14.0` does not expose HTTP rate-limit headers in normal command output. If `tea` prints a rate-limit wait in an error, honor it; otherwise wait at least 15 minutes before resuming.
- `scripts/tea-rate-limit.sh` has best-effort 5xx detection. Treat a non-zero `tea` exit code as the authoritative failure signal even if the output is not recognized as a 5xx.
- Run `tea --version` and the relevant `tea <subcommand> --help` when command syntax matters.

## Before Remote Mutation

Confirm the target repo and only the current state needed for the operation. For single-issue work, prefer the lightweight checker and a targeted issue read:

```bash
.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh quick
tea issues 17 --fields index,title,state,milestone,labels,url
```

Use `recite-pm-check.sh full` for broad planning or label/milestone audits, not as a routine before every mutation. The full mode lists labels, milestones, and open issues, so avoid running it repeatedly in one workflow unless the remote project state may have changed externally.

If a command would create or edit many remote objects, write an idempotent script that checks current state first and skips existing objects.

## Labels

Use these status labels:

- `status/ready`
- `status/design-needed`
- `status/in-progress`
- `status/review`
- `status/blocked`

Use scoped labels for area, kind, size, and risk when useful:

- Areas: `area/parser`, `area/ast`, `area/compiler`, `area/runtime`, `area/cli`, `area/lsp`, `area/localisation`, `area/schema`, `area/bevy`, `area/editor`, `area/tests`, `area/docs`
- Kinds: `kind/design`, `kind/implementation`, `kind/tests`, `kind/refactor`, `kind/docs`, `kind/bug`
- Sizes: `size/s`, `size/m`, `size/l`
- Risks: `risk/high`, `risk/cross-cutting`

Recite issues are Mari + agent co-work by default. Do not encode assumptions that an issue will be fully autonomous.

## Milestones

Use the milestone names from `docs/recite-production-spec.md` §22. The serious v1 boundary is Rust core + CLI + LSP first; Bevy and performance harness work are tracked later unless the issue says otherwise.

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
- `cargo test`

## Spec References
- `docs/recite-production-spec.md` §<section>

## Suggested Branch
`issue-<number>-<short-topic>`
```

## Examples

Create one issue:

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
## Goal
Parse block headers with stable source spans.

## Scope
Parser and AST behavior for block headers only.

## Known Decisions
Runtime traversal is out of scope. Diagnostics should carry source spans.

## Open Questions
None known.

## Acceptance Criteria
- Parses named blocks.
- Rejects malformed block headers with a span.
- Adds focused parser tests.

## Out of Scope
Diverts, choices, runtime traversal, and LSP diagnostics.

## Test/Check Commands
- `cargo test`

## Spec References
- `docs/recite-production-spec.md` §5.2

## Suggested Branch
`issue-N-parser-block-headers`
EOF

.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues create \
    --title "Parser: parse block headers" \
    --labels "area/parser,kind/implementation,size/s,status/ready" \
    --description "$(cat "$tmp_body")"
```

Move one issue to review by removing the old status and adding the new one:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues edit \
    --remove-labels "status/in-progress" \
    --add-labels "status/review" \
    17
```

Open a pull request:

```bash
tmp_body="$(mktemp)"
cat > "$tmp_body" <<'EOF'
Closes #17

## Summary
Adds parser support for block headers with source spans.

## Tests
- `cargo test`
EOF

.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea pulls create \
    --head issue-17-parser-block-headers \
    --base main \
    --title "Parser: parse block headers" \
    --description "$(cat "$tmp_body")"
```

## Review and Signed Merge Pipeline

Recite requires signed commits and explicit review gates. Codeberg branch protection is the source of truth for repository-level merge policy; local helpers audit that policy and handle Recite-specific gates that Codeberg cannot express. Do not merge pull requests with the Codeberg web UI or any forge-side merge command, because those paths can create unsigned merge commits and bypass local guardrails. Agents must treat review and merge as separate stages:

1. Review stage:
   - Keep the issue in `status/review`.
   - Confirm the PR targets `main` from the expected short-lived branch.
   - Review the diff and run the requested checks locally.
   - Require explicit Codeberg approval from a known maintainer.
   - Allow trusted maintainer author self-review only as a temporary single-maintainer exception. This is for transparency while Mari is the only maintainer, not a substitute for independent review, and must be revisited before or when another maintainer is added.
   - Require a clean-context agent review comment for the current PR head SHA.
   - Resolve or explicitly reject every review comment before merge.
   - Do not push to `main` or close the PR until review gates pass.
2. Signed merge stage:
   - Start from a clean worktree.
   - Run the review gate helper, which reads PR and branch protection state from the Codeberg API.
   - Fetch `origin/main` and the PR head branch reported by the Codeberg API.
   - Verify every PR commit signature with `git verify-commit`.
   - Create a local no-ff merge, run checks, then commit the merge with `git commit -S`.
   - Push `main` over SSH.
   - Mark the PR as `manually-merged` through the Codeberg API using the signed local merge commit SHA.
   - Verify the PR and linked issue with targeted reads. Linked issues should close automatically from actionable references such as `Closes #17` once Codeberg records the PR as merged.

Use the signed merge helper for the normal path:

```bash
.agents/skills/recite-codeberg-pm/scripts/merge-pr-signed.sh 34 issue-1-workspace-split main
```

The helper refuses to run with a dirty worktree, reads the PR base/head/head SHA from the Codeberg API, verifies review gates, verifies PR commit signatures, stages a no-ff merge, runs `cargo fmt --check` and `cargo test`, creates a signed merge commit, pushes `main`, marks the PR as `manually-merged`, and performs a targeted PR read. If checks fail after the merge is staged, inspect the failure and run:

```bash
git merge --abort
```

The review gate helper is read-only and may be run before attempting a merge:

```bash
.agents/skills/recite-codeberg-pm/scripts/check-pr-review-gates.sh 34 issue-1-workspace-split main
```

The review gate helper uses the Codeberg API as the primary source for PR state, branch protection, reviews, comments, and commit statuses. It requires the base branch to be protected, reports the branch's required approvals and status-check configuration, and then applies Recite-local gates for clean-context agent review and the signed local merge process.

After pushing `main`, the signed merge helper verifies that the PR head SHA is contained in `origin/main`, then calls:

```bash
tea api -X POST repos/{owner}/{repo}/pulls/34/merge \
  -f Do=manually-merged \
  -f MergeCommitID=<signed-local-merge-sha> \
  -f MergeMessageField="Manually merged by signed local merge commit <sha>." \
  -f head_commit_id=<pr-head-sha>
```

If Codeberg reports that `manually-merged` is not an allowed merge style, the helper enables repository manual merge support with `allow_manual_merge=true` and `autodetect_manual_merge=true`, then retries. Do not replace this with `tea pulls merge`, because that would perform a forge-side merge instead of recording the already-pushed signed local merge.

Known maintainers are derived from Codeberg repository metadata where possible: repository owner plus repository collaborators. The helper also reads `RECITE_MAINTAINERS` as a comma-separated fallback/additional list, defaulting to `plethu`. The same maintainer set is used to decide whether a PR author may satisfy the maintainer review gate by self-reviewing, but self-review is accepted only when the trusted maintainer set has exactly one member.

Maintainer approval must use Codeberg PR approval and must go through the courtesy wrapper because it mutates remote PR state:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea pulls approve 34 "Approved for signed local merge."
```

A clean-context agent review is represented by a structured PR comment for the current PR head SHA. The reviewing agent must start from a clean context, review the PR independently, and post this exact shape through the courtesy wrapper:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea comment 34 '<!-- recite-agent-review:v1 -->
Agent-Review: approved
Head-SHA: 5b1c198ce742c81b3010eec0307e9d2cbcd1af92
Context: clean'
```

If the PR head changes, the clean-context agent review is stale and must be repeated for the new head SHA. The gate also blocks failed or errored Codeberg commit statuses when any are reported; if no statuses exist yet, local `cargo fmt --check` and `cargo test` remain mandatory.

Do not use these commands for Recite merges:

```bash
tea pulls merge ...
# or the Codeberg web "Merge" button
```

## API Courtesy Rules

- Do read-only preflight before mutation.
- Keep preflight and verification targeted to the work at hand. Do not run broad issue-list audits when a single issue lookup is enough.
- Never parallelize remote-mutating `tea` commands.
- Use `scripts/tea-rate-limit.sh` for mutating issue, PR, label, and milestone commands.
- `scripts/recite-pm-check.sh` defaults to `quick`, which only checks local remote configuration and the local `tea` version.
- Use `scripts/recite-pm-check.sh issue <number>` after a single-issue mutation.
- Use `scripts/recite-pm-check.sh full` sparingly for planning or project-wide audits. Full mode caches labels and milestones under `/tmp/recite-pm-cache` for 30 minutes by default; adjust with `RECITE_PM_CACHE_DIR` and `RECITE_PM_CACHE_TTL_SECONDS` if needed.
- The wrapper defaults to at least 75 seconds between issue/PR mutations. This is based on a prior observed Codeberg throttle of 31 issue creations under 30 minutes, plus buffer.
- The wrapper defaults to at least 10 seconds between label/milestone mutations as a courtesy safety floor.
- The wrapper does not auto-retry or auto-sleep after a rate-limit failure. It surfaces the limit and exits; the agent must stop the current remote-mutation pass until the user explicitly resumes or the wait window has passed.
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
