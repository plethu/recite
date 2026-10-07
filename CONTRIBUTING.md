# Contributing

Recite is public source, but isn't accepting external code contributions yet while v1 architecture
settles, to minimise churn.

Issues, questions, and design feedback are still welcome on GitHub, especially where they clarify
real authoring, localisation, runtime, or tooling needs. Unsolicited external code pull requests may
be auto-closed for the time being; invited maintainer work follows the repository workflow below.

After the v1 shape is stable, I'll review and publish fuller contribution guidelines covering pull
request scope, tests, review expectations, compatibility policy, and release process.

## Maintainer setup

From a checkout, install the pinned tools and run the complete check:

```sh
mise install
mise exec -- just setup
mise exec -- just check
```

`just check` installs the JavaScript workspace dependencies from the frozen lockfile before using
them. It checks Rust, editor packages, adapters, docs, dependency policy, spelling, and benchmark
smoke. `just verify` and `mise run verify` run that same gate. With mise activated in your shell,
the `mise exec --` prefix is unnecessary. Run `just` to list the cross-project commands and modules,
or `just web`, `just writer`, `just editor`, `just engines`, `just stress`, or `just perf` to list
one area's commands. Each module is also callable from its own directory without the area name, for
example `cd apps/writer && just run --project /path/to/project`.

For a focused change:

```sh
just fmt
just clippy
just test -p recite-runtime -E 'test(restore)'
just test-doc -p recite-runtime
just editor zed test
just supply-chain
```

`fmt` deliberately formats maintained sources; `fmt-check` checks them without rewriting. `lint`
runs the configured language linters and Rust Clippy. The quality lane also runs for
documentation-only contributions. Tools are version-pinned in `.mise.toml` and scoped mise
configurations; `just setup` installs them and locked workspace dependencies. The normal test recipe
uses nextest; doctests have their own recipe and remain part of the complete gate. The Zed extension
is a separate Cargo workspace, covered by the format, lint, and editor gates. `supply-chain` checks
both dependency graphs for advisories, licenses, bans, and sources. `unused-deps` checks both
workspaces with cargo-machete. The existing Godot bindings and `option-ext` retain their MPL-2.0
licenses; the dependency policy names those packages explicitly. Recite's own source remains MIT OR
Apache-2.0. Distributions must retain dependency notices and the MPL source availability obligations
described in the [Mozilla FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).

The complete check needs network access for dependency installation and the advisory database. Its
isolated fixture builds also need temporary disk space. If `/tmp` is a small RAM-backed filesystem,
use `TMPDIR=/var/tmp just check`; the temporary directory must be outside the checkout so isolation
checks and editor project discovery retain their intended boundaries. Rust and Unity headless tests
require no database or running game. The complete adapter gate also runs a temporary headless Godot
project; its pinned 4.6.3 host is provisioned by the scoped `mise.godot.toml` environment. That host
lane currently requires Linux x86_64. Other platform-host and expensive stress evidence have
explicit commands:

```sh
just stress scale --nocapture
just stress watch --nocapture
just engines godot
just editor host neovim
just editor host vscode
just editor host zed
```

Host commands fail when their prerequisites are absent. The VS Code/VSCodium and Zed runners
currently require Linux x86_64, Cage, wtype, and official host downloads; see their `--help` output
and `docs/editor-parity-contract.md` for the evidence boundaries. These runs do not establish
other-platform support or replace the release benchmark baseline.

## Formatting and validation

Versioned configurations at the repository root define formatting and lint rules. `just fmt`, `just
fmt-check` and `just lint` provide common commands; `just quality` lists the non-Rust lane.

| Source                                                                 | Formatter               | Lint or semantic gate                                      |
| ---------------------------------------------------------------------- | ----------------------- | ---------------------------------------------------------- |
| Rust                                                                   | rustfmt                 | Clippy, ast-grep, domain tests                             |
| Markdown, JSON, YAML, JavaScript/TypeScript, CSS, SVG/XML, Dockerfiles | dprint                  | rumdl, Oxlint, type checks, Stylelint, schemas, actionlint |
| TOML                                                                   | Tombi, offline          | Tombi syntax and owning domain checks                      |
| Python                                                                 | Ruff                    | Ruff and harness tests                                     |
| Shell                                                                  | shfmt                   | ShellCheck                                                 |
| Lua                                                                    | StyLua                  | Lua language server and Neovim tests                       |
| GDScript                                                               | gdformat                | gdlint and Godot adapter tests                             |
| C/C++                                                                  | clang-format            | FFI compilation with warnings as errors                    |
| C#                                                                     | dotnet format           | Headless Unity compilation/tests                           |
| Nix                                                                    | Alejandra               | Flake/package checks                                       |
| PowerShell                                                             | PSScriptAnalyzer        | PSScriptAnalyzer                                           |
| Astro components                                                       | dprint                  | Astro type checks and site browser tests                   |
| Just recipes                                                           | just's native formatter | Recipe and workflow checks                                 |

Recite source, Fluent, gettext PO and Tree-sitter queries use their parser, typed-contract and
fixture gates. They are validated, not automatically reformatted: whitespace and deliberately
incomplete inputs can be test data. Generated files and frozen historical evidence are excluded
explicitly in the formatter configurations; their generators or owning tests check them. Do not edit
dependency/theme sources to satisfy our style rules.

The [site](docs-site/README.md) uses Astro/Starlight with mise-managed Node/pnpm for builds and
checks. Its output is static HTML, CSS and browser modules; hosting needs no Node server or UI
framework runtime. Python wheels used to distribute formatters are tools, not new application
modules.

## Performance tooling

Criterion remains the Rust timing suite. Maintained external LSP probes live in
`scripts/lsp_tools/`, with one CLI and a scoped, locked Python 3.12 environment. The pinned uv
runner owns environment setup; psutil is locked in `uv.lock`. Ruff belongs to the shared mise
quality toolchain.

```sh
just perf setup
just perf check
just perf bench lsp large,realistic:v1-pack 'lsp/change_refresh'
just perf compare BASE_COMMIT
just perf build
just perf lsp --help
just maintainability
```

`perf check` runs harness and CI contract tests; the complete `just check` includes it. Python
formatting and linting belong to the common quality commands. The
[current LSP overview](docs/lsp-cancellation-design.md) and
[profiling playbook](docs/profiling-and-optimisation.md) explain ownership, measurement boundaries
and the dependency/maintainability tradeoff. Completed diagnostic experiments are archived at named
revisions rather than kept as permanent workflow modes. Bash launches tools and handles platform
shell operations; Python owns structured external-process measurements and report checks. Reuse
production Rust APIs for domain semantics instead of reproducing them in either language.

Python is retained provisionally for the already validated LSP harness. The
[language assessment](docs/lsp-dependency-decisions.md#maintainer-tooling-language) compares Rust,
Go, Python and Node, with bounded probes and explicit learning, setup and maintenance costs. Start
substantial new general tooling with a private Rust tool crate; evaluate Rust and Go before
extending the external harness's ownership. A replacement must reduce total maintenance or material
driver interference while preserving the gate. Expand Node only for a concrete editor/frontend need.

Keep current ownership, commands, limits and reopening conditions in their existing guides.
Concluded reports belong in Git or PR history; retain raw assets only when they support a decision
or reproduction. Historical milestone counts and prose are not CI contracts.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request
  operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary
  content, copied private material, or dependency code that is incompatible with that distribution.
- The [production specification](docs/recite-production-spec.md) routes to subsystem contracts in
  `docs/spec/`. Read the affected chapter; GitHub owns implementation task state, and historical
  measurements remain separate.
- LSP dependency choices, experiments and reevaluation triggers are recorded in
  [the dependency decision record](docs/lsp-dependency-decisions.md).
- The trusted pull-request policy in `.github/workflows/trusted-policy.yml` runs base-owned policy
  code with read-only permissions. It fetches proposed commits as Git objects for metadata checks
  and never checks out or executes pull-request files. Keep that boundary intact when changing
  workflow or policy files; ordinary CI remains a separate, untrusted pull-request lane. The
  repository's deterministic fixture gate also performs static workflow assertions. Pinned
  `actionlint` validates workflow syntax, expressions and reusable-workflow wiring in CI and the
  complete local gate.
- The canonical local quality gate is `mise exec -- just check` (`scripts/verify.sh`). It loads the
  scoped `maintainability` mise environment for the pinned ast-grep check. GitHub Actions selects
  affected lanes on pushes to `main` and pull requests (`.github/workflows/ci.yml`), then validates
  their results with the unconditional `required-check` rollup. The base-owned trusted policy lane
  is a separate `pull_request_target` check (`.github/workflows/trusted-policy.yml`); required CI
  and branch protection remain authoritative for the final protected PR. Focused checks are
  acceptable for narrow documentation or instruction-only changes; run the full gate locally for
  broad or high-risk code changes.

## Change and review workflow

Start standalone work from `main` on a short-lived `<kind>/<short-kebab-topic>` branch. Supported
kinds are `feat`, `fix`, `refactor`, `perf`, `ci`, `docs`, `test`, `build`, `chore`, `spike`,
`release`, `security` and `integration`. Never prefix a branch with an issue number.

Commit subjects begin with `[REC-N] <type>(optional-scope): <subject>`, with at most one explanatory
body sentence and no agent-attribution trailers. Run `scripts/check-git-policy.sh` locally.
Standalone PR titles use the same issue code as every commit in their range; the body includes
`Closes #N`, `Fixes #N` or `Resolves #N` matching that title.

Milestone work uses a coordinator-owned `integration/<short-kebab-topic>` branch from `main`.
Delegated slices use isolated purpose-first branches/worktrees at its stated base SHA, do not open
slice PRs, and are reviewed before mechanical integration. The final PR targets `main`, carries
`workflow/integration`, and uses the milestone tracking issue's code in its title and closing token.
Its commits may address multiple issues; list accepted slices in the PR. Agent delegation procedures
live in the [GitHub workflow skill](.agents/skills/recite-github-pm/SKILL.md#authorized-delegation).

Protected `main` requires signed commits, required CI and resolved review threads. Maintainer
approval remains authoritative; optional automated reviews are advisory. The
[merge helper reference](.agents/skills/recite-github-pm/references/github-merge-details.md)
explains the current-head checks and solo-maintainer approval path. Verify linked issue and
milestone state after merging. Do not bypass this path with direct pushes to `main`.

## Instructions and skills

`AGENTS.md` routes agents to repository contracts and the four Recite-specific skills under
`.agents/skills/`. Humans can use those procedures directly; no personal skill collection or
machine-local instruction file is required. Contributor requirements belong here or in their owning
contract, and enforceable rules belong in versioned configuration and checks.

Maintainers may keep general skills, agent/model preferences and personal working agreements in
their own agent configuration. Do not reference them as repository prerequisites, import a
maintainer's home-directory instructions, or copy a general skill collection into this checkout.

## CI coverage

`scripts/ci-scope.py` selects checks using the complete Git diff, including deleted paths and both
sides of renames. Pull requests use their merge base; pushes compare the previous and current
commit. Unknown paths and shared Cargo manifests or locks select the complete suite; known toolchain
edits select their consumers. A missing comparison revision fails the required check instead of
silently skipping coverage.

| Changed surface                                                     | Selected checks, in addition to policy, spelling, workflow validation and CI fixtures                        |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Markdown and documentation                                          | Documentation build, schema examples and maintainability                                                     |
| Core Rust                                                           | Rust, adapters, writer UI/accessibility, Windows contracts, editor clients, benchmark smoke, maintainability |
| Writer source                                                       | Rust, adapters, writer UI/accessibility, maintainability                                                     |
| VS Code or Helix                                                    | Editor clients and maintainability                                                                           |
| Just recipe layout                                                  | Maintainability recipe checks; changed code or scripts select their own lanes                                |
| Engine companion scripts                                            | Rust adapter gate and maintainability                                                                        |
| CI routing contracts                                                | Unconditional policy fixtures and maintainability                                                            |
| Schema and shared fixtures                                          | Rust, Windows, docs, editor clients, benchmark smoke, maintainability                                        |
| Packaging definitions or assets                                     | Native, Nix and Flatpak packages, docs, maintainability                                                      |
| Cargo manifests/locks, broad shared toolchain edits, unknown inputs | Complete suite, including packages                                                                           |

The Rust lane retains the existing writer tests and Linux native accessibility probe. Source changes
can still reveal platform-specific packaging failures in the weekly run; run **Writer package
previews** manually before a release or when changing platform-dependent source. All ten package
jobs run for changed packaging/build inputs, on the Monday 05:23 UTC complete run, and on demand.
The **CI** workflow also supports a manual complete run. These jobs build and inspect packages; they
do not replace installed-package or manual accessibility acceptance.

Title, body and label edits rerun only the trusted policy workflow. Its base-owned checks validate
current metadata and the commit range; source CI is not restarted. Changes to source still cancel
superseded runs.

The required rollup accepts a skipped lane only when selection explicitly says it is unaffected. A
failed selector, missing result, cancellation, or unexpected skip blocks the rollup. Package results
are included through the reusable workflow, so package failures cannot leave the aggregate green.

Inspect selection locally and run its regression tests with:

```sh
python3 scripts/ci-scope.py --base origin/main --head HEAD
python3 -m unittest discover -s tests/ci -p 'test_*.py'
```
