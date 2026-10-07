# Protected Merge Checks

[CONTRIBUTING.md](../../../../CONTRIBUTING.md#change-and-review-workflow) owns the delivery
workflow. GitHub branch protection is authoritative; the local helper also checks Recite-specific
requirements that protection cannot express.

From a clean worktree, after checks appropriate to the changed surface:

```sh
.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh 34 feat/workspace-split main
gh pr merge 34 --repo plethu/recite --squash --delete-branch
```

Run the merge only when authorized. The helper reads the PR's base/head, current head SHA, standard
GitHub reviews, unresolved threads and reported checks. Refresh the gate if the head changes. Git
policy validates the PR title so the squash subject needs no separate override. Protected `main`
requires the normal PR path; do not bypass it with a direct push.

## Maintainer approval

`RECITE_MAINTAINERS` explicitly allowlists maintainers, defaulting to `plethu`. Do not infer merge
authority from every collaborator or automation account.

GitHub does not permit an author to approve their own PR. While Recite is solo-maintained, the
helper permits the allowlisted maintainer's self-review path. With another human maintainer, it
requires their standard GitHub approval for the exact current head and rejects stale approvals or
outstanding requested changes. Approval must not be inferred from comments or reactions.

## Check results

Required GitHub checks and resolved review threads remain mandatory. Optional automated review does
not replace them or maintainer approval. Request it only when authorized, assess findings and report
any uncovered surfaces; do not treat an older review as coverage of the current head.

The helper blocks failed or errored active checks and ignores only the retired Codex workflow/status
on older heads. GitHub may report `UNSTABLE` for such a head; the helper accepts it only when branch
protection, active check results and all other gates pass. If checks have not reported yet,
risk-appropriate local verification remains required.
