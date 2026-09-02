#!/usr/bin/env bash

maintainability_collect_paths() {
  local repo_root="$1"
  local base_sha="$2"
  local head_sha="$3"
  local empty_base="$4"
  local status base_path path
  local -n output_paths="$5"
  local -n output_base_paths="$6"
  local -n output_renamed_paths="$7"

  if (( empty_base )); then
    diff_command=(git -C "$repo_root" diff --name-status -z -M --diff-filter=ACMR "$base_sha" "$head_sha" --)
    diff_range="$base_sha $head_sha"
  else
    diff_command=(git -C "$repo_root" diff --name-status -z -M --diff-filter=ACMR "${base_sha}...${head_sha}" --)
    # shellcheck disable=SC2034
    diff_range="$base_sha...$head_sha"
  fi

  while IFS= read -r -d '' status; do
    case "$status" in
      R*|C*)
        if ! IFS= read -r -d '' base_path || ! IFS= read -r -d '' path; then
          echo "malformed changed-path record from git diff: $status" >&2
          return 2
        fi
        # shellcheck disable=SC2034
        output_renamed_paths["$path"]=1
        ;;
      *)
        if ! IFS= read -r -d '' path; then
          echo "malformed changed-path record from git diff: $status" >&2
          return 2
        fi
        base_path="$path"
        ;;
    esac
    output_paths+=("$path")
    # shellcheck disable=SC2034
    output_base_paths["$path"]="$base_path"
  done < <("${diff_command[@]}")
}
