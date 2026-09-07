#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-helix.sh [repo-root]

Checks the source Helix language configuration and syntax-query projection.
This is a package/configuration gate; it does not claim installed-host
keyboard evidence.
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
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

helix_dir="$repo_root/editors/helix"
language_file="$helix_dir/languages.toml"
query_file="$helix_dir/runtime/queries/recite/highlights.scm"
grammar_query="$repo_root/editors/recite-tree-sitter/queries/highlights.scm"

for required in "$helix_dir/README.md" "$language_file" "$query_file"; do
  if [[ ! -f "$required" || -L "$required" ]]; then
    echo "missing or symlinked Helix integration file: ${required#"$repo_root"/}" >&2
    exit 2
  fi
done

if ! cmp -s "$grammar_query" "$query_file"; then
  echo "Helix highlight query diverges from the host-neutral Tree-sitter query" >&2
  diff -u "$grammar_query" "$query_file" | sed -n '1,120p' >&2 || true
  exit 1
fi

python3 - "$language_file" <<'PY'
import pathlib
import sys
import tomllib

language_path = pathlib.Path(sys.argv[1])
with language_path.open("rb") as handle:
    document = tomllib.load(handle)

if document.get("use-grammars") != {"only": ["recite"]}:
    raise SystemExit('Helix grammar selection must be exactly {"only": ["recite"]}')

server = document.get("language-server", {}).get("recite-lsp")
expected_server = {"command": "recite-lsp", "args": [], "timeout": 20}
if server != expected_server:
    raise SystemExit(f"recite-lsp language-server drifted: {server!r}")

languages = [entry for entry in document.get("language", []) if entry.get("name") == "recite"]
if len(languages) != 1:
    raise SystemExit("languages.toml must contain exactly one recite language")
language = languages[0]
for key, expected in {
    "language-id": "recite",
    "scope": "source.recite",
    "injection-regex": "recite",
    "file-types": ["recite"],
    "roots": ["recite.project.toml"],
    "language-servers": ["recite-lsp"],
    "grammar": "recite",
}.items():
    if language.get(key) != expected:
        raise SystemExit(f"recite language {key} drifted: {language.get(key)!r}")

grammars = [entry for entry in document.get("grammar", []) if entry.get("name") == "recite"]
if len(grammars) != 1:
    raise SystemExit("languages.toml must contain exactly one recite grammar")
grammar = grammars[0]
source = grammar.get("source", {})
if source.get("git") != "https://github.com/plethu/recite":
    raise SystemExit(f"recite grammar source drifted: {source!r}")
if source.get("subpath") != "editors/recite-tree-sitter":
    raise SystemExit(f"recite grammar subpath drifted: {source!r}")
approved_revision = "cb0e34a94709df290ebadcf371bcfd4e3ad587b1"
if source.get("rev") != approved_revision:
    raise SystemExit(
        "recite grammar revision must be the approved public base "
        f"{approved_revision}, got {source.get('rev')!r}"
    )

print(
    "Helix package contract passed: .recite filetype, recite-lsp stdio, "
    "pinned nested Tree-sitter grammar, and explicit project root marker."
)
PY

helix_bin="${HELIX_BIN:-}"
if [[ -z "$helix_bin" ]]; then
  helix_bin="$(command -v helix 2>/dev/null || true)"
fi
if [[ -z "$helix_bin" ]]; then
  echo "Helix binary unavailable; installed-host health check not claimed"
  exit 0
fi
if [[ ! -x "$helix_bin" ]]; then
  echo "HELIX_BIN is not executable: $helix_bin" >&2
  exit 2
fi

if ! version_output="$("$helix_bin" --version 2>&1)"; then
  echo "HELIX_BIN did not accept --version: $helix_bin" >&2
  sed -n '1,20p' <<<"$version_output" >&2
  exit 1
fi
version_line="${version_output%%$'\n'*}"
if [[ ! "$version_line" =~ ^helix[[:space:]][0-9]+\.[0-9]+\.[0-9]+([[:space:]]|$) ]]; then
  echo "HELIX_BIN is not a Helix executable: $helix_bin" >&2
  sed -n '1,20p' <<<"$version_output" >&2
  exit 1
fi

probe="$(mktemp -d "${TMPDIR:-/tmp}/recite-helix.XXXXXX")"
cleanup() {
  rm -rf -- "$probe"
}
trap cleanup EXIT
mkdir -p "$probe/config/helix/runtime/queries/recite"
cp "$language_file" "$probe/config/helix/languages.toml"
cp "$query_file" "$probe/config/helix/runtime/queries/recite/highlights.scm"

health="$probe/health.txt"
if ! XDG_CONFIG_HOME="$probe/config" HELIX_RUNTIME="$probe/config/helix/runtime" \
  "$helix_bin" --health recite >"$health" 2>&1; then
  echo "Helix rejected the isolated recite language configuration:" >&2
  sed -n '1,120p' "$health" >&2
  exit 1
fi
if ! rg -q -i 'recite' "$health"; then
  echo "Helix health output did not identify the recite language" >&2
  sed -n '1,120p' "$health" >&2
  exit 1
fi
for health_header in "Configured language servers:" "Tree-sitter parser:" "Highlight queries:"; do
  if ! rg -Fq "$health_header" "$health"; then
    echo "HELIX_BIN did not produce a real language health report: $helix_bin" >&2
    sed -n '1,120p' "$health" >&2
    exit 1
  fi
done
if ! rg -q 'Highlight queries:.*✓' "$health"; then
  echo "HELIX_BIN did not report positive Recite highlight query discovery: $helix_bin" >&2
  sed -n '1,120p' "$health" >&2
  exit 1
fi
echo "Helix $version_line discovered the isolated recite language configuration and query"
