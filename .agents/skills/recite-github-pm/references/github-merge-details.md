# GitHub Review And Merge Details

Recite requires signed commits and explicit review gates. GitHub branch
protection is the source of truth for repository-level merge policy; the local
gate audits the pull request state and Recite-specific checks that protection
cannot express. Recite is currently solo-maintained, so branch protection need
not require an approving review until another human maintainer is added.

## Normal Helper Path

```bash
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh 34 issue-1-workspace-split main
gh pr merge 34 --repo plethu/recite --squash --delete-branch
```

The gate reads the pull request's base/head, review decision, current head SHA,
review comments, and reported checks. The merge command must be run from a
clean worktree after `mise run verify` and the gate pass; protected `main`
remains the authority for its configured review policy, aggregate status check,
linear history, and signed commits.

## Maintainer Review

Known maintainers come from the explicit `RECITE_MAINTAINERS` allowlist,
defaulting to `plethu`. Do not infer merge authority from every repository
collaborator: read-only contributors and automation accounts are not
maintainers.

When a second human maintainer exists, approval must be recorded as a GitHub
pull-request review:

```bash
gh pr review 34 --repo plethu/recite --approve --body "Approved for merge."
```

GitHub does not permit an author to approve their own pull request. While Recite
has one human maintainer, the helper instead requires that the PR author is the
allowlisted maintainer plus a current clean-context agent review. Once another
human maintainer is added, the helper requires their independent approval.

## Clean-Context Agent Review

A clean-context agent review is represented by a structured pull-request
comment for the current head SHA. Its GitHub author must be in the explicit
`RECITE_REVIEWERS` allowlist, which defaults to the maintainer allowlist. The
reviewing agent must start from a clean context, review the pull request
independently, and post this exact shape:

```bash
gh pr comment 34 --repo plethu/recite --body '<!-- recite-agent-review:v1 -->
Agent-Review: approved
Head-SHA: 5b1c198ce742c81b3010eec0307e9d2cbcd1af92
Context: clean
Checks:
- mise run verify'
```

If the pull-request head changes, the clean-context review is stale and must be
repeated for the new head SHA. The gate blocks failed or errored reported checks
when any are present; if checks have not reported yet, local checks remain
mandatory.

Do not use direct pushes to `main` or bypass the protected pull-request path.
