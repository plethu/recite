# GitHub Review And Merge Details

Recite requires signed commits and explicit review gates. GitHub branch
protection is the source of truth for repository-level merge policy; the local
gate audits the pull request state and Recite-specific checks that protection
cannot express. Recite is currently solo-maintained, so branch protection need
not require an approving review until another human maintainer is added.

## Normal Helper Path

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh 34 feat/workspace-split main
gh pr merge 34 --repo plethu/recite --squash --delete-branch
```

The gate reads the pull request's base/head, review decision, current head SHA,
standard GitHub reviews, review threads, and reported checks. The merge command
must be run from a clean worktree after checks appropriate to the changed
surface and the gate pass: focused checks for documentation or instruction-only
changes, and `mise run verify` for broad or high-risk code changes. Required
GitHub CI and protected `main` remain authoritative for merge policy, aggregate
status, linear history, and signed commits. The Git policy validates the full
pull-request title before this step, so GitHub's squash merge inherits the
validated title without a redundant subject override.

## Milestone integration path

The coordinator creates one purpose-first `integration/<short-kebab-topic>`
branch from `main` for a milestone. Bounded implementers work in isolated
normal purpose-first branches or worktrees based on that branch. They do not
open issue-slice pull requests.
The coordinator reviews each slice, returns actionable findings to its owning
implementer, and mechanically cherry-picks accepted commits. A direct
fast-forward may use `--ff-only`; do not create default non-fast-forward merge
commits because their generated subject fails Recite's commit policy. Any
exceptional merge commit requires coordinator review and an explicit
policy-compliant `[REC-N] <type>: <subject>` message. Only mechanical conflict
resolution belongs in the coordinator's worktree.

At a stable checkpoint, open exactly one integration pull request to protected
`main`, label it `workflow/integration`, and put the milestone tracking issue
in its `[REC-N]` title. Integration mode allows multiple valid issue codes in
the commit range while still requiring every commit's normal subject and
no-attribution rules. Use this helper and the normal GitHub review path for
that final PR. After the merge, verify linked issue and milestone state on
GitHub.

## Maintainer Review

Known maintainers come from the explicit `RECITE_MAINTAINERS` allowlist,
defaulting to `plethu`. Do not infer merge authority from every repository
collaborator: read-only contributors and automation accounts are not
maintainers.

When a second human maintainer exists, approval must be recorded as a GitHub
pull-request review for the current head commit:

```bash
gh pr review 34 --repo plethu/recite --approve --body "Approved for merge."
```

GitHub does not permit an author to approve their own pull request. While Recite
has one human maintainer, the helper permits the allowlisted maintainer's
self-review path. Once another human maintainer is added, the helper requires
their independent approval and rejects stale approvals or outstanding requested
changes. Approval must be a standard GitHub pull-request review for the exact
current head SHA.

## Codex Code Review

Request Codex Code Review in the pull request from a GitHub account connected
to Codex by commenting:

```
@codex review
```

Do not post that comment from GitHub Actions: its bot account is not connected
to Codex. The base-owned `codex-review-gate.yml` observes review completion and
unresolved findings, then writes `codex-review-ready` on the PR head. `main`
requires that status in addition to the existing CI and trusted-policy checks;
`.github/required-status-checks.json` records the expected status contexts.
After the workflow is installed on `main`, an administrator can apply only the
status-check portion of protection with:

```bash
gh api -X PATCH repos/plethu/recite/branches/main/protection/required_status_checks \
  --input .github/required-status-checks.json
```

Codex findings inform review but do not replace human maintainer approval or
tests. The local gate does not parse Codex comments or rely on a bot username.
See the [official Codex GitHub review documentation](https://learn.chatgpt.com/docs/third-party/github)
for setup and availability details.

Request review when the PR is ready for that pass, then continue useful disjoint
work while it runs. Inspect findings against the current diff, return actionable
findings to the owning implementer, and resolve each thread after the correction
pass. The upstream readiness action can accept an older clean reaction after an
eight-minute grace period when no unresolved findings remain; a green status is
not proof that Codex reviewed every commit. Re-request review when a material
change needs fresh assessment. If the status does not update after thread
resolution, run `gh workflow run codex-review-gate.yml --repo plethu/recite` to
reconcile open PRs. An unavailable review service leaves the required status
unmet; do not bypass it with custom comment parsing.

The gate blocks failed or errored reported checks when any are present; if
checks have not reported yet, risk-appropriate local checks remain mandatory.

Do not use direct pushes to `main` or bypass the protected pull-request path.
