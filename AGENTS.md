# Recite Agent Instructions

Recite is a Rust-first deterministic dialogue compiler, runtime, and tooling project for ECS-oriented games. It is dual-licensed public open source under MIT OR Apache-2.0; do not introduce proprietary content, copied private material, or dependency code that is incompatible with that distribution.

## Project Workflow

- Use GitHub as the canonical forge. Use the GitHub CLI (`gh`) with explicit
  `--repo plethu/recite` for issue, milestone, label, and pull-request work.
- Work from `main` on short-lived branches.
- Name branches purpose-first as `<kind>/<short-kebab-topic>`, where `kind` is
  one of `feat`, `fix`, `refactor`, `perf`, `ci`, `docs`, `test`, `build`,
  `chore`, `spike`, `release`, or `security`. Never prefix a branch with an
  issue number.
- Begin every commit subject with its Recite issue code, then use a concise
  conventional-commit-style subject, for example `[REC-143] ci: enforce Git
  workflow policy`. Keep the body to at most one explanatory sentence and
  never add `Co-Authored-By:` or other agent-attribution trailers.
- Run `scripts/check-git-policy.sh` locally; it is part of the complete
  verification gate and checks the relevant change range on pull requests.
- Keep patches scoped to the issue or user request.
- Do not revert unrelated user changes.
- Prefer small, reviewable changes over broad refactors.
- After merging issue or PR work into `main`, verify `docs/roadmap.md` against
  live GitHub issue and PR state before handoff. If the merge closed, unblocked,
  or superseded roadmap items, update the roadmap on `main`, commit, and push
  that follow-through.
- For non-trivial Rust changes, use `.agents/skills/recite-rust-quality/SKILL.md` to review maintainability, module boundaries, duplication, visibility, validation ownership, and file-size triggers before handoff.
- Test the crate(s) or workspace surface changed. The current repo is a workspace; use `cargo test` for broad changes unless a narrower crate check is clearly sufficient.
- Follow the Rust test organization policy in `.agents/skills/recite-testing-diagnostics/SKILL.md`; PR gates fail if tests are added in the wrong location.
- Repo-local skills must be Recite-specific overlays or Recite domain guidance. Put reusable personal workflow skills in the global agent config instead.

## Agent Workflow Routing

- Ordinary issue work routes through `.agents/skills/recite-github-pm/SKILL.md`.
- Parallel issue work routes through
  `.agents/skills/recite-parallel-issue-orchestration/SKILL.md`, with the
  global `workflow-roles` skill supplying portable role contracts.
- Clean diff review uses the global `code-review` skill plus the relevant
  Recite domain or language skill in a fresh reviewer context.
- Roadmap ownership and final protected merges remain with the coordinating
  main session; workers keep `docs/roadmap.md` read-only unless their issue
  explicitly includes it.

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
- Multi-agent roadmap or milestone orchestration: `.agents/skills/recite-parallel-issue-orchestration/SKILL.md`
- Rust maintainability, architecture, DRY, visibility, validation ownership, and file-size review triggers: `.agents/skills/recite-rust-quality/SKILL.md`
- Parser, AST, compiler, runtime, schema, effects, localisation IDs, and deterministic dialogue semantics: `.agents/skills/recite-core-language/SKILL.md`
- Fixtures, snapshots, diagnostics, CLI checks, LSP behavior, and headless runtime tests: `.agents/skills/recite-testing-diagnostics/SKILL.md`

For agent-facing instruction edits, use the global `agent-instructions` skill when available. In this repo, keep `AGENTS.md` to workflow, product invariants, spec routing, and Recite-specific skill pointers.
