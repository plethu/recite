# Recite Agent Instructions

Recite is a Rust-first deterministic dialogue compiler, runtime, and tooling project for
ECS-oriented games. It is dual-licensed public open source under MIT OR Apache-2.0; do not introduce
proprietary content, copied private material, or dependency code that is incompatible with that
distribution.

## Project Workflow

- Use GitHub as the canonical forge. Use the GitHub CLI (`gh`) with explicit `--repo plethu/recite`
  for issue, milestone, label, and pull-request work.
- Standalone work starts from `main` on a short-lived, purpose-first branch and follows the
  machine-wide branch naming convention; never prefix a branch with an issue number. Milestone work
  uses a coordinator-owned `integration/<short-kebab-topic>` branch; its delegated slices follow the
  GitHub PM skill and do not open issue-slice pull requests. Recite commit subjects begin with
  `[REC-N]`, followed by a concise conventional-commit-style subject.
- Run `scripts/check-git-policy.sh` locally. It is part of the complete verification gate and checks
  the relevant change range on pull requests; never add agent-attribution trailers.
- Keep patches scoped to the issue or user request.
- Do not revert unrelated user changes.
- Prefer small, reviewable changes over broad refactors.
- Track outstanding work in GitHub issues and milestones. After a merge, verify linked issue and
  milestone state before handoff.
- For non-trivial Rust changes, use the relevant Recite overlay, especially
  `.agents/skills/recite-rust-quality/SKILL.md`, and load the global `rust-quality` skill when it is
  available.
- The complete local gate is `just check` (`mise run verify` is an alias; use `mise exec -- just
  check` without shell activation). Use a narrower documented check only when the changed surface
  makes that sufficient.
- Follow the Rust test organization policy in `.agents/skills/recite-testing-diagnostics/SKILL.md`;
  PR gates fail if tests are added in the wrong location.
- Repo-local skills must be Recite-specific overlays or Recite domain guidance. Put reusable
  personal workflow skills in the global agent config instead.

## Agent Workflow Routing

- Issue planning and milestone integration route through `.agents/skills/recite-github-pm/SKILL.md`.
- Requested code review uses the global `code-review` skill and the relevant Recite skill. Follow
  session delegation rules; do not spawn reviews by default.
- Final protected merges remain with the coordinating main session.

## Product Invariants

- Runtime traversal must be deterministic.
- Runtime code must never perform game-side effects.
- Effects are typed, schema-checked requests emitted to the caller.
- Runtime state must be serializable without game state.
- Author-visible line and choice IDs must remain stable once written.
- Dialogue outputs, choices, metadata, effects, and diagnostics should be structured values, not
  prose conventions.
- Validation should catch malformed project content without running a game engine.
- Semantic changes should include tests unless the work is explicitly exploratory.

## Spec Authority

The [production specification](docs/recite-production-spec.md) is a routing hub. Read only the
affected chapter and subsections before changing that contract:

| Surface                         | Contract                                 |
| ------------------------------- | ---------------------------------------- |
| Product/invariants              | `docs/spec/product.md` §1–4              |
| Parser/source format            | `docs/spec/source.md` §5                 |
| Conditions/effects              | `docs/spec/conditions-effects.md` §6–7   |
| Runtime/localisation/stable IDs | `docs/spec/runtime-localisation.md` §8–9 |
| Schema                          | `docs/spec/schema.md` §10                |
| Scene manifests/compiler/CLI    | `docs/spec/build-cli.md` §11–13          |
| LSP/editors/engine adapters     | `docs/spec/tooling.md` §14–16            |
| Tests/diagnostics/performance   | `docs/spec/quality.md` §17–19            |
| Migration/scope/release gates   | `docs/spec/release.md` §20–24            |

Read current design summaries for ownership; open historical experiment reports only for a decision
being revisited. Do not load all skills, chapters or evidence as a preflight. GitHub owns
outstanding tasks. Update the owning contract when behaviour changes instead of appending a second
specification to a report.

## Repo-Local Skills

Use the relevant skill for procedural details:

- GitHub issues, milestones, labels, pull requests, and project planning:
  `.agents/skills/recite-github-pm/SKILL.md`
- Recite-specific Rust maintainability, diagnostics, FFI, and file-size review triggers:
  `.agents/skills/recite-rust-quality/SKILL.md`
- Parser, AST, compiler, runtime, schema, effects, localisation IDs, and deterministic dialogue
  semantics: `.agents/skills/recite-core-language/SKILL.md`
- Fixtures, snapshots, diagnostics, CLI checks, LSP behavior, and headless runtime tests:
  `.agents/skills/recite-testing-diagnostics/SKILL.md`

For agent-facing instruction edits, use the global `agent-instructions` skill when available. In
this repo, keep `AGENTS.md` to workflow, product invariants, spec routing, and Recite-specific skill
pointers.
