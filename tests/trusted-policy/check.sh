#!/usr/bin/env bash
set -euo pipefail

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

workflow="$repo_root/.github/workflows/trusted-policy.yml"
wrapper="$repo_root/scripts/check-trusted-pr-policy.sh"
fixture="$repo_root/tests/trusted-policy/fixtures/base-policy.sh"
for required_file in "$workflow" "$wrapper" "$fixture"; do
  [[ -f "$required_file" ]] || { echo "missing trusted-policy fixture file: $required_file" >&2; exit 1; }
done

fail_static() {
  echo "trusted-policy static assertion failed: $1" >&2
  exit 1
}

grep -Fq 'pull_request_target:' "$workflow" || fail_static 'pull_request_target trigger'
grep -Fq 'name: Trusted Git workflow policy' "$workflow" || fail_static 'unique check name'
grep -Fq 'contents: read' "$workflow" || fail_static 'read-only contents permission'
grep -Fq 'pull-requests: read' "$workflow" || fail_static 'read-only pull-request permission'
grep -Fq 'ref: main' "$workflow" || fail_static 'base checkout ref'
grep -Fq "refs/pull/\${pr_number}/head" "$wrapper" || fail_static 'PR object fetch'
grep -Fq 'refs/recite/trusted-pr-head' "$wrapper" || fail_static 'non-checkout PR ref'
grep -Fq -- '--filter=blob:none' "$wrapper" || fail_static 'blob-filtered PR object fetch'
grep -Fq 'check-git-policy.sh' "$wrapper" || fail_static 'base policy delegation'
if grep -Eq '^  pull_request:$|secrets\.|permissions:.*write|gh[[:space:]]+pr[[:space:]]+checkout|git[[:space:]]+checkout|github\.event\.pull_request\.head\.sha' "$workflow" "$wrapper"; then
  fail_static 'untrusted checkout, secret, write permission, or event-head execution'
fi
if ! grep -Eq 'actions/checkout@[0-9a-f]{40}' "$workflow"; then
  fail_static 'unpinned checkout action'
fi

test_root="$(mktemp -d)"
trap 'rm -rf -- "$test_root"' EXIT
origin="$test_root/origin.git"
repo="$test_root/repo"
fake_bin="$test_root/bin"
mkdir -p "$fake_bin"
git init --bare --quiet "$origin"
git clone --quiet "$origin" "$repo"
mkdir -p "$repo/scripts"
cp -- "$fixture" "$repo/scripts/check-git-policy.sh"
cp -- "$wrapper" "$repo/scripts/check-trusted-pr-policy.sh"
chmod +x "$repo/scripts/check-git-policy.sh"
chmod +x "$repo/scripts/check-trusted-pr-policy.sh"
git -C "$repo" switch --quiet -c main
git -C "$repo" config user.name 'Trusted policy fixture'
git -C "$repo" config user.email 'trusted-policy-fixture@example.invalid'
git -C "$repo" config commit.gpgsign false
git -C "$repo" add scripts/check-git-policy.sh scripts/check-trusted-pr-policy.sh
git -C "$repo" commit --quiet -m '[REC-164] ci: fixture base policy'
git -C "$repo" push --quiet origin HEAD:refs/heads/main
base_sha="$(git -C "$repo" rev-parse HEAD)"

git -C "$repo" switch --quiet -c pr
untrusted_policy_line="printf 'untrusted-policy\\n' > \"\${UNTRUSTED_POLICY_MARKER:?}\""
printf '%s\n' '# untrusted policy replacement must never execute' "$untrusted_policy_line" > "$repo/scripts/check-git-policy.sh"
git -C "$repo" add scripts/check-git-policy.sh
git -C "$repo" commit --quiet -m '[REC-164] ci: malicious policy fixture'
head_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" push --quiet origin HEAD:refs/pull/164/head
git -C "$repo" switch --quiet --detach main

cat > "$fake_bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
fixture="${GH_FIXTURE_JSON:?}"
if [[ -n "${GH_CALL_COUNT_FILE:-}" ]]; then
  call_count=0
  if [[ -f "$GH_CALL_COUNT_FILE" ]]; then
    call_count="$(<"$GH_CALL_COUNT_FILE")"
  fi
  call_count=$((call_count + 1))
  printf '%s\n' "$call_count" > "$GH_CALL_COUNT_FILE"
  if [[ "$call_count" -gt 1 && -n "${GH_FINAL_FIXTURE_JSON:-}" ]]; then
    fixture="$GH_FINAL_FIXTURE_JSON"
  fi
fi
cat "$fixture"
EOF
chmod +x "$fake_bin/gh"
cat > "$test_root/event.json" <<EOF
{"number":164,"pull_request":{"head":{"sha":"$head_sha"}}}
EOF
cat > "$test_root/live.json" <<EOF
{"number":164,"state":"open","title":"[REC-164] ci: add trusted pull request policy","body":"Closes #164","base":{"ref":"main","sha":"$base_sha","repo":{"full_name":"plethu/recite"}},"head":{"ref":"feat/trusted-policy","sha":"$head_sha","repo":{"full_name":"example/contributor"}},"labels":[]}
EOF

marker="$test_root/base-policy.marker"
untrusted_marker="$test_root/untrusted-policy.marker"
if ! PATH="$fake_bin:$PATH" \
  GH_FIXTURE_JSON="$test_root/live.json" \
  GITHUB_EVENT_NAME=pull_request_target \
  GITHUB_EVENT_PATH="$test_root/event.json" \
  GITHUB_REPOSITORY=plethu/recite \
  TRUSTED_POLICY_MARKER="$marker" \
  UNTRUSTED_POLICY_MARKER="$untrusted_marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null; then
  echo 'valid trusted-policy fixture was rejected' >&2
  exit 1
fi
[[ "$(<"$marker")" == base-policy ]] || { echo 'base policy did not execute' >&2; exit 1; }
[[ ! -e "$untrusted_marker" ]] || { echo 'untrusted policy executed' >&2; exit 1; }

jq '.title = "[REC-164] ci: metadata changed after validation"' "$test_root/live.json" > "$test_root/raced-live.json"
rm -f "$test_root/gh-call-count"
if PATH="$fake_bin:$PATH" GH_FIXTURE_JSON="$test_root/live.json" \
  GH_FINAL_FIXTURE_JSON="$test_root/raced-live.json" GH_CALL_COUNT_FILE="$test_root/gh-call-count" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/race.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null 2>&1; then
  echo 'metadata changed on final read was accepted' >&2
  exit 1
fi

jq '.base.ref = "release"' "$test_root/live.json" > "$test_root/invalid-live.json"
if PATH="$fake_bin:$PATH" GH_FIXTURE_JSON="$test_root/invalid-live.json" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/invalid.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null 2>&1; then
  echo 'invalid base branch was accepted' >&2
  exit 1
fi

jq '.pull_request.head.sha = "0000000000000000000000000000000000000000"' "$test_root/event.json" > "$test_root/stale-event.json"
if PATH="$fake_bin:$PATH" GH_FIXTURE_JSON="$test_root/live.json" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/stale-event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/stale.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null 2>&1; then
  echo 'stale event head was accepted' >&2
  exit 1
fi

echo 'trusted pull-request policy fixtures passed'
