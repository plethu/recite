---
name: recite-parallel-issue-orchestration
description: Use when coordinating multiple independent Recite issues, worktrees, reviews, and protected GitHub merges.
---

# Recite Parallel Issue Orchestration

This is a small Recite-specific overlay, not a role registry or launcher. Use
the global `workflow-roles` skill and `~/.config/agents/roles.toml` for the
portable explorer, researcher, architect, implementer, reviewer, verifier, and
editor contracts, context policy, harness mappings, and diagnostics. Use
`recite-github-pm` for GitHub issue, pull-request, review, and merge procedure.

## Routing

- Ordinary single-issue work uses `recite-github-pm` and an implementer in the
  current task worktree.
- Parallel issue work uses a coordinator/main session plus one implementer
  worktree per independent issue. Select disjoint write scopes and resolve
  dependencies before delegation; do not create a project-specific launcher or
  second role registry.
- Clean review uses a fresh read-only reviewer context with `code-review` and
  the relevant domain/language skills. Verification uses a separate verifier
  context where practical; an editor handles voice-preserving documentation
  changes.
- The coordinator owns roadmap refreshes, issue-state mutations, worktree
  lifecycle, and the final protected merge. Workers do not edit
  `docs/roadmap.md` unless their issue explicitly includes it.

## Recite guardrails

- GitHub is canonical. Pass `--repo plethu/recite` to each `gh` issue, project,
  pull-request, review, and merge subcommand.
- Branches are purpose-first: `<kind>/<short-kebab-topic>` using the kinds in
  `AGENTS.md`; never put an issue number first. Keep each worker branch and
  worktree outside the main checkout, and never share a writable worktree.
- Pass each delegated worker the issue, acceptance criteria, write scope,
  worktree/branch, relevant spec and skills, and current evidence. Do not rely
  on parent conclusions, loaded skills, or undocumented child-context
  inheritance; a child must be able to rediscover the repository instructions.
- Workers report changed behaviour, decisions, exact checks, failures, and
  residual risk. Reviewers receive the actual diff and surrounding code in a
  clean context, not an implementer's conclusions as facts.
- Use standard GitHub reviews and the configured protected-branch gate. Official
  Codex Code Review is an optional independent signal when enabled; it is
  advisory and never a replacement for human authority, CI, or branch
  protection. Do not parse custom review comments, bot usernames, or marker
  blocks.
- Before merge, use the read-only gate and require `mise run verify` on the
  current pull-request head. Keep the main checkout clean for merge
  orchestration; after merge, verify the linked issue/PR and refresh the
  roadmap against live GitHub state. Follow `recite-github-pm` for the exact
  commands.

## Handoff boundary

The coordinator decides which issues may run together and whether evidence is
strong enough to merge. An implementer may propose a split or follow-up, but
must not expand the product direction. Subjective language/runtime/API choices
remain with the human-led coordinator; this overlay only constrains execution.

For the bounded evidence exercise that validated this overlay, read
`references/two-issue-exercise.md`. It records observed results, not a second
procedure to keep in sync.
