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
status, linear history, and signed commits.

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
that final PR. After the merge, refresh the roadmap on `main` against live
GitHub state.

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

When Codex cloud Code Review is enabled for the repository, request a review in
the pull request by commenting:

```
@codex review
```

Repository owners may enable automatic reviews in Codex settings instead. Codex
posts a standard GitHub review; inspect its findings against the current diff,
then resolve or explicitly reject each review thread. The local gate does not
parse review-comment payloads or rely on a bot username. Codex findings are
advisory and do not replace human maintainer approval, branch protection,
required checks, or tests. See the [official Codex GitHub review
documentation](https://learn.chatgpt.com/docs/third-party/github) for current
setup and availability details.

Treat the review as an optional asynchronous signal. Continue useful local or
disjoint work; do not make waiting for it a critical path. When a
standard review is available, inspect it against the exact current head SHA,
return actionable findings to the owning implementer, resolve or explicitly
reject each thread, and run the gate again after the correction pass. If the
head changes, treat an earlier review as stale. Fall back to the normal
human/manual review path when the optional service is unavailable; never revive
custom comment parsing.

The gate blocks failed or errored reported checks when any are present; if
checks have not reported yet, risk-appropriate local checks remain mandatory.

Do not use direct pushes to `main` or bypass the protected pull-request path.
