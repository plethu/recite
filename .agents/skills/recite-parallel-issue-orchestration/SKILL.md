---
name: recite-parallel-issue-orchestration
description: Use when orchestrating Recite roadmap or milestone work across multiple independent issues, branches, worktrees, reviews, pull requests, and protected merges.
---

# Recite Parallel Issue Orchestration

## Why

Use the global `parallel-issue-orchestration` skill for the generic
supervisor/worker loop. This skill adds Recite-specific roadmap, issue, review,
and merge requirements.

For ordinary one-off GitHub issue or PR work, use
`.agents/skills/recite-github-pm/SKILL.md` instead.

If the user says only "orchestrate", apply this workflow without asking for a
longer prompt.

## Recite Guardrails

- The supervisor owns `docs/roadmap.md`; workers treat it as read-only unless their issue explicitly includes roadmap editing.
- Use branch names as `issue-<number>-<short-topic>`.
- Keep worktrees outside the main checkout, for example `../recite-worktrees/issue-<number>-<short-topic>`.
- Use `.agents/skills/recite-github-pm/SKILL.md` for Recite labels, PR creation,
  review gates, and issue closeout.
- For Rust changes, run the `recite-rust-quality` quick audit before review and include size-triggered files with a split/cohesion/follow-up decision in the worker handoff.

## Supervisor Setup

1. Read `.agents/skills/recite-github-pm/SKILL.md`.
2. Refresh `docs/roadmap.md` once from live GitHub state before selecting
   work. Keep the existing structure, but update stale issue state, dependency
   edges, and "can start now" entries.
3. Commit and push the roadmap refresh before spawning workers if it changed.
4. Select issues with disjoint write scopes and no unresolved dependency chain.
5. Report the selected issues, independence reason, expected write scope,
   branch/worktree names, and any terminal-worker usage approval needed.
6. Claim each issue sequentially through the GitHub CLI:

```bash
gh issue edit <issue> --repo plethu/recite \
  --remove-label "status/ready" --remove-label "status/design-needed" \
  --add-label "status/in-progress"

.agents/skills/recite-github-pm/scripts/recite-pm-check.sh issue <issue>
```

## Recite Worktree

```bash
git fetch origin main
git worktree add ../recite-worktrees/<branch> -b <branch> origin/main
```

Give each worker the issue number, branch, worktree path, expected write scope,
the relevant roadmap excerpt, and this rule: other agents may be working
elsewhere, so do not revert unrelated changes. Workers may read their assigned
issue live, but they should not re-audit the full issue graph.

## PR And Merge

Push each branch, open the PR through `gh pr create`, and move the issue to
`status/review`; use
`.agents/skills/recite-github-pm/references/issue-pr-examples.md` for command
shape. Before merge, require a current clean-context review comment for the
exact PR head SHA; see
`.agents/skills/recite-github-pm/references/github-merge-details.md`.

Merge one PR at a time from a clean main worktree after the read-only gate and
full project verification:

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh <pr> <branch> main
mise run verify
gh pr merge <pr> --repo plethu/recite --squash --delete-branch
```

After merge, verify the PR and linked issue, explicitly close the issue if the
merge did not auto-close it, then remove the worktree and branch.

When all worker PRs are merged, refresh `docs/roadmap.md` again from the final
issue/PR state, commit and push it if it changed, and mention the roadmap commit
in the handoff.
