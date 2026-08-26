# Contributing

Recite is public source, but isn't accepting external code contributions yet while v1 architecture settles, to minimise churn.

Issues, questions, and design feedback are still welcome on GitHub, especially where they clarify real authoring, localisation, runtime, or tooling needs. Pull requests will be auto-closed for the time being.

After the v1 shape is stable, I'll review and publish fuller contribution guidelines covering pull request scope, tests, review expectations, compatibility policy, and release process.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The production spec is in `docs/recite-production-spec.md`.
- Current development is issue-led and branch-based, using short-lived
  purpose-first branches from `main` under `feat/`, `fix/`, `refactor/`,
  `perf/`, `ci/`, `docs/`, `test/`, `build/`, `chore/`, `spike/`, `release/`,
  or `security/`; do not prefix a branch with an issue number. Commit subjects
  begin with `[REC-N]` and a concise conventional-commit-style subject, with at
  most one explanatory body sentence and no agent-attribution trailers.
- The canonical quality gate is `mise run verify` (`scripts/verify.sh`). GitHub Actions runs the same gate on every push to `main` and on pull requests (`.github/workflows/ci.yml`); run it locally before pushing. `scripts/check-project-gates.sh` remains the focused Rust and adapter subset.
