#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-trusted-pr-policy.sh

Checks live pull-request metadata and the untrusted commit range using the
base-owned Git workflow policy. This script is only for pull_request_target:
it never checks out or executes pull-request files.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi
if [[ $# -ne 0 ]]; then
  usage >&2
  exit 2
fi

if [[ "${GITHUB_EVENT_NAME:-}" != pull_request_target ]]; then
  echo "trusted policy requires pull_request_target" >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "unable to resolve Git repository root" >&2
  exit 2
}

event_path="${GITHUB_EVENT_PATH:-}"
if [[ -z "$event_path" || ! -f "$event_path" ]]; then
  echo "trusted policy requires the pull_request_target event payload" >&2
  exit 2
fi

repository="${GITHUB_REPOSITORY:-plethu/recite}"
if [[ ! "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
  echo "invalid GitHub repository: $repository" >&2
  exit 2
fi

pr_number="$(jq -er '.number // .pull_request.number // empty' "$event_path")" || {
  echo "event payload has no pull-request number" >&2
  exit 2
}
if [[ ! "$pr_number" =~ ^[1-9][0-9]*$ ]]; then
  echo "invalid pull-request number: $pr_number" >&2
  exit 2
fi

live_pr="$(gh api --method GET "repos/${repository}/pulls/${pr_number}")" || {
  echo "unable to read live pull-request metadata" >&2
  exit 2
}

json_value() {
  local filter="$1"
  jq -er "$filter" <<<"$live_pr"
}

live_number="$(json_value '.number')" || { echo "live pull request has no number" >&2; exit 1; }
title="$(json_value '.title')" || { echo "live pull request has no title" >&2; exit 1; }
body="$(jq -er '.body // ""' <<<"$live_pr")" || { echo "live pull request body is unreadable" >&2; exit 1; }
base_ref="$(json_value '.base.ref')" || { echo "live pull request has no base ref" >&2; exit 1; }
base_sha="$(json_value '.base.sha')" || { echo "live pull request has no base SHA" >&2; exit 1; }
head_ref="$(json_value '.head.ref')" || { echo "live pull request has no head ref" >&2; exit 1; }
head_sha="$(json_value '.head.sha')" || { echo "live pull request has no head SHA" >&2; exit 1; }
head_repo="$(json_value '.head.repo.full_name')" || { echo "live pull request has no head repository" >&2; exit 1; }
base_repo="$(json_value '.base.repo.full_name')" || { echo "live pull request has no base repository" >&2; exit 1; }
state="$(json_value '.state')" || { echo "live pull request has no state" >&2; exit 1; }

if [[ "$state" != open ]]; then
  echo "pull request is not open: $state" >&2
  exit 1
fi
if [[ "$live_number" != "$pr_number" ]]; then
  echo "live pull-request number mismatch: $live_number" >&2
  exit 1
fi
if [[ "$base_repo" != "$repository" ]]; then
  echo "pull request base repository mismatch: $base_repo" >&2
  exit 1
fi
if [[ "$base_ref" != main ]]; then
  echo "pull request must target main: $base_ref" >&2
  exit 1
fi
if [[ ! "$base_sha" =~ ^[0-9a-fA-F]{40}$ || ! "$head_sha" =~ ^[0-9a-fA-F]{40}$ ]]; then
  echo "pull request metadata contains an invalid commit SHA" >&2
  exit 1
fi
if [[ -z "$head_ref" || "$head_ref" == *$'\n'* || "$head_ref" == *$'\r'* ]]; then
  echo "pull request metadata contains an invalid head ref" >&2
  exit 1
fi
if [[ -z "$head_repo" ]]; then
  echo "pull request metadata contains no head repository" >&2
  exit 1
fi

event_head_sha="$(jq -er '.pull_request.head.sha // empty' "$event_path")" || {
  echo "event payload has no pull-request head SHA" >&2
  exit 2
}
if [[ "$event_head_sha" != "$head_sha" ]]; then
  echo "live pull-request head changed since this event; refusing stale policy run" >&2
  exit 1
fi

if ! git -C "$repo_root" cat-file -e "${base_sha}^{commit}" 2>/dev/null; then
  echo "base SHA is not present in the trusted checkout: $base_sha" >&2
  exit 2
fi

# GitHub exposes refs/pull/N/head on the repository remote even when the PR
# originates in a fork. Fetch only the ref as objects; never check it out.
git -C "$repo_root" fetch --no-tags --no-write-fetch-head origin \
  "refs/pull/${pr_number}/head:refs/recite/trusted-pr-head"
fetched_head="$(git -C "$repo_root" rev-parse --verify "refs/recite/trusted-pr-head^{commit}")"
if [[ "$fetched_head" != "$head_sha" ]]; then
  echo "fetched pull-request head does not match live metadata" >&2
  exit 1
fi

labels_integration=0
if jq -e 'any(.labels[]?; .name == "workflow/integration")' <<<"$live_pr" >/dev/null; then
  labels_integration=1
fi

# The script and fixtures below are from the checked-out base branch. The PR
# ref is supplied only as a Git object reference and is never executed.
RECITE_PR_TITLE="$title" \
RECITE_PR_BODY="$body" \
RECITE_PR_BASE_REF="$base_ref" \
RECITE_BASE_REF="$base_sha" \
RECITE_HEAD_REF=refs/recite/trusted-pr-head \
RECITE_BRANCH_NAME="$head_ref" \
RECITE_INTEGRATION_LABEL="$labels_integration" \
RECITE_INTEGRATION_PR="$labels_integration" \
GITHUB_EVENT_NAME=pull_request \
GITHUB_HEAD_REF="$head_ref" \
GITHUB_BASE_REF="$base_ref" \
  "$repo_root/scripts/check-git-policy.sh" "$repo_root"

echo "Trusted Git workflow policy passed for pull request #${pr_number}."
