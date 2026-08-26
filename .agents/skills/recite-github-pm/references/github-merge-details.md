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
standard GitHub reviews, review threads, and reported checks. The merge command
must be run from a clean worktree after `mise run verify` and the gate pass;
protected `main` remains the authority for its configured review policy,
aggregate status check, linear history, and signed commits.

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
required checks, or tests. See the [official OpenAI Codex GitHub integration
documentation](https://developers.openai.com/codex/integrations/github/) for
current setup and availability details.

The gate blocks failed or errored reported checks when any are present; if checks
have not reported yet, local checks remain mandatory.

Do not use direct pushes to `main` or bypass the protected pull-request path.
