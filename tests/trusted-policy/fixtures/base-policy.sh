#!/usr/bin/env bash
set -euo pipefail

[[ "${RECITE_PR_TITLE:-}" == "[REC-164] ci: add trusted pull request policy" ]]
# shellcheck source=scripts/git-policy/metadata.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/git-policy/metadata.sh"
RECITE_ISSUE_CODE='' git_policy_validate_pr_metadata 1 0
[[ "${RECITE_PR_BASE_REF:-}" == main ]]
[[ "${RECITE_BRANCH_NAME:-}" == feat/trusted-policy ]]
[[ "${RECITE_BASE_REF:-}" =~ ^[0-9a-f]{40}$ ]]
[[ "${RECITE_HEAD_REF:-}" == refs/recite/trusted-pr-head ]]
printf 'base-policy\n' >"${TRUSTED_POLICY_MARKER:?}"
