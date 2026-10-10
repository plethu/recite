# Git workflow policy fixtures

`scripts/check-git-policy.sh` exercises these fixtures before checking the current repository.
`branches.tsv` records branch-name expectations; `commit-messages/` uses `valid-` and `invalid-`
filename prefixes for commit-message cases. The checker loads its policy modules from the trusted
base checkout.

Run `bash tests/git-policy/check-integration.sh` for the integration-branch cases. Contribution and
merge policy belongs in [CONTRIBUTING.md](../../CONTRIBUTING.md); these fixtures enforce it.
