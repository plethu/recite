# Recite Agent Instructions

Recite is a Rust-first deterministic dialogue compiler, runtime, and tooling project for ECS-oriented games. It is dual-licensed public open source under MIT OR Apache-2.0; do not introduce proprietary content, copied private material, or dependency code that is incompatible with that distribution.

## Project Workflow

- Use GitHub as the canonical forge. Use the GitHub CLI (`gh`) with explicit
  `--repo plethu/recite` for issue, milestone, label, and pull-request work.
- Work from `main` on short-lived, purpose-first branches and follow the
  machine-wide branch naming convention; never prefix a branch with an issue
  number. Recite commit subjects begin with `[REC-N]`, followed by a concise
  conventional-commit-style subject.
- Run `scripts/check-git-policy.sh` locally. It is part of the complete
  verification gate and checks the relevant change range on pull requests;
  never add agent-attribution trailers.
- Keep patches scoped to the issue or user request.
- Do not revert unrelated user changes.
- Prefer small, reviewable changes over broad refactors.
- After merging issue or PR work into `main`, verify `docs/roadmap.md` against
  live GitHub issue and PR state before handoff. If the merge closed, unblocked,
  or superseded roadmap items, update the roadmap on `main`, commit, and push
  that follow-through.
- For non-trivial Rust changes, use the relevant Recite overlay, especially
  `.agents/skills/recite-rust-quality/SKILL.md`, and load the global
  `rust-quality` skill when it is available.
- The complete local gate is `mise run verify`; use a narrower documented check
  only when the changed surface makes that sufficient.
- Follow the Rust test organization policy in `.agents/skills/recite-testing-diagnostics/SKILL.md`; PR gates fail if tests are added in the wrong location.
- Repo-local skills must be Recite-specific overlays or Recite domain guidance. Put reusable personal workflow skills in the global agent config instead.

## Multi-Agent Execution Contract

- The coordinating Sol session owns scope, product direction, subjective decisions, integration review, and final acceptance. When the user's/task's authorized delivery target includes repository mutations, a delegated Luna implementer owns its bounded change through implementation, focused checks, a signed commit, push, and pull-request updates. Sol does not absorb implementation fixes; return findings to the owning implementer, except for mechanical conflict resolution.
- Task packets stay compact: outcome, write scope, settled constraints, permitted decisions, stop-and-ask categories, acceptance evidence, and delivery target. Keep assumptions bounded: agents may choose reversible local details, but must stop for semantics, public compatibility, destructive changes, or scope expansion.
- Parallel writable implementers, or workers with potentially conflicting writable scopes, use isolated branches/worktrees at the stated base SHA. Read-only workers and a lone worker with an available clean checkout may use it after the same preflight for clean state, repository/remote, branch policy, and required tools or GitHub access. Concurrent writable workers must have disjoint scopes; never share a writable worktree.
- If implementation fails, retry once with the concrete error. On a second failure, replace it with a fresh worker/context and diagnose the environment or boundary; do not silently take over the implementation.
- Reviewers and auditors remain independent and read-only. Send actionable findings back to the implementer for a correction pass; do not turn review into orchestrator-authored patching.
- Verify in proportion to change: targeted checks in the inner loop, one appropriate full gate when a coherent slice stabilises, and broader checks at milestone or release boundaries. Batch review fixes and do not rerun the whole gate after every tiny correction. Codex review is advisory and asynchronous; continue useful work rather than making it a critical path.
- A delivery handoff names the resulting behaviour, commit SHA, pushed branch/PR, checks and outcomes, and residual uncertainty. “Changes exist in a temporary worktree” is not completion.

## Agent Workflow Routing

- Ordinary issue work routes through `.agents/skills/recite-github-pm/SKILL.md`.
- Clean diff review uses the global `code-review` skill plus the relevant
  Recite domain or language skill in a fresh reviewer context.
- Roadmap ownership and final protected merges remain with the coordinating
  main session; delegated work keeps `docs/roadmap.md` read-only unless its
  issue explicitly includes it.

## Product Invariants

- Runtime traversal must be deterministic.
- Runtime code must never perform game-side effects.
- Effects are typed, schema-checked requests emitted to the caller.
- Runtime state must be serializable without game state.
- Author-visible line and choice IDs must remain stable once written.
- Dialogue outputs, choices, metadata, effects, and diagnostics should be structured values, not prose conventions.
- Validation should catch malformed project content without running a game engine.
- Semantic changes should include tests unless the work is explicitly exploratory.

## Code Review Rules

- Preserve deterministic runtime traversal and keep game-side effects outside
  runtime code; effects remain typed, schema-checked requests for the caller.
- Treat serialisable runtime state, structured dialogue outputs, and
  author-visible line and choice IDs as compatibility surfaces. Flag changes
  that weaken those boundaries and identify the safe migration path.

## Spec Authority

The production spec lives at `docs/recite-production-spec.md`. Route work to these sections:

- Parser/source format: §5
- Conditions: §6
- Effects: §7
- Runtime: §8
- Localisation and stable IDs: §9
- Schema: §10
- Scene manifests/compiler: §11-12
- CLI: §13
- LSP/editor support: §14-15
- Bevy adapter: §16
- Tests and diagnostics: §17-18
- Performance/benchmarks: §19
- Milestones and serious v1 gate: §22-23

Verify section numbers against the committed spec before editing this table.

Do not copy large sections of the spec into agent guidance. Read the relevant spec section before changing that subsystem.

## Repo-Local Skills

Use the relevant skill for procedural details:

- GitHub issues, milestones, labels, pull requests, and project planning:
  `.agents/skills/recite-github-pm/SKILL.md`
- Recite-specific Rust maintainability, diagnostics, FFI, and file-size review
  triggers: `.agents/skills/recite-rust-quality/SKILL.md`
- Parser, AST, compiler, runtime, schema, effects, localisation IDs, and deterministic dialogue semantics: `.agents/skills/recite-core-language/SKILL.md`
- Fixtures, snapshots, diagnostics, CLI checks, LSP behavior, and headless runtime tests: `.agents/skills/recite-testing-diagnostics/SKILL.md`

For agent-facing instruction edits, use the global `agent-instructions` skill when available. In this repo, keep `AGENTS.md` to workflow, product invariants, spec routing, and Recite-specific skill pointers.
