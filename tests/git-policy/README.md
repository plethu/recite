# Git workflow policy fixtures

`scripts/check-git-policy.sh` reads these deterministic fixtures on every
invocation before checking the current repository. `branches.tsv` records
expected branch-name results. Files under `commit-messages/` use the filename
prefix `valid-` or `invalid-` to record expected commit-message results,
including the issue prefix, body-sentence, and attribution-trailer rules. An
ordinary pull-request title supplies one issue code that every commit must
use. Run the integration fixture with `bash tests/git-policy/check-integration.sh`;
it proves that only a matching `workflow/integration` label and
`integration/<short-kebab-topic>` branch may contain multiple valid issue codes,
while retaining the subject and attribution rules.
