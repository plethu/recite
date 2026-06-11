# Contributing

Recite is public source, but isn't accepting external code contributions yet while v1 architecture settles, to minimise churn.

Issues, questions, and design feedback are still welcome on Codeberg, especially where they clarify real authoring, localisation, runtime, or tooling needs. Pull requests will be auto-closed for the time being.

After the v1 shape is stable, I'll review and publish fuller contribution guidelines covering pull request scope, tests, review expectations, compatibility policy, and release process.

## Project Notes

- Recite is hosted on Codeberg. Do not use GitHub workflows for this project.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The production spec is in `docs/recite-production-spec.md`.
- Current development is issue-led and branch-based, using short-lived branches from `main`.
- The canonical quality gate is `scripts/check-project-gates.sh`. Forgejo Actions runs the same script on every push to `main` and on pull requests (`.forgejo/workflows/ci.yml`); run it locally before pushing.
