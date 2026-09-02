#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "${1:-.}" rev-parse --show-toplevel)"
grammar_dir="$repo_root/editors/recite-tree-sitter"
fixture="$grammar_dir/test/fixtures/compact-diverts.recite"
if [[ -z "${XDG_CACHE_HOME:-}" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter-compact.XXXXXX")"
production_output="$scratch/production.txt"
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT

source_text="$(<"$fixture")"
lf_file="$scratch/compact-diverts.recite"
crlf_file="$scratch/compact-diverts-crlf.recite"
eof_file="$scratch/compact-diverts-eof.recite"
printf '%s\n' "$source_text" > "$lf_file"
crlf_text="${source_text//$'\n'/$'\r\n'}"
printf '%s\r\n' "$crlf_text" > "$crlf_file"
printf '%s' "$source_text" > "$eof_file"

echo "== compact diverts and production differential =="
for file in "$lf_file" "$crlf_file" "$eof_file"; do
  tree="$scratch/$(basename "$file").tree"
  rc=0
  if (cd "$repo_root" && tree-sitter parse --grammar-path "$grammar_dir" "$file") > "$tree" 2>&1; then
    rc=0
  else
    rc=$?
  fi
  if (( rc > 1 )); then
    echo "compact-divert fixture failed to parse: $file" >&2
    sed -n '1,160p' "$tree" >&2
    exit 1
  fi
  if grep -Eq '\((ERROR|MISSING)( |\))' "$tree" \
    || [[ "$(grep -Fc '(divert_statement' "$tree")" -ne 4 ]] \
    || [[ "$(grep -Fc '(target' "$tree")" -ne 4 ]] \
    || [[ "$(grep -Fc '(end_target' "$tree")" -ne 3 ]]; then
    echo "compact-divert fixture lost structure: $file" >&2
    sed -n '1,160p' "$tree" >&2
    exit 1
  fi
  if ! cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
    validate "$file" > "$production_output" 2>&1; then
    echo "production parser rejected valid compact-divert fixture: $file" >&2
    sed -n '1,100p' "$production_output" >&2
    exit 1
  fi
done

captures="$scratch/compact-divert-captures.txt"
tree-sitter query --grammar-path "$grammar_dir" --captures \
  "$grammar_dir/queries/highlights.scm" "$lf_file" > "$captures"
for expectation in \
  ' - punctuation.special, start: (3, 0), end: (3, 2), text: `->`' \
  ' - constant.builtin, start: (3, 2), end: (3, 5), text: `END`' \
  ' - punctuation.special, start: (4, 0), end: (4, 2), text: `->`' \
  ' - variable, start: (4, 2), end: (4, 16), text: `compact_target`' \
  ' - punctuation.special, start: (7, 2), end: (7, 4), text: `->`'; do
  if ! grep -Fq "$expectation" "$captures"; then
    echo "compact-divert capture expectation is missing: $expectation" >&2
    sed -n '1,160p' "$captures" >&2
    exit 1
  fi
done

recovery_file="$scratch/compact-divert-recovery.recite"
printf '%s' $':: compact_recovery default\n> first@0123456789abcdef0123\n  First.\n->target,\n> following@fedcba98765432100123\n  Following.\n->\n' > "$recovery_file"
recovery_tree="$scratch/compact-divert-recovery.tree"
recovery_rc=0
if (cd "$repo_root" && tree-sitter parse --grammar-path "$grammar_dir" "$recovery_file") > "$recovery_tree" 2>&1; then
  recovery_rc=0
else
  recovery_rc=$?
fi
if (( recovery_rc > 1 )) \
  || ! grep -Eq '\((ERROR|MISSING)( |\))' "$recovery_tree" \
  || [[ "$(grep -Fc '(line_statement' "$recovery_tree")" -ne 2 ]] \
  || [[ "$(grep -Fc '(divert_statement' "$recovery_tree")" -ne 1 ]]; then
  echo "compact-divert recovery did not retain the following statement" >&2
  sed -n '1,160p' "$recovery_tree" >&2
  exit 1
fi
if cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  validate "$recovery_file" > "$production_output" 2>&1; then
  echo "production parser unexpectedly accepted malformed compact-divert recovery" >&2
  sed -n '1,100p' "$production_output" >&2
  exit 1
fi

echo "compact-divert syntax and production differential passed"
