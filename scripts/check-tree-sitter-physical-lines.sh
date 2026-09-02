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

echo "== indented block headers and body boundaries =="
indented_block_source=$':: outer default\n  > first@0123456789abcdef0123\n    First body.\n  :: nested speaker=guide\n  > nested@fedcba9876543210fedc\n    Nested body.\n\t:: tabbed\n\t> tabbed@00112233445566778899\n\t  Tabbed body.\n'
parse_clean indented-block block_statement "$indented_block_source"
indented_block_file="$scratch/indented-block.recite"
printf '%s' "$indented_block_source" > "$indented_block_file"
indented_block_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$indented_block_file" 2>&1)"
if [[ "$(grep -Fc '(block_statement' <<<"$indented_block_output")" -ne 3 ]]; then
  echo "indented block boundary probe did not retain all three block statements" >&2
  sed -n '1,160p' <<<"$indented_block_output" >&2
  exit 1
fi
indented_block_captures="$scratch/indented-block-captures.txt"
tree-sitter query --grammar-path "$grammar_dir" --captures \
  "$grammar_dir/queries/highlights.scm" "$indented_block_file" > "$indented_block_captures"
for expectation in \
  ' - keyword, start: (3, 2), end: (3, 4), text: `::`' \
  ' - label, start: (3, 5), end: (3, 11), text: `nested`' \
  ' - property, start: (3, 12), end: (3, 19), text: `speaker`' \
  ' - keyword, start: (6, 1), end: (6, 3), text: `::`' \
  ' - label, start: (6, 4), end: (6, 10), text: `tabbed`'; do
  if ! grep -Fq "$expectation" "$indented_block_captures"; then
    echo "indented block capture expectation is missing: $expectation" >&2
    sed -n '1,160p' "$indented_block_captures" >&2
    exit 1
  fi
done

indented_block_eof_source="${indented_block_source%$'\n'}"
parse_clean indented-block-eof block_statement "$indented_block_eof_source"
indented_block_eof_file="$scratch/indented-block-eof.recite"
printf '%s' "$indented_block_eof_source" > "$indented_block_eof_file"
parse_clean final-indented-block-eof block_statement $'  :: final_eof'
indented_block_crlf_source="${indented_block_source//$'\n'/$'\r\n'}"
parse_clean indented-block-crlf block_statement "$indented_block_crlf_source"
indented_block_crlf_file="$scratch/indented-block-crlf.recite"
printf '%s' "$indented_block_crlf_source" > "$indented_block_crlf_file"

production_output="$scratch/production.txt"
for production_fixture in "$indented_block_file" "$indented_block_eof_file" "$indented_block_crlf_file"; do
  if ! cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
    validate "$production_fixture" > "$production_output" 2>&1; then
    echo "production parser rejected an indented-block boundary fixture: $production_fixture" >&2
    sed -n '1,100p' "$production_output" >&2
    exit 1
  fi
done

malformed_block_file="$scratch/malformed-indented-block.recite"
printf '%s' $':: outer default\n> outer@0123456789abcdef0123\n  Outer body.\n  ::: malformed\n  :: recovered\n> recovered@fedcba9876543210fedc\n  Recovered body.\n' > "$malformed_block_file"
malformed_block_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$malformed_block_file" 2>&1)" || true
if ! grep -Eq '\((ERROR|MISSING)( |\))' <<<"$malformed_block_output" \
  || [[ "$(grep -Fc '(block_statement' <<<"$malformed_block_output")" -lt 3 ]]; then
  echo "malformed indented block near-miss did not expose recovery and retain the later header" >&2
  sed -n '1,160p' <<<"$malformed_block_output" >&2
  exit 1
fi

non_space_indent_file="$scratch/non-space-indented-block.recite"
printf '%s' $' :: nbsp\n\v:: vertical\n:: valid\n' > "$non_space_indent_file"
non_space_indent_output="$(tree-sitter parse --grammar-path "$grammar_dir" "$non_space_indent_file" 2>&1)" || true
if [[ "$(grep -Ec '\((ERROR|MISSING)( |\))' <<<"$non_space_indent_output")" -lt 2 ]] \
  || [[ "$(grep -Fc '(block_statement' <<<"$non_space_indent_output")" -ne 1 ]]; then
  echo "non-space indentation was accepted as a block header or hid recovery" >&2
  sed -n '1,160p' <<<"$non_space_indent_output" >&2
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

"$repo_root/scripts/check-tree-sitter-escaped-braces.sh" "$repo_root"
"$repo_root/scripts/check-tree-sitter-compact-diverts.sh" "$repo_root"
"$repo_root/scripts/check-tree-sitter-compact-markers.sh" "$repo_root"

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
