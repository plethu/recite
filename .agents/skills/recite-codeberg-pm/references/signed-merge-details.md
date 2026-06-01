# Signed Merge Details

Recite requires signed commits and explicit review gates. Codeberg branch protection is the source of truth for repository-level merge policy; local helpers audit that policy and handle Recite-specific gates that Codeberg cannot express.

## Normal Helper Path

```bash
.agents/skills/recite-codeberg-pm/scripts/check-pr-review-gates.sh 34 issue-1-workspace-split main
.agents/skills/recite-codeberg-pm/scripts/merge-pr-signed.sh 34 issue-1-workspace-split main
```

The merge helper refuses to run with a dirty worktree, reads PR base/head/head SHA from the Codeberg API, verifies review gates, verifies PR commit signatures, stages a no-ff merge, runs the Recite local checks, creates a signed merge commit, pushes `main`, marks the PR as `manually-merged`, and performs a targeted PR read.

If checks fail after the merge is staged, inspect the failure and run:

```bash
git merge --abort
```

## Manual Merge Recording

After pushing `main`, the signed merge helper verifies that the PR head SHA is contained in `origin/main`, then calls:

```bash
tea api -X POST repos/{owner}/{repo}/pulls/34/merge \
  -f Do=manually-merged \
  -f MergeCommitID=<signed-local-merge-sha> \
  -f MergeMessageField="Manually merged by signed local merge commit <sha>." \
  -f head_commit_id=<pr-head-sha>
```

If Codeberg reports that `manually-merged` is not an allowed merge style, the helper enables repository manual merge support with `allow_manual_merge=true` and `autodetect_manual_merge=true`, then retries.

## Maintainer Approval

Known maintainers are derived from Codeberg repository metadata where possible: repository owner plus repository collaborators. The helper also reads `RECITE_MAINTAINERS` as a comma-separated fallback/additional list, defaulting to `plethu`.

Maintainer approval must use Codeberg PR approval and must go through the courtesy wrapper because it mutates remote PR state:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea pulls approve 34 "Approved for signed local merge."
```

The same maintainer set is used to decide whether a PR author may satisfy the maintainer review gate by self-reviewing, but self-review is accepted only when the trusted maintainer set has exactly one member.

## Clean-Context Agent Review

A clean-context agent review is represented by a structured PR comment for the current PR head SHA. The reviewing agent must start from a clean context, review the PR independently, and post this exact shape through the courtesy wrapper:

```bash
.agents/skills/recite-codeberg-pm/scripts/tea-rate-limit.sh issue -- \
  tea comment 34 '<!-- recite-agent-review:v1 -->
Agent-Review: approved
Head-SHA: 5b1c198ce742c81b3010eec0307e9d2cbcd1af92
Context: clean
Checks:
- scripts/check-project-gates.sh'
```

If the PR head changes, the clean-context agent review is stale and must be repeated for the new head SHA. The gate also blocks failed or errored Codeberg commit statuses when any are reported; if no statuses exist yet, local checks remain mandatory.

Do not use these commands for Recite merges:

```bash
tea pulls merge ...
# or the Codeberg web "Merge" button
```
