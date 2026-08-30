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
lint_fixture="$repo_root/tests/trusted-policy/fixtures/base-lint-suppression-policy.sh"
lint_checker="$repo_root/scripts/check-lint-suppressions.py"
lint_ast="$repo_root/scripts/lint_suppression_ast.py"
lint_meta="$repo_root/scripts/lint_suppression_meta.py"
lint_allowlist="$repo_root/scripts/generated-rust-allowlist.txt"
for required_file in "$workflow" "$wrapper" "$fixture" "$lint_fixture" "$lint_checker" "$lint_ast" "$lint_meta" "$lint_allowlist"; do
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
grep -Fq 'jdx/mise-action@3c2e0cf82a5b2e5249f0d3635a4d83d0ae861518' "$workflow" || fail_static 'pinned base toolchain action'
grep -Fq 'MISE_ENV: maintainability' "$workflow" || fail_static 'maintainability tool environment'
grep -Fq 'mise current rust' "$workflow" || fail_static 'pinned Rust toolchain check'
grep -Fq '1.96.0' "$workflow" || fail_static 'pinned Rust toolchain version'
grep -Fq 'command -v rustfmt' "$workflow" || fail_static 'rustfmt component check'
grep -Fq "refs/pull/\${pr_number}/head" "$wrapper" || fail_static 'PR object fetch'
grep -Fq 'refs/recite/trusted-pr-head' "$wrapper" || fail_static 'non-checkout PR ref'
grep -Fq -- '--filter=blob:none' "$wrapper" || fail_static 'blob-filtered PR object fetch'
grep -Fq 'check-git-policy.sh' "$wrapper" || fail_static 'base policy delegation'
grep -Fq 'check-lint-suppressions.sh' "$wrapper" || fail_static 'base lint suppression delegation'
grep -Fq -- '--policy-revision' "$wrapper" || fail_static 'base-owned lint policy revision'
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
ast_grep_bin="$(command -v ast-grep || true)"
[[ -n "$ast_grep_bin" ]] || {
  echo 'trusted-policy fixture requires the pinned ast-grep tool' >&2
  exit 2
}
rustfmt_bin="$(command -v rustfmt || true)"
rustc_bin="$(command -v rustc || true)"
[[ -n "$rustfmt_bin" && -n "$rustc_bin" ]] || {
  echo 'trusted-policy fixture requires the pinned Rustfmt toolchain' >&2
  exit 2
}
[[ "$($rustc_bin --version)" == rustc\ 1.96.0\ \(* ]] || {
  echo "trusted-policy fixture found an unpinned Rust toolchain: $($rustc_bin --version)" >&2
  exit 2
}
[[ "$($ast_grep_bin --version)" == 'ast-grep 0.44.1' ]] || {
  echo "trusted-policy fixture found an unpinned ast-grep: $($ast_grep_bin --version)" >&2
  exit 2
}
# Exercise the policy with only the fake GitHub CLI, the pinned parser, and
# standard system tools. This prevents an unrelated user PATH from masking a
# missing trusted-policy dependency.
clean_path="$fake_bin:$(dirname "$ast_grep_bin"):$(dirname "$rustfmt_bin"):$(dirname "$rustc_bin"):/usr/bin:/bin"
parse_probe="$test_root/rustfmt-parse-only.rs"
printf '%s\n' 'fn parse_only( ){let _=1;}' > "$parse_probe"
before_parse_probe="$(sha256sum "$parse_probe")"
"$rustfmt_bin" --edition 2024 --config skip_children=true --emit stdout \
  "$parse_probe" >/dev/null
[[ "$before_parse_probe" == "$(sha256sum "$parse_probe")" ]] || {
  echo 'rustfmt parse-only probe rewrote its input' >&2
  exit 2
}
git init --bare --quiet "$origin"
git clone --quiet "$origin" "$repo"
mkdir -p "$repo/scripts" "$repo/crates/demo/src"
cp -- "$fixture" "$repo/scripts/check-git-policy.sh"
cp -- "$lint_fixture" "$repo/scripts/check-lint-suppressions.sh"
cp -- "$lint_checker" "$repo/scripts/check-lint-suppressions.py"
cp -- "$lint_ast" "$repo/scripts/lint_suppression_ast.py"
cp -- "$lint_meta" "$repo/scripts/lint_suppression_meta.py"
cp -- "$lint_allowlist" "$repo/scripts/generated-rust-allowlist.txt"
cp -- "$wrapper" "$repo/scripts/check-trusted-pr-policy.sh"
chmod +x "$repo/scripts/check-git-policy.sh"
chmod +x "$repo/scripts/check-lint-suppressions.sh"
chmod +x "$repo/scripts/check-trusted-pr-policy.sh"
git -C "$repo" switch --quiet -c main
git -C "$repo" config user.name 'Trusted policy fixture'
git -C "$repo" config user.email 'trusted-policy-fixture@example.invalid'
git -C "$repo" config commit.gpgsign false
git -C "$repo" add scripts/check-git-policy.sh scripts/check-lint-suppressions.sh \
  scripts/check-lint-suppressions.py scripts/lint_suppression_ast.py \
  scripts/lint_suppression_meta.py \
  scripts/generated-rust-allowlist.txt \
  scripts/check-trusted-pr-policy.sh
git -C "$repo" commit --quiet -m '[REC-164] ci: fixture base policy'
git -C "$repo" push --quiet origin HEAD:refs/heads/main
base_sha="$(git -C "$repo" rev-parse HEAD)"

git -C "$repo" switch --quiet -c pr
untrusted_policy_line="printf 'untrusted-policy\\n' > \"\${UNTRUSTED_POLICY_MARKER:?}\""
printf '%s\n' '# untrusted policy replacement must never execute' "$untrusted_policy_line" > "$repo/scripts/check-git-policy.sh"
untrusted_lint_line="printf 'untrusted-lint-policy\\n' > \"\${UNTRUSTED_LINT_POLICY_MARKER:?}\""
printf '%s\n' '# untrusted lint policy replacement must never execute' "$untrusted_lint_line" > "$repo/scripts/check-lint-suppressions.sh"
git -C "$repo" add scripts/check-git-policy.sh
git -C "$repo" add scripts/check-lint-suppressions.sh
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
lint_marker="$test_root/base-lint-policy.marker"
untrusted_lint_marker="$test_root/untrusted-lint-policy.marker"
if ! PATH="$clean_path" \
  GH_FIXTURE_JSON="$test_root/live.json" \
  GITHUB_EVENT_NAME=pull_request_target \
  GITHUB_EVENT_PATH="$test_root/event.json" \
  GITHUB_REPOSITORY=plethu/recite \
  TRUSTED_POLICY_MARKER="$marker" \
  UNTRUSTED_POLICY_MARKER="$untrusted_marker" \
  TRUSTED_LINT_POLICY_MARKER="$lint_marker" \
  UNTRUSTED_LINT_POLICY_MARKER="$untrusted_lint_marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null; then
  echo 'valid trusted-policy fixture was rejected' >&2
  exit 1
fi
[[ "$(<"$marker")" == base-policy ]] || { echo 'base policy did not execute' >&2; exit 1; }
[[ ! -e "$untrusted_marker" ]] || { echo 'untrusted policy executed' >&2; exit 1; }
[[ "$(<"$lint_marker")" == base-lint-policy ]] || { echo 'base lint policy did not execute' >&2; exit 1; }
[[ ! -e "$untrusted_lint_marker" ]] || { echo 'untrusted lint policy executed' >&2; exit 1; }

# A pull request cannot grant a new exemption by changing the generated
# allowlist in the same change. The trusted base checker must read its policy
# files at the base revision while inspecting the fetched PR tree.
git -C "$repo" switch --quiet pr
cat > "$repo/crates/demo/src/generated.rs" <<'EOF'
#[allow(dead_code)]
fn fake_generated_from_pr() {}
EOF
printf '%s\n' 'crates/demo/src/generated.rs' > "$repo/scripts/generated-rust-allowlist.txt"
git -C "$repo" add crates/demo/src/generated.rs scripts/generated-rust-allowlist.txt
git -C "$repo" commit --quiet -m '[REC-185] fixture: tamper with generated allowlist'
tampered_head_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" push --quiet --force origin HEAD:refs/pull/164/head
git -C "$repo" switch --quiet --detach main
git -C "$repo" update-ref -d refs/recite/trusted-pr-head
jq --arg sha "$tampered_head_sha" '.pull_request.head.sha = $sha' "$test_root/event.json" > "$test_root/tampered-event.json"
jq --arg sha "$tampered_head_sha" '.head.sha = $sha' "$test_root/live.json" > "$test_root/tampered-live.json"
if PATH="$clean_path" GH_FIXTURE_JSON="$test_root/tampered-live.json" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/tampered-event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/tampered.marker" \
  TRUSTED_LINT_POLICY_MARKER="$test_root/tampered-lint.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >"$test_root/tampered-output" 2>&1; then
  echo 'PR-modified generated allowlist granted an exemption' >&2
  cat "$test_root/tampered-output" >&2
  exit 1
fi
grep -Fq 'non-empty literal reason' "$test_root/tampered-output" || {
  echo 'tampered allowlist rejection missed the production reason diagnostic' >&2
  cat "$test_root/tampered-output" >&2
  exit 1
}

# Use a fresh valid PR head for the race fixture. The initial event, live API
# response, and fetched ref are therefore consistent; only the final API read
# changes, proving the post-validation re-read is what rejects the run.
git -C "$repo" switch --quiet pr
git -C "$repo" rm -q crates/demo/src/generated.rs
git -C "$repo" show "$base_sha:scripts/generated-rust-allowlist.txt" > \
  "$repo/scripts/generated-rust-allowlist.txt"
git -C "$repo" add scripts/generated-rust-allowlist.txt
git -C "$repo" commit --quiet -m '[REC-185] fixture: prepare metadata race'
race_head_sha="$(git -C "$repo" rev-parse HEAD)"
git -C "$repo" push --quiet --force origin HEAD:refs/pull/164/head
git -C "$repo" switch --quiet --detach main
git -C "$repo" update-ref -d refs/recite/trusted-pr-head
jq --arg sha "$race_head_sha" '.pull_request.head.sha = $sha' "$test_root/event.json" > "$test_root/race-event.json"
jq --arg sha "$race_head_sha" '.head.sha = $sha' "$test_root/live.json" > "$test_root/race-live.json"
jq '.title = "[REC-164] ci: metadata changed after validation"' "$test_root/race-live.json" > "$test_root/raced-live.json"
rm -f "$test_root/gh-call-count"
if PATH="$clean_path" GH_FIXTURE_JSON="$test_root/race-live.json" \
  GH_FINAL_FIXTURE_JSON="$test_root/raced-live.json" GH_CALL_COUNT_FILE="$test_root/gh-call-count" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/race-event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/race.marker" \
  TRUSTED_LINT_POLICY_MARKER="$test_root/race-lint.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >"$test_root/race-output" 2>&1; then
  echo 'metadata changed on final read was accepted' >&2
  cat "$test_root/race-output" >&2
  exit 1
fi
grep -Fq 'policy metadata changed during validation' "$test_root/race-output" || {
  echo 'metadata race missed the final reread diagnostic' >&2
  cat "$test_root/race-output" >&2
  exit 1
}

jq '.base.ref = "release"' "$test_root/live.json" > "$test_root/invalid-live.json"
if PATH="$clean_path" GH_FIXTURE_JSON="$test_root/invalid-live.json" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/invalid.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null 2>&1; then
  echo 'invalid base branch was accepted' >&2
  exit 1
fi

jq '.pull_request.head.sha = "0000000000000000000000000000000000000000"' "$test_root/event.json" > "$test_root/stale-event.json"
if PATH="$clean_path" GH_FIXTURE_JSON="$test_root/live.json" \
  GITHUB_EVENT_NAME=pull_request_target GITHUB_EVENT_PATH="$test_root/stale-event.json" \
  GITHUB_REPOSITORY=plethu/recite TRUSTED_POLICY_MARKER="$test_root/stale.marker" \
  bash -c 'cd "$1" && ./scripts/check-trusted-pr-policy.sh' trusted-policy "$repo" >/dev/null 2>&1; then
  echo 'stale event head was accepted' >&2
  exit 1
fi

echo 'trusted pull-request policy fixtures passed'
