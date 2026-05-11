#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 issue|metadata -- tea <args...>" >&2
  exit 2
}

mode="${1:-}"
[[ "$mode" == "issue" || "$mode" == "metadata" ]] || usage
shift
[[ "${1:-}" == "--" ]] || usage
shift
[[ "${1:-}" == "tea" ]] || usage

state_dir="${TMPDIR:-/tmp}/recite-tea-rate-limit"
lock_dir="$state_dir/lock"
stamp_file="$state_dir/$mode.last"
mkdir -p "$state_dir"

if ! mkdir "$lock_dir" 2>/dev/null; then
  echo "another recite tea mutation appears to be running; refusing concurrent mutation" >&2
  exit 1
fi
trap 'rmdir "$lock_dir"' EXIT

case "$mode" in
  issue) min_interval=75 ;;
  metadata) min_interval=10 ;;
esac

now="$(date +%s)"
if [[ -f "$stamp_file" ]]; then
  last="$(cat "$stamp_file")"
  if [[ ! "$last" =~ ^[0-9]+$ ]]; then
    echo "corrupt rate-limit stamp file: $stamp_file" >&2
    exit 1
  fi

  elapsed=$((now - last))
  if (( elapsed < min_interval )); then
    sleep_for=$((min_interval - elapsed))
    echo "Codeberg courtesy throttle: sleeping ${sleep_for}s before $mode mutation" >&2
    sleep "$sleep_for"
  fi
fi

set +e
output="$("$@" 2>&1)"
status=$?
set -e
printf '%s\n' "$output"

if (( status == 0 )); then
  date +%s > "$stamp_file"
  exit 0
fi

if printf '%s\n' "$output" | grep -Eiq 'rate.?limit|too many requests|retry-after'; then
  wait_hint="$(printf '%s\n' "$output" | grep -Eio '[0-9]+[[:space:]]*(seconds?|minutes?)' | tail -n 1 || true)"
  echo "Codeberg rate limit detected. Stop this remote-mutation pass before retrying." >&2
  if [[ -n "$wait_hint" ]]; then
    echo "Server/client message mentioned: $wait_hint" >&2
  else
    echo "No explicit wait was exposed by tea; wait at least 15 minutes." >&2
  fi
fi

if printf '%s\n' "$output" | grep -Eiq 'HTTP/[0-9.]+[[:space:]]+5[0-9][0-9]|status[[:space:]]+5[0-9][0-9]|bad gateway|service unavailable|gateway timeout'; then
  echo "Possible Forgejo/Codeberg 5xx detected. Stop this remote-mutation pass and surface the failure." >&2
fi

exit "$status"
