# Contributing

Recite is public source, but isn't accepting external code contributions yet while v1 architecture settles, to minimise churn.

Issues, questions, and design feedback are still welcome on GitHub, especially where they clarify real authoring, localisation, runtime, or tooling needs. Unsolicited external code pull requests may be auto-closed for the time being; invited maintainer work follows the repository workflow below.

After the v1 shape is stable, I'll review and publish fuller contribution guidelines covering pull request scope, tests, review expectations, compatibility policy, and release process.

## Maintainer setup

From a checkout, install the pinned tools and run the complete check:

```sh
mise install
mise exec -- just check
```

`just check` installs the JavaScript workspace dependencies from the frozen
lockfile before using them. It checks Rust, editor packages, adapters, docs,
dependency policy, spelling, and benchmark smoke. `just verify` and
`mise run verify` run that same gate. With mise activated in your shell, the
`mise exec --` prefix is unnecessary. Run `just` to list the commands.

For a focused change:

```sh
just fmt
just clippy
just test -p recite-runtime -E 'test(restore)'
just test-doc -p recite-runtime
just test-zed
just supply-chain
```

`fmt` deliberately rewrites Rust and maintained toolchain TOML files;
`fmt-check` only checks them. The normal test recipe uses nextest; doctests have
their own recipe and remain part of the complete gate. The Zed extension is a
separate Cargo workspace, covered by the format, lint, and editor gates.
`supply-chain` checks both dependency graphs for advisories, licenses, bans,
and sources. `unused-deps` checks both workspaces with cargo-machete.
The existing Godot bindings and `option-ext` retain their MPL-2.0 licenses;
the dependency policy names those packages explicitly. Recite's own source
remains MIT OR Apache-2.0. Distributions must retain dependency notices and
the MPL source availability obligations described in the
[Mozilla FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

The complete check needs network access for dependency installation and the
advisory database. Rust and Unity headless tests require no database or running
game. The complete adapter gate also runs a temporary headless Godot project;
its pinned 4.6.3 host is provisioned by the scoped `mise.godot.toml` environment.
That host lane currently requires Linux x86_64. Other platform-host and expensive
stress evidence have explicit commands:

```sh
just test-stress --nocapture
just test-watch-stress --nocapture
just test-godot
just test-editor-host neovim
just test-editor-host vscode
just test-editor-host zed
```

Host commands fail when their prerequisites are absent. The VS Code/VSCodium
and Zed runners currently require Linux x86_64, Cage, wtype, and official host
downloads; see their `--help` output and `docs/editor-parity-contract.md` for
the evidence boundaries. These runs do not establish other-platform support
or replace the release benchmark baseline.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The production spec is in `docs/recite-production-spec.md`.
- The trusted pull-request policy in `.github/workflows/trusted-policy.yml`
  runs base-owned policy code with read-only permissions. It fetches proposed
  commits as Git objects for metadata checks and never checks out or executes
  pull-request files. Keep that boundary intact when changing workflow or
  policy files; ordinary CI remains a separate, untrusted pull-request lane.
  The repository's deterministic fixture gate also performs static workflow
  assertions. Run `actionlint` locally when it is available; it is not
  installed by the repository toolchain, so CI records static coverage rather
  than downloading an unpinned validator.
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
- The canonical local quality gate is `mise exec -- just check`
  (`scripts/verify.sh`). It loads the scoped `maintainability` mise
  environment for the pinned ast-grep check. GitHub Actions runs separate Git
  policy, Rust, documentation, benchmark, and maintainability lanes, followed
  by the required-check rollup, on every push to `main` and on pull requests
  (`.github/workflows/ci.yml`). The base-owned trusted policy lane is a separate
  `pull_request_target` check (`.github/workflows/trusted-policy.yml`);
  required CI and branch protection remain authoritative for the final
  protected PR. Focused checks are acceptable for narrow documentation or
  instruction-only changes; run the full gate locally for broad or high-risk
  code changes.
