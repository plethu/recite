#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-tree-sitter.sh

Checks the syntax-only Recite Tree-sitter grammar. The check verifies that the
checked-in generated parser is reproducible, the corpus passes, the canonical
corpus source remains linked to the shared Recite fixture, required highlight
captures are present, and incomplete input produces a recoverable tree.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if (( $# > 0 )); then
  usage >&2
  exit 2
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "unable to resolve Git repository root" >&2
  exit 2
}
grammar_dir="$repo_root/editor/recite-tree-sitter"
canonical_fixture="$repo_root/fixtures/recite/valid/language_pressure.recite"
canonical_corpus="$grammar_dir/test/corpus/canonical.txt"
capture_fixture="$grammar_dir/test/fixtures/capture-values.recite"
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
  "$capture_fixture" \
  "$incomplete_fixture"; do
  if [[ ! -f "$required_file" ]]; then
    echo "missing required Tree-sitter grammar file: $required_file" >&2
    exit 2
  fi
done

if ! command -v tree-sitter >/dev/null 2>&1; then
  echo "missing required tool: tree-sitter" >&2
  echo "Install the pinned Tree-sitter CLI used by the editor toolchain." >&2
  exit 2
fi

echo "== Tree-sitter version =="
tree-sitter --version

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-tree-sitter.XXXXXX")"
query_output="$scratch/query-captures.txt"
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
  tree-sitter generate
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

echo "Tree-sitter grammar checks passed."
