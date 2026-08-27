#!/usr/bin/env bash
set -euo pipefail

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a Git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve Git repo root from current directory" >&2
    exit 2
  fi
fi
gate="$repo_root/.agents/skills/recite-github-pm/scripts/check-pr-review-gates.sh"
fixture_root="$repo_root/tests/check-pr-review-gates/fixtures"

run_metadata_fixture() {
  local expected_status="$1"
  local fixture="$2"
  local expected_head="${3:-}"
  local expected_base="${4:-main}"
  local output status

  set +e
  output="$($gate --check-metadata "$fixture" "$expected_head" "$expected_base" 2>&1)"
  status=$?
  set -e

  if [[ "$status" != "$expected_status" ]]; then
    echo "metadata fixture status expectation failed: ${fixture##*/} (expected $expected_status, got $status)" >&2
    echo "$output" >&2
    return 1
  fi
}

run_fixture() {
  local expected_status="$1"
  local fixture="$2"
  local output status

  set +e
  output="$("$gate" --check-rollup "$fixture" 2>&1)"
  status=$?
  set -e

  if [[ "$status" != "$expected_status" ]]; then
    echo "fixture status expectation failed: ${fixture##*/} (expected $expected_status, got $status)" >&2
    echo "$output" >&2
    return 1
  fi

  if [[ "${fixture##*/}" == "distinct-check-names.json" && "$output" != *"Lint: failure"* ]]; then
    echo "fixture check-name expectation failed: distinct failing check was not retained" >&2
    echo "$output" >&2
    return 1
  fi
}

run_fixture 0 "$fixture_root/superseded-success.json"
run_fixture 0 "$fixture_root/mixed-status-context.json"
run_fixture 1 "$fixture_root/newest-failure.json"
run_fixture 1 "$fixture_root/newest-pending.json"
run_fixture 1 "$fixture_root/ambiguous-newest-tie.json"
run_fixture 1 "$fixture_root/ambiguous-pending-neutral-tie.json"
run_fixture 1 "$fixture_root/unorderable-pending.json"
run_fixture 1 "$fixture_root/missing-required.json"
run_fixture 1 "$fixture_root/distinct-check-names.json"

run_metadata_fixture 0 "$fixture_root/metadata-valid-integration.json" integration/milestone-integration
run_metadata_fixture 0 "$fixture_root/metadata-valid-ordinary.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-title.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-body.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-integration.json" integration/milestone-integration
run_metadata_fixture 1 "$fixture_root/metadata-invalid-label.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-head.json" main
run_metadata_fixture 1 "$fixture_root/metadata-invalid-integration-label.json" integration/milestone-integration

echo "check-pr-review-gates rollup and metadata fixtures passed."
