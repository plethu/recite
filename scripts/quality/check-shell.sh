#!/usr/bin/env bash
set -euo pipefail

# Sourced modules are checked through the owning entrypoint, with its variables.
files=()
while IFS= read -r -d '' path; do
  case "$path" in
    scripts/maintainability/*.sh | tests/editor-parity/hostile_cases.sh) continue ;;
  esac
  files+=("$path")
done < <(rg --files --hidden --null --glob '*.sh' --glob '!.git/**')
shellcheck "${files[@]}"
