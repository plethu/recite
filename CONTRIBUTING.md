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

Versioned configurations own formatter/linter selection and coverage. Use the common commands above;
`just quality` lists the non-Rust lane.

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

Use [profiling and optimisation](docs/profiling-and-optimisation.md) for benchmark selection,
process measurements, CPU sampling and allocation probes. `just perf` lists commands; its harness
checks run in `just check`. The [LSP overview](docs/lsp-cancellation-design.md) owns concurrency,
and [dependency decisions](docs/lsp-dependency-decisions.md#maintainer-tooling-language) own harness
language choices and conditions for replacing or extending that owner.

Keep current ownership, commands, limits and reopening conditions in their existing guides.
Concluded reports belong in Git or PR history. Raw captures, benchmark output and disposable probes
belong under ignored `target/` or a local archive, rather than in the maintained source tree. Keep
shared regression inputs in their fixture owner. Historical milestone counts and prose are not CI
contracts.

## Project Notes

- Recite is hosted on GitHub. Use `gh` with `--repo plethu/recite` for issue and pull-request
  operations.
- Recite is dual-licensed public open source under MIT OR Apache-2.0. Do not submit proprietary
  content, copied private material, or dependency code that is incompatible with that distribution.
- The [production specification](docs/recite-production-spec.md) routes to subsystem contracts in
  `docs/spec/`. Read the affected chapter; GitHub owns implementation task state, and historical
  measurements remain separate.

## Change and review workflow

Start standalone work from `main` on a short-lived `<kind>/<short-kebab-topic>` branch. Supported
kinds are `feat`, `fix`, `refactor`, `perf`, `ci`, `docs`, `test`, `build`, `chore`, `spike`,
`release`, `security` and `integration`. Never prefix a branch with an issue number.

Commit subjects begin with `[REC-N] <type>(optional-scope): <subject>`, with at most one explanatory
body sentence and no agent-attribution trailers. Run `scripts/check-git-policy.sh` locally.
Standalone PR titles use the same issue code as every commit in their range; the body includes
`Closes #N`, `Fixes #N`, `Resolves #N`, `Refs #N` or `References #N` matching that title. Use a
closing reference when the PR completes the issue; use a nonclosing reference when work remains
after merge.

Milestone work uses a coordinator-owned `integration/<short-kebab-topic>` branch from `main`.
Delegated slices use isolated purpose-first branches/worktrees at its stated base SHA, do not open
slice PRs, and are reviewed before mechanical integration. The final PR targets `main`, carries
`workflow/integration`, and uses the milestone tracking issue's code in its title and closing token.
Its commits may address multiple issues; list the included changes in the PR.

Protected `main` requires signed commits, current required CI and resolved review threads. Inspect
the final diff and address review findings before merging. Use native GitHub checks and protection:

```sh
gh pr view PR --repo plethu/recite
gh pr checks PR --repo plethu/recite --required
gh pr merge PR --repo plethu/recite --squash --delete-branch --match-head-commit REVIEWED_SHA
```

Merge only when authorized, against the reviewed head. Optional automated reviews are advisory; they
do not replace maintainer approval or cover later changes. Verify linked issue and milestone state
after merging. Do not bypass protection with direct pushes to `main`.

## Instructions and skills

`AGENTS.md` routes agents to repository contracts and the four Recite-specific skills under
`.agents/skills/`. Humans can use those procedures directly; no personal skill collection or
machine-local instruction file is required. Contributor requirements belong here or in their owning
contract, and enforceable rules belong in versioned configuration and checks.

Maintainers may keep general skills, agent/model preferences and personal working agreements in
their own agent configuration. Do not reference them as repository prerequisites, import a
maintainer's home-directory instructions, or copy a general skill collection into this checkout.

## CI coverage

[`scripts/ci-scope.py`](scripts/ci-scope.py) and its tests own affected-lane selection. Inspect it
with `just perf scope` and verify it with `just perf check`. The unconditional `required-check`
rollup rejects missing, failed, cancelled and unexpected skipped results. Trusted policy executes
base-owned code with read-only permissions, separately from ordinary untrusted CI.

Workflow definitions own lanes, schedules and manual complete/package runs. Run package previews
before release or after platform-dependent changes. Installed package and native accessibility
acceptance require their own evidence.

## Preparing and publishing releases

Use [SemVer 2.0](https://semver.org/). Before 1.0, bump the minor version for incompatible public
changes and the patch for compatible fixes. From 1.0, use major/minor/patch for breaking
changes/additions/fixes. The product tag is `vVERSION`; core crates share the root version, while
Writer and the FFI crate retain their own version groups. ABI and persisted-format counters change
only for their respective compatibility rules.

Number previews explicitly: `0.2.0-beta.1`, `0.2.0-beta.2`, `0.2.0-rc.1`, `0.2.0-rc.2`, then
`0.2.0`. Alpha is available for earlier development. Beta invites feedback; RC freezes intended
scope while addressing release blockers. Changing code or an embedded version requires a new
candidate run. Published versions and signed tags are immutable; corrections get a new version. Do
not use build metadata to distinguish releases.

Writer preserves SemVer in application identity. Debian installers map the prerelease separator to
`~` so beta → RC → stable upgrades follow
[Debian's version ordering](https://www.debian.org/doc/debian-policy/ch-controlfields.html#version).

Keep the release's accepted scope, compatibility notes, known limits and candidate run in a GitHub
release issue attached to the relevant milestone. Link unresolved blockers there rather than copying
task state into Markdown. The issue body becomes release notes. Before stable 1.0, assess
[§22–23](docs/spec/release.md#23-acceptance-criteria-for-a-serious-v1), including remaining consumer
evidence; green CI alone does not establish readiness.

1. Create a release branch such as `release/prepare-v0-2-0-beta-1` from current `main`, linked to
   that release issue. Branches follow the usual kebab-case policy; tags retain exact SemVer.
   Provision tools with `just release setup`, inspect `just release prepare VERSION`, then apply
   with `just release set-version VERSION`. This coordinated helper updates core and Writer; the FFI
   version is deliberately excluded. Review the manifest, lockfile, installer and AppStream changes
   and update `CHANGELOG.md`. Commit with `[REC-N] release: prepare VERSION` and use the usual
   protected PR workflow with `Refs #N`, keeping the release issue open until publication succeeds.
   Neither helper commits, tags, pushes nor publishes.
2. After the approved merge, resolve the complete commit ID. Dispatch **Release candidate** from
   `main`, supplying that exact `commit` and prepared `version`. It runs complete correctness,
   performance/session and distribution verification, exercises archived CLI/LSP binaries, and
   collects Writer installers. Wait for success and inspect its receipt and artifacts. Branch
   rehearsals are supported but cannot be published. Candidates expire after 30 days; rerun expired
   candidates rather than rebuilding inside publication.
3. Once release publication is authorized, create and verify one annotated signed product tag: `git
   tag -s vVERSION COMMIT -m 'Recite VERSION'`, `git verify-tag vVERSION`, then `git push origin
   refs/tags/vVERSION`. The signing key must be registered with GitHub so its tag API reports a
   verified signature. Never force-update or recycle a version tag.
4. Dispatch **Publish verified release** from `main` with the successful candidate run, same
   version/commit and release issue. Before the first publication, configure the GitHub `release`
   environment to allow only `main` and require maintainer approval. Its optional
   `CARGO_REGISTRY_TOKEN` is needed only when explicitly selecting Rust `crates` to publish. Leave
   that input blank for artifact-only previews. For first registry publication, use `registry_only`
   with dependency-ordered batches within cargo-release's default five-new-crate limit; each batch
   checks the same candidate and publishes no GitHub release. Once those dependencies are available,
   run final publication with `registry_only` false and any remaining selected crates. Publication
   checks workflow origin, tag identity and all hashes, stages a draft, then publishes those
   artifacts without rebuilding them. Beta/RC versions are GitHub prereleases. Registry publication
   verifies crate packages separately; it does not rebuild the downloadable artifacts.
5. Verify the published assets and notes, close the release issue and reconcile its milestone. If
   publication fails, inspect partial registry uploads and the unpublished draft before retrying;
   delete only an incomplete draft, never overwrite a public release or re-upload different bytes
   under an existing crate version. Stable promotion requires a stable-version preparation PR and
   fresh candidate, even when its code matches the last RC.
