#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "${1:-.}" rev-parse --show-toplevel)"
grammar_dir="$repo_root/editors/recite-tree-sitter"
if [[ -z "${XDG_CACHE_HOME:-}" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter-escaped.XXXXXX")"
production_output="$scratch/production.txt"
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT

escaped_fixture="$grammar_dir/test/fixtures/escaped-braces.recite"
escaped_source="$(<"$escaped_fixture")"
escaped_lf_file="$scratch/escaped-braces.recite"
escaped_crlf_file="$scratch/escaped-braces-crlf.recite"
escaped_eof_file="$scratch/escaped-braces-eof.recite"
printf '%s\n' "$escaped_source" > "$escaped_lf_file"
escaped_crlf_source="${escaped_source//$'\n'/$'\r\n'}"
printf '%s\r\n' "$escaped_crlf_source" > "$escaped_crlf_file"
printf '%s' "$escaped_source" > "$escaped_eof_file"

echo "== escaped braces and production differential =="
for escaped_file in "$escaped_lf_file" "$escaped_crlf_file" "$escaped_eof_file"; do
  escaped_output="$scratch/$(basename "$escaped_file").tree"
  escaped_rc=0
  if (
    cd "$repo_root"
    tree-sitter parse --grammar-path "$grammar_dir" "$escaped_file"
  ) > "$escaped_output" 2>&1; then
    escaped_rc=0
  else
    escaped_rc=$?
  fi
  if (( escaped_rc > 1 )) || grep -Eq '\((ERROR|MISSING)( |\))' "$escaped_output"; then
    echo "escaped-brace fixture produced a recovery node: $escaped_file" >&2
    sed -n '1,160p' "$escaped_output" >&2
    exit 1
  fi
  if [[ "$(grep -Fc '(escaped_brace' "$escaped_output")" -lt 8 ]] \
    || [[ "$(grep -Fc '(interpolation' "$escaped_output")" -ne 2 ]] \
    || [[ "$(grep -Fc '(line_statement' "$escaped_output")" -ne 1 ]] \
    || [[ "$(grep -Fc '(choice_statement' "$escaped_output")" -ne 1 ]]; then
    echo "escaped-brace fixture lost prose, interpolation, line, or choice structure: $escaped_file" >&2
    sed -n '1,160p' "$escaped_output" >&2
    exit 1
  fi
  if ! cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
    validate "$escaped_file" > "$production_output" 2>&1; then
    echo "production parser rejected escaped-brace fixture: $escaped_file" >&2
    sed -n '1,100p' "$production_output" >&2
    exit 1
  fi
done

escaped_recovery_file="$scratch/escaped-braces-recovery.recite"
printf '%s' $':: escaped_recovery default\n> first@0123456789abcdef0123\n  Two slashes \\\\{name}\n> next@fedcba98765432100123\n  Recovery keeps the following statement.\n-> END\n' > "$escaped_recovery_file"
escaped_recovery_output="$scratch/escaped-braces-recovery.tree"
escaped_recovery_rc=0
if (
  cd "$repo_root"
  tree-sitter parse --grammar-path "$grammar_dir" "$escaped_recovery_file"
) > "$escaped_recovery_output" 2>&1; then
  escaped_recovery_rc=0
else
  escaped_recovery_rc=$?
fi
if (( escaped_recovery_rc > 1 )) \
  || ! grep -Eq '\((ERROR|MISSING)( |\))' "$escaped_recovery_output" \
  || [[ "$(grep -Fc '(line_statement' "$escaped_recovery_output")" -ne 2 ]] \
  || ! grep -Fq '(divert_statement' "$escaped_recovery_output"; then
  echo "double-backslash escaped-brace recovery did not retain the following statements" >&2
  sed -n '1,160p' "$escaped_recovery_output" >&2
  exit 1
fi
escaped_recovery_production_rc=0
if cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  validate "$escaped_recovery_file" > "$production_output" 2>&1; then
  escaped_recovery_production_rc=0
else
  escaped_recovery_production_rc=$?
fi
if (( escaped_recovery_production_rc == 0 )); then
  echo "production parser unexpectedly accepted the unescaped closing brace" >&2
  sed -n '1,100p' "$production_output" >&2
  exit 1
fi
echo "escaped-brace syntax and production differential passed"
