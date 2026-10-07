# Recite Agent Instructions

Recite is a Rust-first deterministic dialogue compiler, runtime, and tooling project for
ECS-oriented games, dual-licensed MIT OR Apache-2.0. Do not introduce incompatible dependencies,
proprietary content or copied private material.

## Workflow

- Follow [CONTRIBUTING.md](CONTRIBUTING.md) for setup, branch and commit conventions, contribution
  policy and protected merges. Use `gh` with explicit `--repo plethu/recite` for forge work.
- Keep changes within the request and preserve unrelated user edits. Commits, pushes, review
  requests and merges must stay within the authorized delivery stages.
- Run `scripts/check-git-policy.sh` locally. The complete gate is `just check` (`mise run verify` is
  an alias; use `mise exec -- just check` without shell activation). Use focused checks for narrow
  changes and the complete gate for broad or high-risk changes.
- GitHub owns outstanding tasks and milestone state. Verify linked issue and milestone state after
  an authorized merge.
- Repository guidance must work from a fresh checkout without personal skills or machine-local
  instruction files. Keep general personal workflows in the maintainer's own configuration.

## Product Invariants

- Runtime traversal is deterministic; game-side effects stay outside the runtime.
- Effects are typed, schema-checked requests emitted to the caller.
- Runtime state is serializable without game state.
- Author-visible line and choice IDs remain stable once written.
- Outputs, choices, metadata, effects and diagnostics are structured values.
- Validation catches malformed content without running a game engine.
- Semantic changes include tests unless explicitly exploratory.

## Contracts and Release

The [production specification](docs/recite-production-spec.md) routes to subsystem contracts. Read
only the affected chapter and subsections before changing that contract. Update its existing owner
when behavior changes; do not append a second specification to an experiment report. Consult
historical evidence only when revisiting its decision.

Recite remains pre-release. Before recommending 1.0, consult [release gates](docs/spec/release.md)
§22–23 and name remaining consumer and compatibility evidence gaps. Passing automated checks alone
does not establish readiness; developer previews remain appropriate while those boundaries are being
exercised.

## Task Routing

| Task                                                     | Repository skill                                                              |
| -------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Rust implementation and maintainability                  | [Rust quality](.agents/skills/recite-rust-quality/SKILL.md)                   |
| Parser, compiler, runtime, schema and language semantics | [Core language](.agents/skills/recite-core-language/SKILL.md)                 |
| Tests, fixtures, diagnostics, LSP and benchmarks         | [Testing and diagnostics](.agents/skills/recite-testing-diagnostics/SKILL.md) |
| GitHub planning, milestone integration and merge checks  | [GitHub workflow](.agents/skills/recite-github-pm/SKILL.md)                   |

Load relevant skills only. For reviews, use the relevant domain skill and inspect the final diff,
callers and verification evidence. Follow session delegation rules; do not spawn reviews by default.
Keep this file to workflow, invariants and task routing, and repo-local skills to Recite-specific
procedures.
