#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "${1:-.}" rev-parse --show-toplevel)"
grammar_dir="$repo_root/editors/recite-tree-sitter"
fixture="$grammar_dir/test/fixtures/compact-markers.recite"
if [[ -z "${XDG_CACHE_HOME:-}" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter-markers.XXXXXX")"
production_output="$scratch/production.txt"
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT

parse_clean() {
  local file="$1" output="$2" rc=0
  if (cd "$repo_root" && tree-sitter parse --grammar-path "$grammar_dir" "$file") > "$output" 2>&1; then
    rc=0
  else
    rc=$?
  fi
  if (( rc > 1 )) || grep -Eq '\((ERROR|MISSING)( |\))' "$output"; then
    echo "compact-marker fixture produced a recovery node: $file" >&2
    sed -n '1,180p' "$output" >&2
    exit 1
  fi
  if [[ "$(grep -Fc '(block_statement' "$output")" -ne 3 ]] \
    || [[ "$(grep -Fc '(effect_statement' "$output")" -ne 4 ]]; then
    echo "compact-marker fixture lost block/effect structure: $file" >&2
    sed -n '1,180p' "$output" >&2
    exit 1
  fi
}

source_text="$(<"$fixture")"
lf_file="$scratch/compact-markers.recite"
crlf_file="$scratch/compact-markers-crlf.recite"
eof_file="$scratch/compact-markers-eof.recite"
printf '%s\n' "$source_text" > "$lf_file"
crlf_text="${source_text//$'\n'/$'\r\n'}"
printf '%s\r\n' "$crlf_text" > "$crlf_file"
printf '%s' "$source_text" > "$eof_file"

echo "== compact/spaced/indented block and effect markers =="
for file in "$lf_file" "$crlf_file" "$eof_file"; do
  tree="$scratch/$(basename "$file").tree"
  parse_clean "$file" "$tree"
  if ! CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}" \
    cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
    validate "$file" > "$production_output" 2>&1; then
    echo "production parser rejected valid compact-marker fixture: $file" >&2
    sed -n '1,100p' "$production_output" >&2
    exit 1
  fi
done

captures="$scratch/compact-marker-captures.txt"
tree-sitter query --grammar-path "$grammar_dir" --captures \
  "$grammar_dir/queries/highlights.scm" "$lf_file" > "$captures"
for expectation in \
  ' - keyword, start: (0, 0), end: (0, 2), text: `::`' \
  ' - label, start: (0, 2), end: (0, 17), text: `compact_markers`' \
  ' - punctuation.special, start: (1, 0), end: (1, 1), text: `!`' \
  ' - keyword, start: (1, 1), end: (1, 10), text: `immediate`' \
  ' - function.call, start: (1, 11), end: (1, 19), text: `play_sfx`' \
  ' - keyword, start: (3, 2), end: (3, 4), text: `::`' \
  ' - label, start: (3, 4), end: (3, 20), text: `indented_compact`' \
  ' - punctuation.special, start: (4, 4), end: (4, 5), text: `!`' \
  ' - keyword, start: (4, 5), end: (4, 14), text: `immediate`' \
  ' - function.call, start: (4, 15), end: (4, 23), text: `play_sfx`'; do
  if ! grep -Fq "$expectation" "$captures"; then
    echo "compact-marker capture expectation is missing: $expectation" >&2
    sed -n '1,180p' "$captures" >&2
    exit 1
  fi
done

echo "== malformed separator and payload recovery =="
recovery_file="$scratch/compact-markers-recovery.recite"
printf '%s' $'::compact_recovery default\n!immediate play_sfx()\n::\n! \n!bogus play_sfx()\n! immediate\n! immediate ,\n!;immediate play_sfx()\n! immediate play_sfx()\n>following@0123456789abcdef0123\n  Recovery prose.\n' > "$recovery_file"
recovery_tree="$scratch/compact-markers-recovery.tree"
recovery_rc=0
if (cd "$repo_root" && tree-sitter parse --grammar-path "$grammar_dir" "$recovery_file") > "$recovery_tree" 2>&1; then
  recovery_rc=0
else
  recovery_rc=$?
fi
if (( recovery_rc > 1 )) \
  || [[ "$(grep -Ec '\((ERROR|MISSING)( |\))' "$recovery_tree")" -lt 5 ]] \
  || [[ "$(grep -Fc '(line_statement' "$recovery_tree")" -ne 1 ]] \
  || [[ "$(grep -Fc '(prose_line' "$recovery_tree")" -ne 1 ]]; then
  echo "compact-marker recovery did not retain malformed payloads and following statement" >&2
  sed -n '1,220p' "$recovery_tree" >&2
  exit 1
fi
if CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}" \
  cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  validate "$recovery_file" > "$production_output" 2>&1; then
  echo "production parser unexpectedly accepted malformed compact-marker recovery" >&2
  sed -n '1,100p' "$production_output" >&2
  exit 1
fi

echo "compact-marker syntax and production differential passed"
