#!/usr/bin/env bash
set -euo pipefail

[[ "${1:-}" == "${RECITE_BASE_REF:?}" ]]
[[ "${2:-}" == "${RECITE_HEAD_REF:?}" ]]
printf 'base-lint-policy\n' > "${TRUSTED_LINT_POLICY_MARKER:?}"
