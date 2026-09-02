#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-tree-sitter.sh [repo-root]

Checks the syntax-only Recite Tree-sitter grammar. The check verifies that the
checked-in generated parser is reproducible, the corpus passes, the canonical
corpus source remains linked to the shared Recite fixture, required highlight
captures are exact for representative syntax, all canonical Recite fixtures
parse through the same grammar, malformed and draft IDs remain recoverable and
highlighted, the compiler still rejects those IDs, and incomplete input
produces a recoverable tree.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if (( $# > 1 )); then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)" || {
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  }
else
  repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "unable to resolve Git repository root" >&2
    exit 2
  }
fi
# Tree-sitter and its parser-generation dependencies use XDG cache locations.
# Keep direct invocations writable and scoped to this checkout; mise tasks
# provide the same value explicitly for nested verification commands.
if [[ -z "${XDG_CACHE_HOME:-}" ]]; then
  export XDG_CACHE_HOME="$repo_root/target/tree-sitter-cache"
fi
grammar_dir="$repo_root/editors/recite-tree-sitter"
parser_abi=14
canonical_fixture="$repo_root/fixtures/recite/valid/language_pressure.recite"
canonical_corpus="$grammar_dir/test/corpus/canonical.txt"
recovery_corpus="$grammar_dir/test/corpus/recovery.txt"
capture_fixture="$grammar_dir/test/fixtures/capture-values.recite"
id_recovery_fixture="$grammar_dir/test/fixtures/id-recovery.recite"
incomplete_fixture="$grammar_dir/test/fixtures/incomplete.recite"

for required_file in \
  "$grammar_dir/grammar.js" \
  "$grammar_dir/tree-sitter.json" \
  "$grammar_dir/queries/highlights.scm" \
  "$grammar_dir/src/grammar.json" \
  "$grammar_dir/src/node-types.json" \
  "$grammar_dir/src/parser.c" \
  "$canonical_fixture" \
  "$canonical_corpus" \
  "$recovery_corpus" \
  "$capture_fixture" \
  "$id_recovery_fixture" \
  "$incomplete_fixture" "$grammar_dir/test/fixtures/compact-markers.recite"; do
  if [[ ! -f "$required_file" ]]; then
    echo "missing required Tree-sitter grammar file: $required_file" >&2
    exit 2
  fi
done

if ! grep -Eq "^#define LANGUAGE_VERSION $parser_abi$" "$grammar_dir/src/parser.c"; then
  echo "generated Tree-sitter parser does not target ABI $parser_abi" >&2
  exit 1
fi

if ! command -v tree-sitter >/dev/null 2>&1; then
  echo "missing required tool: tree-sitter" >&2
  echo "Install the pinned Tree-sitter CLI used by the editor toolchain." >&2
  exit 2
fi

echo "== Tree-sitter version =="
tree-sitter --version

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter.XXXXXX")"
query_output="$scratch/query-captures.txt"
id_query_output="$scratch/id-recovery-captures.txt"
recovery_output="$scratch/recovery-tree.txt"
canonical_source="$scratch/canonical.recite"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT

echo "== generated parser reproducibility =="
mkdir -p "$scratch/grammar"
cp -R "$grammar_dir/." "$scratch/grammar/"
rm -rf "$scratch/grammar/src"
(
  cd "$scratch/grammar"
  tree-sitter generate --abi "$parser_abi"
)
for generated_file in \
  src/grammar.json \
  src/node-types.json \
  src/parser.c \
  src/tree_sitter/alloc.h \
  src/tree_sitter/array.h \
  src/tree_sitter/parser.h; do
  if ! cmp -s "$grammar_dir/$generated_file" "$scratch/grammar/$generated_file"; then
    echo "checked-in generated file is stale: $grammar_dir/$generated_file" >&2
    diff -u "$grammar_dir/$generated_file" "$scratch/grammar/$generated_file" | sed -n '1,100p' >&2 || true
    exit 1
  fi
done

echo "== native parser build (ABI $parser_abi) =="
tree-sitter build --output "$scratch/recite-tree-sitter.so" "$grammar_dir"
if [[ ! -s "$scratch/recite-tree-sitter.so" ]]; then
  echo "Tree-sitter native parser build produced no library" >&2
  exit 1
fi

echo "== canonical fixture linkage =="
awk '
  NR <= 3 { next }
  /^---$/ { exit }
  { lines[++count] = $0 }
  END {
    if (count > 0 && lines[count] == "") count--
    for (i = 1; i <= count; i++) print lines[i]
  }
' "$canonical_corpus" > "$canonical_source"
if ! cmp -s "$canonical_fixture" "$canonical_source"; then
  echo "canonical Tree-sitter corpus source diverges from shared Recite fixture:" >&2
  diff -u "$canonical_fixture" "$canonical_source" >&2 || true
  exit 1
fi

echo "== corpus =="
(
  cd "$grammar_dir"
  tree-sitter test --overview-only
)

echo "== highlight query captures =="
(
  cd "$grammar_dir"
  tree-sitter query --grammar-path . --captures queries/highlights.scm \
    test/fixtures/capture-values.recite
) > "$query_output"

required_captures=(
  keyword
  keyword.conditional
  punctuation.special
  label
  variable
  constant.builtin
  property
  constant
  string
  number
  boolean
  variable.builtin
  operator
  function.call
  punctuation.bracket
  punctuation.delimiter
  string.special
  tag
  variable.parameter
)
for capture in "${required_captures[@]}"; do
  if ! grep -Fq " - $capture," "$query_output"; then
    echo "required highlight capture is not exercised: @$capture" >&2
    exit 1
  fi
done

exact_captures=(
  ' - string, start: (0, 27), end: (0, 38), text: `"Archivist"`'
  ' - number, start: (0, 67), end: (0, 72), text: `-12.5`'
  ' - number, start: (0, 79), end: (0, 81), text: `+7`'
  ' - operator, start: (1, 58), end: (1, 59), text: `=`'
  ' - punctuation.delimiter, start: (0, 92), end: (0, 93), text: `,`'
  ' - punctuation.delimiter, start: (1, 64), end: (1, 65), text: `:`'
  ' - string.special, start: (3, 2), end: (3, 14), text: `Plain prose.`'
  ' - string.special, start: (4, 2), end: (4, 15), text: `Interpolated `'
  ' - variable.parameter, start: (4, 16), end: (4, 20), text: `name`'
  ' - string.special, start: (5, 3), end: (5, 11), text: ` Plural `'
  ' - variable.parameter, start: (5, 12), end: (5, 17), text: `count`'
  ' - function.call, start: (7, 14), end: (7, 22), text: `play_sfx`'
  ' - number, start: (7, 32), end: (7, 37), text: `-4.25`'
  ' - constant.builtin, start: (9, 5), end: (9, 8), text: `END`'
  ' - string.special, start: (10, 2), end: (10, 21), text: `- Hyphen-led prose.`'
  ' - string.special, start: (11, 2), end: (11, 21), text: `:ifx this is prose.`'
  ' - string.special, start: (12, 2), end: (12, 23), text: `:elsex this is prose.`'
  ' - string.special, start: (13, 2), end: (13, 24), text: `:matchx this is prose.`'
  ' - string.special, start: (14, 2), end: (14, 23), text: `:casex this is prose.`'
  ' - string.special, start: (15, 2), end: (15, 12), text: `:if{value}`'
  ' - string.special, start: (16, 2), end: (16, 12), text: `:else[tag]`'
  ' - string.special, start: (17, 2), end: (17, 15), text: `:match(value)`'
  ' - string.special, start: (18, 2), end: (18, 16), text: `:case[variant]`'
  ' - keyword.conditional, start: (19, 2), end: (19, 5), text: `:if`'
  ' - keyword.conditional, start: (20, 2), end: (20, 7), text: `:else`'
  ' - keyword.conditional, start: (21, 2), end: (21, 8), text: `:match`'
  ' - keyword.conditional, start: (22, 2), end: (22, 7), text: `:case`'
)
for expectation in "${exact_captures[@]}"; do
  if ! grep -Fq "$expectation" "$query_output"; then
    echo "exact highlight capture expectation is missing: $expectation" >&2
    exit 1
  fi
done

echo "== syntax-only ID recovery =="
id_recovery_output="$scratch/id-recovery.tree"
id_recovery_rc=0
if (
  cd "$repo_root"
  tree-sitter parse --grammar-path "$grammar_dir" "$id_recovery_fixture"
) > "$id_recovery_output" 2>&1; then
  id_recovery_rc=0
else
  id_recovery_rc=$?
fi
if (( id_recovery_rc > 1 )); then
  echo "Tree-sitter ID recovery probe failed to run (exit $id_recovery_rc)" >&2
  exit 1
fi
if grep -Eq '\((ERROR|MISSING)( |\))' "$id_recovery_output"; then
  echo "malformed or draft ID collapsed the Tree-sitter recovery tree" >&2
  sed -n '/ERROR\|MISSING/p' "$id_recovery_output" | sed -n '1,40p' >&2
  exit 1
fi
for node in line_statement choice_statement; do
  if [[ "$(grep -Fc "($node" "$id_recovery_output")" -lt 1 ]]; then
    echo "ID recovery probe lost a $node node" >&2
    exit 1
  fi
done
for node in draft_id stable_id; do
  if ! grep -Fq "($node" "$id_recovery_output"; then
    echo "ID recovery probe did not retain $node classification" >&2
    exit 1
  fi
done

(
  cd "$grammar_dir"
  tree-sitter query --grammar-path . --captures queries/highlights.scm \
    test/fixtures/id-recovery.recite
) > "$id_query_output"
for expectation in \
  ' - label, start: (5, 12), end: (5, 15), text: `bad`' \
  ' - label, start: (7, 9), end: (7, 22), text: `NOT_AN_ANCHOR`' \
  ' - label, start: (12, 7), end: (12, 28), text: `0123456789abcdef01234`' \
  ' - label, start: (14, 14), end: (14, 38), text: `0123456789abcdef0123junk`' \
  ' - label, start: (16, 8), end: (16, 28), text: `0123456789abcdef0123`'; do
  if ! grep -Fq "$expectation" "$id_query_output"; then
    echo "ID recovery capture expectation is missing: $expectation" >&2
    exit 1
  fi
done
for unexpected in \
  ' - label, start: (12, 7), end: (12, 27), text: `0123456789abcdef0123`' \
  ' - label, start: (14, 14), end: (14, 34), text: `0123456789abcdef0123`'; do
  if grep -Fq "$unexpected" "$id_query_output"; then
    echo "malformed ID retained an accidental stable-ID prefix capture: $unexpected" >&2
    exit 1
  fi
done

semantic_output="$scratch/id-recovery-semantic.txt"
semantic_rc=0
if cargo run --quiet --locked --manifest-path "$repo_root/Cargo.toml" -p recite-cli -- \
  check-ids "$id_recovery_fixture" > "$semantic_output" 2>&1; then
  semantic_rc=0
else
  semantic_rc=$?
fi
if (( semantic_rc == 0 )); then
  echo "compiler-facing ID validation unexpectedly accepted the recovery fixture" >&2
  exit 1
fi
if ! grep -Eq 'RECITE_ID00[78]' "$semantic_output"; then
  echo "compiler-facing ID validation did not report a stable-ID diagnostic" >&2
  sed -n '1,80p' "$semantic_output" >&2
  exit 1
fi
echo "syntax-only ID recovery passed; semantic validation remains compiler-owned"

echo "== incomplete input recovery =="
recovery_rc=0
if (
  cd "$grammar_dir"
  tree-sitter parse --grammar-path . test/fixtures/incomplete.recite
) > "$recovery_output"; then
  recovery_rc=0
else
  recovery_rc=$?
fi
if (( recovery_rc > 1 )); then
  echo "Tree-sitter recovery probe failed to run (exit $recovery_rc)" >&2
  exit 1
fi
if ! grep -Eq '\((ERROR|MISSING)( |\))' "$recovery_output"; then
  echo "incomplete fixture did not expose an explicit recovery node" >&2
  exit 1
fi

echo "== canonical fixture differential parse =="
valid_count=0
invalid_count=0
while IFS= read -r -d '' fixture; do
  fixture_output="$scratch/$(basename "$fixture").tree"
  fixture_rc=0
  if (
    cd "$repo_root"
    tree-sitter parse --grammar-path "$grammar_dir" "$fixture"
  ) > "$fixture_output" 2>&1; then
    fixture_rc=0
  else
    fixture_rc=$?
  fi
  if [[ "$fixture" == "$repo_root/fixtures/recite/valid/"* ]]; then
    valid_count=$((valid_count + 1))
    if (( fixture_rc != 0 )) || grep -Eq '\((ERROR|MISSING)( |\))' "$fixture_output"; then
      echo "canonical valid fixture does not parse cleanly: ${fixture#$repo_root/}" >&2
      sed -n '/ERROR\|MISSING/p' "$fixture_output" | sed -n '1,40p' >&2
      exit 1
    fi
  elif [[ "$fixture" == "$repo_root/fixtures/recite/invalid/"* ]]; then
    invalid_count=$((invalid_count + 1))
    if (( fixture_rc > 1 )); then
      echo "canonical invalid fixture failed to produce a parse tree: ${fixture#$repo_root/} (exit $fixture_rc)" >&2
      exit 1
    fi
  fi
done < <(find "$repo_root/fixtures/recite" -type f -name '*.recite' -print0 | sort -z)
if (( valid_count == 0 || invalid_count == 0 )); then
  echo "canonical fixture differential did not cover both valid and invalid .recite inputs" >&2
  exit 1
fi
echo "canonical fixture differential passed: $valid_count valid, $invalid_count invalid"

echo "== CRLF, non-BMP, and unexpected punctuation probes =="
crlf_fixture="$scratch/canonical-crlf-non-bmp.recite"
awk '{ sub(/tide/, "tide 🌊"); printf "%s\r\n", $0 }' "$canonical_fixture" > "$crlf_fixture"
crlf_output="$scratch/crlf.tree"
if ! (
  cd "$repo_root"
  tree-sitter parse --grammar-path "$grammar_dir" "$crlf_fixture"
) > "$crlf_output" 2>&1; then
  echo "canonical CRLF/non-BMP fixture failed to parse" >&2
  exit 1
fi
if grep -Eq '\((ERROR|MISSING)( |\))' "$crlf_output"; then
  echo "canonical CRLF/non-BMP fixture produced a recovery node" >&2
  exit 1
fi

punctuation_fixture="$scratch/unexpected-punctuation.recite"
printf '%s\n' \
  ':: recovery' \
  ':if stage("open") == 2' \
  '> later@0123456789abcdef0123' \
  '  Later prose.' \
  '-> END' > "$punctuation_fixture"
punctuation_output="$scratch/unexpected-punctuation.tree"
punctuation_rc=0
if (
  cd "$repo_root"
  tree-sitter parse --grammar-path "$grammar_dir" "$punctuation_fixture"
) > "$punctuation_output" 2>&1; then
  punctuation_rc=0
else
  punctuation_rc=$?
fi
if (( punctuation_rc > 1 )) || ! grep -Eq '\((ERROR|MISSING)( |\))' "$punctuation_output"; then
  echo "unexpected punctuation probe did not expose a bounded recovery node" >&2
  exit 1
fi
for node in line_statement prose_line divert_statement; do
  if ! grep -Fq "($node" "$punctuation_output"; then
    echo "unexpected punctuation probe lost later $node" >&2
    exit 1
  fi
done
echo "edge-case probes passed"

"$repo_root/scripts/check-tree-sitter-physical-lines.sh" "$repo_root"

echo "Tree-sitter grammar checks passed."
