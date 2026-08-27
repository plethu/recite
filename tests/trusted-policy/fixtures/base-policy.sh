#!/usr/bin/env bash
set -euo pipefail

[[ "${RECITE_PR_TITLE:-}" == "[REC-164] ci: add trusted pull request policy" ]]
[[ "${RECITE_PR_BODY:-}" == "Closes #164" ]]
[[ "${RECITE_PR_BASE_REF:-}" == main ]]
[[ "${RECITE_BRANCH_NAME:-}" == feat/trusted-policy ]]
[[ "${RECITE_BASE_REF:-}" =~ ^[0-9a-f]{40}$ ]]
[[ "${RECITE_HEAD_REF:-}" == refs/recite/trusted-pr-head ]]
printf 'base-policy\n' > "${TRUSTED_POLICY_MARKER:?}"
