# Contributing

Recite is public source, but isn't accepting external code contributions yet while v1 architecture settles, to minimise churn.

Issues, questions, and design feedback are still welcome on GitHub, especially where they clarify real authoring, localisation, runtime, or tooling needs. Unsolicited external code pull requests may be auto-closed for the time being; invited maintainer work follows the repository workflow below.

After the v1 shape is stable, I'll review and publish fuller contribution guidelines covering pull request scope, tests, review expectations, compatibility policy, and release process.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The production spec is in `docs/recite-production-spec.md`.
- Current development is issue-led and branch-based, using short-lived,
  purpose-first branches from `main` under `feat/`, `fix/`, `refactor/`,
  `perf/`, `ci/`, `docs/`, `test/`, `build/`, `chore/`, `spike/`, `release/`,
  `security/`, or `integration/`; do not prefix a branch with an issue number.
  For milestone
  work, the coordinator creates one purpose-first
  `integration/<short-kebab-topic>` branch from `main`. Bounded slices use
  isolated normal purpose-first branches or worktrees based on it, do not open
  issue-slice pull requests, and are reviewed and
  mechanically integrated by the coordinator. At a stable checkpoint, exactly
  one protected integration pull request targets `main`; apply the
  `workflow/integration` label and use the milestone tracking issue in its
  title. Commit subjects always begin with `[REC-N]` and a concise
  conventional-commit-style subject, with at most one explanatory body
  sentence and no agent-attribution trailers.
- The canonical local quality gate is `mise run verify`
  (`scripts/verify.sh`). It loads the scoped `maintainability` mise
  environment for the pinned ast-grep check. GitHub Actions runs separate Git
  policy, Rust, documentation, benchmark, and maintainability lanes, followed
  by the required-check rollup, on every push to `main` and on pull requests
  (`.github/workflows/ci.yml`); required CI and branch protection remain
  authoritative for the final protected PR. Focused checks are acceptable for
  narrow documentation or instruction-only changes; run the full gate locally
  for broad or high-risk code changes.
