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
`mise exec --` prefix is unnecessary. Run `just` to list the cross-project
commands and modules, or `just web`, `just writer`, `just editor`,
`just engines`, `just stress`, or `just perf` to list one area's commands. Each module is
also callable from its own directory without the area name, for example
`cd apps/writer && just run --project /path/to/project`.

For a focused change:

```sh
just fmt
just clippy
just test -p recite-runtime -E 'test(restore)'
just test-doc -p recite-runtime
just editor zed test
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
advisory database. Its isolated fixture builds also need temporary disk space.
If `/tmp` is a small RAM-backed filesystem, use `TMPDIR=/var/tmp just check`;
the temporary directory must be outside the checkout so isolation checks and
editor project discovery retain their intended boundaries.
Rust and Unity headless tests require no database or running
game. The complete adapter gate also runs a temporary headless Godot project;
its pinned 4.6.3 host is provisioned by the scoped `mise.godot.toml` environment.
That host lane currently requires Linux x86_64. Other platform-host and expensive
stress evidence have explicit commands:

```sh
just stress scale --nocapture
just stress watch --nocapture
just engines godot
just editor host neovim
just editor host vscode
just editor host zed
```

Host commands fail when their prerequisites are absent. The VS Code/VSCodium
and Zed runners currently require Linux x86_64, Cage, wtype, and official host
downloads; see their `--help` output and `docs/editor-parity-contract.md` for
the evidence boundaries. These runs do not establish other-platform support
or replace the release benchmark baseline.

## Performance tooling

Criterion remains the Rust timing suite. Maintained external LSP probes live in
`scripts/lsp_tools/`, with one CLI and a scoped, locked Python 3.12 environment.
The pinned uv runner owns environment setup; psutil and Ruff are locked in
`uv.lock`. Ordinary Rust work does not need that environment.

```sh
just perf setup
just perf check
just perf bench lsp large,realistic:v1-pack 'lsp/change_refresh'
just perf compare BASE_COMMIT
just perf build
just perf lsp --help
just maintainability
```

`perf check` enforces Python formatting/lints and the CI contract tests; the
complete `just check` includes it. `perf fmt` deliberately formats the maintained
LSP tools. The
[current LSP overview](docs/lsp-cancellation-design.md) and
[profiling playbook](docs/profiling-and-optimisation.md) explain ownership,
measurement boundaries and the dependency/maintainability tradeoff.
Completed diagnostic experiments are archived at named revisions rather than
kept as permanent workflow modes. Bash launches tools and handles platform shell
operations; Python owns structured external-process measurements and report
checks. Reuse production Rust APIs for domain semantics instead of reproducing
them in either language.

Python is retained provisionally for the already validated LSP harness. The
[language assessment](docs/lsp-dependency-decisions.md#maintainer-tooling-language)
compares Rust, Go, Python and Node, with bounded probes and explicit learning,
setup and maintenance costs. Start substantial new general tooling with a
private Rust tool crate; evaluate Rust and Go before extending the external
harness's ownership. A replacement must reduce total maintenance or material
driver interference while preserving the gate. Expand Node only for a concrete
editor/frontend need.

Concluded reports and raw assets live together in [the archive](docs/archive/README.md).
Keep current ownership, commands, limits and reopening conditions in their
existing guides. Historical milestone counts and prose are not CI contracts.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary content, copied private material, or dependency code that is incompatible with that distribution.
- The [production specification](docs/recite-production-spec.md) routes to
  subsystem contracts in `docs/spec/`. Read the affected chapter; GitHub owns
  implementation task state, and historical measurements remain separate.
- LSP dependency choices, experiments and reevaluation triggers are recorded in
  [the dependency decision record](docs/lsp-dependency-decisions.md).
- The trusted pull-request policy in `.github/workflows/trusted-policy.yml`
  runs base-owned policy code with read-only permissions. It fetches proposed
  commits as Git objects for metadata checks and never checks out or executes
  pull-request files. Keep that boundary intact when changing workflow or
  policy files; ordinary CI remains a separate, untrusted pull-request lane.
  The repository's deterministic fixture gate also performs static workflow
  assertions. Pinned `actionlint` validates workflow syntax, expressions and
  reusable-workflow wiring in CI and the complete local gate.
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
  environment for the pinned ast-grep check. GitHub Actions selects affected
  lanes on pushes to `main` and pull requests (`.github/workflows/ci.yml`),
  then validates their results with the unconditional `required-check` rollup.
  The base-owned trusted policy lane is a separate
  `pull_request_target` check (`.github/workflows/trusted-policy.yml`);
  required CI and branch protection remain authoritative for the final
  protected PR. Focused checks are acceptable for narrow documentation or
  instruction-only changes; run the full gate locally for broad or high-risk
  code changes.

## CI coverage

`scripts/ci-scope.py` selects checks using the complete Git diff, including
deleted paths and both sides of renames. Pull requests use their merge base;
pushes compare the previous and current commit. Unknown paths and shared Cargo
manifests or locks select the complete suite; known toolchain edits select their
consumers. A missing comparison revision
fails the required check instead of silently skipping coverage.

| Changed surface | Selected checks, in addition to policy, spelling, workflow validation and CI fixtures |
| --- | --- |
| Markdown and documentation | Documentation build and schema examples |
| Core Rust | Rust, adapters, writer UI/accessibility, Windows contracts, editor clients, benchmark smoke, maintainability |
| Writer source | Rust, adapters, writer UI/accessibility, maintainability |
| VS Code or Helix | Editor clients and maintainability |
| Just recipe layout | Maintainability recipe checks; changed code or scripts select their own lanes |
| Engine companion scripts | Rust adapter gate and maintainability |
| CI routing contracts | Unconditional policy fixtures and maintainability |
| Schema and shared fixtures | Rust, Windows, docs, editor clients, benchmark smoke, maintainability |
| Packaging definitions or assets | Native, Nix and Flatpak packages, docs, maintainability |
| Cargo manifests/locks, broad shared toolchain edits, unknown inputs | Complete suite, including packages |

The Rust lane retains the existing writer tests and Linux native accessibility
probe. Source changes can still reveal platform-specific packaging failures in
the weekly run; run **Writer package previews** manually before a release or
when changing platform-dependent source. All ten package jobs run for changed
packaging/build inputs, on the Monday 05:23 UTC complete run, and on demand.
The **CI** workflow also supports a manual complete run. These jobs build and
inspect packages; they do not replace installed-package or manual accessibility
acceptance.

Title, body and label edits rerun only the trusted policy workflow. Its
base-owned checks validate current metadata and the commit range; source CI is
not restarted. Changes to source still cancel superseded runs.

The required rollup accepts a skipped lane only when selection explicitly says
it is unaffected. A failed selector, missing result, cancellation, or unexpected
skip blocks the rollup. Package results are included through the reusable
workflow, so package failures cannot leave the aggregate green.

Inspect selection locally and run its regression tests with:

```sh
python3 scripts/ci-scope.py --base origin/main --head HEAD
python3 -m unittest discover -s tests/ci -p 'test_*.py'
```
