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
run_metadata_fixture 1 "$fixture_root/metadata-invalid-body-whitespace.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-integration.json" integration/milestone-integration
run_metadata_fixture 1 "$fixture_root/metadata-invalid-label.json" docs/policy-metadata
run_metadata_fixture 1 "$fixture_root/metadata-invalid-head.json" main
run_metadata_fixture 1 "$fixture_root/metadata-invalid-integration-label.json" integration/milestone-integration

run_final_recheck_fixture() {
  local fixture="$1"
  local temp_dir output status

  temp_dir="$(mktemp -d)"
  set +e
  output="$(PATH="$repo_root/tests/check-pr-review-gates:$PATH" \
    RECITE_FAKE_GH_FIXTURE="$fixture" \
    RECITE_FAKE_GH_STATE="$temp_dir/gh-view-count" \
    RECITE_GITHUB_REPO=plethu/recite \
    RECITE_MAINTAINERS=plethu \
    "$gate" 163 integration/milestone-integration main 2>&1)"
  status=$?
  set -e
  rm -f "$temp_dir/gh-view-count"
  rmdir "$temp_dir" 2>/dev/null || true

  if [[ "$status" == 0 || "$output" == *"passed Recite review gates."* ]]; then
    echo "final live recheck mutation fixture did not fail closed" >&2
    echo "$output" >&2
    return 1
  fi
}

run_final_recheck_fixture "$fixture_root/final-recheck-state-mutation.json"

echo "check-pr-review-gates rollup and metadata fixtures passed."
