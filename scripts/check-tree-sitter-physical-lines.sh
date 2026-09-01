#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git -C "${1:-.}" rev-parse --show-toplevel)"
grammar_dir="$repo_root/editors/recite-tree-sitter"
if [[ -z "${XDG_CACHE_HOME:-}" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter-lines.XXXXXX")"
cleanup() { rm -rf "$scratch"; }
trap cleanup EXIT

parse_clean() {
  local name="$1" expected="$2" source="$3" file="$scratch/$1.recite" output rc
  printf '%s' "$source" > "$file"
  rc=0
  if output="$(tree-sitter parse --grammar-path "$grammar_dir" "$file" 2>&1)"; then
    rc=0
  else
    rc=$?
  fi
  if (( rc > 1 )) || grep -Eq '\((ERROR|MISSING)( |\))' <<<"$output"; then
    echo "physical-line probe produced recovery for $name" >&2
    sed -n '1,100p' <<<"$output" >&2
    exit 1
  fi
  if ! grep -Fq "($expected" <<<"$output"; then
    echo "physical-line probe lost $expected for $name" >&2
    sed -n '1,100p' <<<"$output" >&2
    exit 1
  fi
}

echo "== final physical statements without LF =="
final_names=(
  comment_line block_statement line_statement choice_statement effect_statement
  divert_statement if_statement else_statement match_statement case_statement
  plural_line prose_line
)
final_sources=(
  '# final comment'
  ':: final'
  '> final'
  '? final'
  '! immediate play_sfx("relay")'
  '-> END'
  ':if true'
  ':else'
  ':match true'
  ':case open'
  '| final plural'
  '  final prose'
)
for index in "${!final_names[@]}"; do
  parse_clean "final-${final_names[$index]}" "${final_names[$index]}" "${final_sources[$index]}"
done

echo "== internal line separation =="
parse_clean internal-separation if_statement $'  first prose\n:if true'
adjacent_file="$scratch/adjacent.recite"
printf '%s' ':: first:: second' > "$adjacent_file"
adjacent_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$adjacent_file" 2>&1)" || true
if ! grep -Eq '\((ERROR|MISSING)( |\))' <<<"$adjacent_output"; then
  echo "same-line statements were accepted without a physical separator" >&2
  sed -n '1,100p' <<<"$adjacent_output" >&2
  exit 1
fi

echo "== blank lines, CRLF, and production differential =="
boundary_source=$':: physical default\n> line@0123456789abcdef0123 bind=(name:string=$name)\n   \n\t\n  [slow]Final {name} prose.[/slow]'
parse_clean blank-indentation prose_line "$boundary_source"
boundary_file="$scratch/boundary.recite"
printf '%s' "$boundary_source" > "$boundary_file"
boundary_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$boundary_file" 2>&1)"
if [[ "$(grep -Fc '(blank_line' <<<"$boundary_output")" -ne 2 ]]; then
  echo "space/tab-only blank lines were not retained as two blank_line nodes" >&2
  sed -n '1,100p' <<<"$boundary_output" >&2
  exit 1
fi
parse_clean final-blank-space blank_line '   '
parse_clean final-blank-tab blank_line $'\t'
parse_clean final-blank-mixed blank_line $' \t '
parse_clean final-blank-after-statement blank_line $':: source default\n  '
parse_clean final-blank-lf blank_line $'  \n'
parse_clean final-blank-crlf blank_line $'  \r\n'
spaced_adjacent_file="$scratch/spaced-adjacent.recite"
printf '%s' ':: first :: second' > "$spaced_adjacent_file"
spaced_adjacent_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$spaced_adjacent_file" 2>&1)" || true
if ! grep -Eq '\((ERROR|MISSING)( |\))' <<<"$spaced_adjacent_output"; then
  echo "same-line statements with horizontal spacing were accepted without a physical separator" >&2
  sed -n '1,100p' <<<"$spaced_adjacent_output" >&2
  exit 1
fi
boundary_captures="$scratch/boundary-captures.txt"
tree-sitter query --grammar-path "$grammar_dir" --captures \
  "$grammar_dir/queries/highlights.scm" "$boundary_file" > "$boundary_captures"
for expectation in \
  ' - tag, start: (4, 3), end: (4, 7), text: `slow`' \
  ' - string.special, start: (4, 8), end: (4, 14), text: `Final `' \
  ' - variable.parameter, start: (4, 15), end: (4, 19), text: `name`'; do
  if ! grep -Fq "$expectation" "$boundary_captures"; then
    echo "final-line capture expectation is missing: $expectation" >&2
    sed -n '1,120p' "$boundary_captures" >&2
    exit 1
  fi
done

crlf_source=$':: physical default\r\n> line@0123456789abcdef0123 bind=(name:string=$name)\r\n  [slow]Final {name} prose.[/slow]'
parse_clean crlf prose_line "$crlf_source"
crlf_file="$scratch/crlf.recite"
printf '%s' "$crlf_source" > "$crlf_file"

production_output="$scratch/production.txt"
if ! cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- validate "$boundary_file" > "$production_output" 2>&1; then
  echo "production parser rejected the blank-line/EOF boundary fixture" >&2
  sed -n '1,100p' "$production_output" >&2
  exit 1
fi
if ! cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- validate "$crlf_file" > "$production_output" 2>&1; then
  echo "production parser rejected the CRLF boundary fixture" >&2
  sed -n '1,100p' "$production_output" >&2
  exit 1
fi

echo "== empty and malformed EOF recovery =="
parse_clean empty source_file ''
malformed_file="$scratch/malformed.recite"
printf '%s' ':if (' > "$malformed_file"
malformed_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$malformed_file" 2>&1)" || true
if ! grep -Eq '\((ERROR|MISSING)( |\))' <<<"$malformed_output"; then
  echo "incomplete final statement did not expose bounded recovery" >&2
  sed -n '1,100p' <<<"$malformed_output" >&2
  exit 1
fi

echo "Physical-line boundary checks passed."
