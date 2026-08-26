# Git workflow policy fixtures

`scripts/check-git-policy.sh` reads these deterministic fixtures on every
invocation before checking the current repository. `branches.tsv` records
expected branch-name results. Files under `commit-messages/` use the filename
prefix `valid-` or `invalid-` to record expected commit-message results,
including the issue prefix, body-sentence, and attribution-trailer rules. The
script also checks that the pull-request title supplies the expected issue code
and that every commit uses that same code.
