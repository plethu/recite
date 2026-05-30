# Contributing

Recite is public source, but it is not accepting external code contributions yet.

The project is still settling its v1 architecture, language shape, and crate boundaries. Until that shape has stabilised, I am keeping implementation work tightly directed so the core design can stay coherent.

Issues, questions, and design feedback are still welcome on Codeberg, especially where they clarify real authoring, localisation, runtime, or tooling needs. Please do not open pull requests unless I have explicitly asked for one.

After the v1 shape is stable, I will review and publish fuller contribution guidelines covering pull request scope, tests, review expectations, compatibility policy, and release process.

## Project Notes

- Recite is hosted on Codeberg. Do not use GitHub workflows for this project.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The production spec is in `docs/recite-production-spec.md`.
- Current development is issue-led and branch-based, using short-lived branches from `main`.
- Until CI is wired up, the local quality gate is `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings`.
