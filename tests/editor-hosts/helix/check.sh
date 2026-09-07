#!/usr/bin/env bash
set -euo pipefail

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi
checker="$repo_root/scripts/check-helix.sh"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/recite-helix-check.XXXXXX")"
cleanup() {
  rm -rf -- "$test_root"
}
trap cleanup EXIT

fixture="$test_root/repo"
mkdir -p \
  "$fixture/editors/helix/runtime/queries/recite" \
  "$fixture/editors/recite-tree-sitter/queries"
cp -- "$repo_root/editors/helix/README.md" "$fixture/editors/helix/README.md"
cp -- "$repo_root/editors/helix/languages.toml" "$fixture/editors/helix/languages.toml"
cp -- "$repo_root/editors/helix/runtime/queries/recite/highlights.scm" \
  "$fixture/editors/helix/runtime/queries/recite/highlights.scm"
cp -- "$repo_root/editors/recite-tree-sitter/queries/highlights.scm" \
  "$fixture/editors/recite-tree-sitter/queries/highlights.scm"
git -C "$fixture" init -q

fake_output="$test_root/fake-output"
if HELIX_BIN=/usr/bin/echo "$checker" "$fixture" >"$fake_output" 2>&1; then
  echo "Helix hostile fixture accepted /usr/bin/echo as HELIX_BIN" >&2
  exit 1
fi
if ! grep -Fq "not a Helix executable" "$fake_output"; then
  echo "Helix hostile fixture rejected fake binary for the wrong reason" >&2
  sed -n '1,80p' "$fake_output" >&2
  exit 1
fi

negative_fake="$test_root/negative-helix"
cat >"$negative_fake" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  --version)
    echo "helix 25.07.1 (hostile-fixture)"
    ;;
  --health)
    cat <<'REPORT'
Configured language servers:
  recite-lsp: configured
Tree-sitter parser: None
Highlight queries: missing
REPORT
    ;;
  *)
    exit 1
    ;;
esac
EOF
chmod +x "$negative_fake"
negative_output="$test_root/negative-output"
if HELIX_BIN="$negative_fake" "$checker" "$fixture" >"$negative_output" 2>&1; then
  echo "Helix hostile fixture accepted a negative highlight-query report" >&2
  exit 1
fi
if ! grep -Fq "positive Recite highlight query" "$negative_output"; then
  echo "Helix hostile fixture rejected negative highlight-query report for the wrong reason" >&2
  sed -n '1,80p' "$negative_output" >&2
  exit 1
fi

sed -i 's/cb0e34a94709df290ebadcf371bcfd4e3ad587b1/0000000000000000000000000000000000000000/' \
  "$fixture/editors/helix/languages.toml"
revision_output="$test_root/revision-output"
if "$checker" "$fixture" >"$revision_output" 2>&1; then
  echo "Helix hostile fixture accepted an unapproved grammar revision" >&2
  exit 1
fi
if ! grep -Fq "approved public base" "$revision_output"; then
  echo "Helix hostile fixture rejected wrong grammar revision for the wrong reason" >&2
  sed -n '1,80p' "$revision_output" >&2
  exit 1
fi

echo "Helix hostile checks passed: fake binary and unapproved grammar revision fail closed."
