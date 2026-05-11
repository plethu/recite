---
name: recite-codeberg-pm
description: Use for Recite Codeberg/Forgejo project management with tea: issues, milestones, labels, pull requests, issue planning, co-work status labels, and issue branch workflow.
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

Confirm the target repo and current state:

```bash
git remote -v
tea issues list --limit 5
tea labels list
tea milestones list
```

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
    --description "$tmp_body"
```

Move one issue to review by removing the old status and adding the new one:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues edit 17 \
    --remove-label "status/in-progress" \
    --labels "status/review"
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
    --description "$tmp_body"
```

## API Courtesy Rules

- Do read-only preflight before mutation.
- Never parallelize remote-mutating `tea` commands.
- Use `scripts/tea-rate-limit.sh` for mutating issue, PR, label, and milestone commands.
- The wrapper defaults to at least 75 seconds between issue/PR mutations. This is based on a prior observed Codeberg throttle of 31 issue creations under 30 minutes, plus buffer.
- The wrapper defaults to at least 10 seconds between label/milestone mutations as a courtesy safety floor.
- The wrapper does not auto-retry or auto-sleep after a rate-limit failure. It surfaces the limit and exits; the agent must stop the current remote-mutation pass until the user explicitly resumes or the wait window has passed.
- On a single 5xx-like Forgejo/Codeberg failure, stop the current remote-mutation pass, surface the failure, and do not silently retry. Treat a second 5xx during the same pass as repeated failure and wait for user direction.
- The wrapper lock prevents concurrent agent sessions or terminals from mutating Codeberg at the same time through this project workflow.

## Verification

After remote changes, run:

```bash
.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh
```
