---
name: recite-parallel-issue-orchestration
description: Use when the user says to orchestrate Recite roadmap or milestone work across multiple independent agents, issues, worktrees, branches, reviews, pull requests, and signed merges.
---

# Recite Parallel Issue Orchestration

## Why

Use this for multi-agent execution only. For ordinary one-off Codeberg issue or
PR work, use `.agents/skills/recite-codeberg-pm/SKILL.md` instead.

If the user says only "orchestrate", apply this workflow without asking for a
longer prompt.

## Guardrails

- One supervisor chooses issues, assigns ownership, and serializes remote
  mutations.
- Each worker owns one Codeberg issue, one branch, one worktree, and one PR.
- Do not parallelize mutating `tea` commands or signed merges.
- Do not treat `codex exec` as unmetered; Codex local usage counts toward
  Codex/agentic usage limits. Ask before launching multiple terminal workers.
- Do not use fast/priority service tiers for bulk workers unless the user asks.
- Prefer in-session subagents when they are available and adequate. Use
  terminal-launched `codex exec` workers only when the user has approved that
  usage.
- The supervisor owns `docs/roadmap.md`; workers treat it as read-only unless
  their issue explicitly includes roadmap editing.

## Model Effort Defaults

Use benchmarked model-effort tiers. Do not substitute `medium`, `high`, or
`xhigh` by analogy when the chosen tier does not have current benchmark data.
Re-check current Artificial Analysis or comparable tier-specific data before
changing these defaults.

| Stage | Default | Escalate when |
| --- | --- | --- |
| Supervisor triage | Current session | Do not spawn a worker only to pick issues. |
| Mechanical docs worker | `gpt-5.4-mini`, `medium` | Use `gpt-5.4-mini` `xhigh` for structured code/test edits. |
| Normal implementation worker | `gpt-5.3-codex`, `xhigh` | Use `gpt-5.5` `medium` when the issue is underspecified or design-heavy. |
| Plan reviewer | `gpt-5.5`, `medium` | Use `gpt-5.5` `high` for cross-cutting design, public API, or data format decisions. |
| Implementation reviewer | `gpt-5.3-codex`, `xhigh` | Use `gpt-5.5` `high` for public API, repeated-review, or hard merge-gate work. |
| High-risk implementation worker | `gpt-5.5`, `medium` | Use `gpt-5.5` `high` or `xhigh` only after repeated failed passes. |

## Supervisor Setup

1. Read `.agents/skills/recite-codeberg-pm/SKILL.md`.
2. Refresh `docs/roadmap.md` once from live Codeberg state before selecting
   work. Keep the existing structure, but update stale issue state, dependency
   edges, and "can start now" entries.
3. Commit and push the roadmap refresh before spawning workers if it changed.
4. Select issues with disjoint write scopes and no unresolved dependency chain.
5. Report the selected issues, independence reason, expected write scope,
   branch/worktree names, and any terminal-worker usage approval needed.
6. Assign branch names as `issue-<number>-<short-topic>`.
7. Keep worktrees outside the main checkout, for example
   `../recite-worktrees/issue-<number>-<short-topic>`.
8. Claim each issue sequentially through the wrapper:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea issues edit <issue> \
    --remove-labels status/ready,status/design-needed \
    --add-labels status/in-progress

.agents/skills/recite-codeberg-pm/scripts/recite-pm-check.sh issue <issue>
```

## Per-Issue Worktree

```bash
git fetch origin main
git worktree add ../recite-worktrees/<branch> -b <branch> origin/main
```

Give each worker the issue number, branch, worktree path, expected write scope,
the relevant roadmap excerpt, and this rule: other agents may be working
elsewhere, so do not revert unrelated changes. Workers may read their assigned
issue live, but they should not re-audit the full issue graph.

## Worker Loop

1. Plan only: read the live issue, relevant spec sections, and local skills; do
   not edit tracked files.
2. Review the plan with an independent reviewer using the table above. Iterate
   until no findings.
3. Implement the approved plan. Run the checks required by the issue or changed
   crates.
4. Review the implementation independently. Fix findings and re-review until no
   findings.

If using terminal-launched Codex workers after cost approval, use local profiles
or config overrides that make model and reasoning effort explicit:

```bash
codex -m gpt-5.3-codex -c 'model_reasoning_effort="xhigh"' \
  -C ../recite-worktrees/<branch> exec \
  -o /tmp/recite-agent-runs/<branch>/plan.md \
  "Create an implementation plan for issue #<issue>. Do not edit tracked files."

codex -m gpt-5.5 -c 'model_reasoning_effort="medium"' \
  -C ../recite-worktrees/<branch> exec \
  -o /tmp/recite-agent-runs/<branch>/plan-review.md \
  "Review the plan for issue #<issue>. Findings first. If clean, say NO FINDINGS."

codex -m gpt-5.3-codex -c 'model_reasoning_effort="xhigh"' \
  -C ../recite-worktrees/<branch> exec \
  -o /tmp/recite-agent-runs/<branch>/impl.md \
  "Implement the approved plan for issue #<issue>. Do not mutate Codeberg."

codex -m gpt-5.3-codex -c 'model_reasoning_effort="xhigh"' \
  -C ../recite-worktrees/<branch> exec review \
  --base origin/main \
  -o /tmp/recite-agent-runs/<branch>/impl-review.md \
  "Review this issue implementation. Findings first. If clean, say NO FINDINGS."
```

## PR And Merge

Push each branch, open the PR through `tea-rate-limit.sh`, and move the issue to
`status/review`; use
`.agents/skills/recite-codeberg-pm/references/issue-pr-examples.md` for command
shape. Before merge, require a current clean-context review comment for the
exact PR head SHA; see
`.agents/skills/recite-codeberg-pm/references/signed-merge-details.md`.

Merge one PR at a time from a clean main worktree:

```bash
.agents/skills/recite-codeberg-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
.agents/skills/recite-codeberg-pm/scripts/merge-pr-signed.sh <pr> <branch> main
```

After merge, verify the PR and linked issue, explicitly close the issue if the
manual signed merge did not auto-close it, then remove the worktree and branch.

When all worker PRs are merged, refresh `docs/roadmap.md` again from the final
issue/PR state, commit and push it if it changed, and mention the roadmap commit
in the handoff.
