#!/usr/bin/env bash

# Path policy is deliberately explicit.  The maintainability check is a
# review signal for handwritten source, not a generic scan of every file in a
# repository.

maintainability_is_test_path() {
  local path="$1"
  [[ "$path" == crates/*/tests/* \
    || "$path" == crates/*/benches/* \
    || "$path" == crates/*/src/tests.rs \
    || "$path" == crates/*/src/tests/* \
    || "$path" == */src/*/tests.rs \
    || "$path" == tests/* \
    || "$path" == editors/*/test/* \
    || "$path" == editors/*/tests/* ]]
}

maintainability_is_tooling_path() {
  local path="$1"
  [[ "$path" == scripts/* \
    || "$path" == editors/*/scripts/* \
    || "$path" == .agents/*/scripts/* \
    || "$path" == .agents/*/*/scripts/* \
    || "$path" == .agents/*/*/*/scripts/* ]]
}

maintainability_is_rust_source_path() {
  local path="$1"
  [[ "$path" == crates/*/src/* || "$path" == crates/*/tests/* \
    || "$path" == crates/*/benches/* \
    || "$path" == tests/* ]]
}

maintainability_is_supported_extension() {
  case "$1" in
    *.rs|*.js|*.mjs|*.cjs|*.lua|*.py|*.sh)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

maintainability_is_excluded_path() {
  case "$1" in
    target/*|include/recite.h|fixtures/generated/* \
      |editors/vscode/src/messages.generated.js \
      |editors/recite-neovim/lua/recite_messages.lua \
      |editors/recite-tree-sitter/src/parser.c \
      |editors/recite-tree-sitter/src/grammar.json \
      |editors/recite-tree-sitter/src/node-types.json)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

maintainability_is_regular_file_at() {
  local repo_root="$1"
  local revision="$2"
  local path="$3"
  local mode
  mode="$(git -C "$repo_root" ls-tree "$revision" -- "$path" | awk 'NF { print $1 }')"
  [[ "$mode" == 100644 || "$mode" == 100755 ]]
}

maintainability_is_valid_path() {
  local path="$1"
  maintainability_is_supported_extension "$path" || return 1
  case "$path" in
    ''|*/../*|*/..|../*|./*|*/./*)
      return 1
      ;;
  esac
  maintainability_is_excluded_path "$path" && return 1

  if [[ "$path" == *.rs ]]; then
    maintainability_is_rust_source_path "$path"
  else
    return 0
  fi
}

maintainability_classify_path() {
  local path="$1"
  maintainability_is_valid_path "$path" || return 1
  if maintainability_is_test_path "$path"; then
    printf 'test/support\n'
  elif maintainability_is_tooling_path "$path"; then
    printf 'tooling\n'
  else
    printf 'production\n'
  fi
}

maintainability_scrutiny_threshold() {
  case "$1" in
    production|tooling) printf '250\n' ;;
    test/support) printf '350\n' ;;
    *) return 1 ;;
  esac
}

maintainability_follow_up_threshold() {
  case "$1" in
    production|tooling) printf '400\n' ;;
    test/support) printf '500\n' ;;
    *) return 1 ;;
  esac
}

maintainability_line_count_at() {
  local repo_root="$1"
  local revision="$2"
  local path="$3"
  if ! git -C "$repo_root" cat-file -e "${revision}:${path}" 2>/dev/null; then
    printf '0\n'
    return
  fi
  git -C "$repo_root" show "${revision}:${path}" | awk 'END { print NR + 0 }'
}
