#!/usr/bin/env bash
set -Eeuo pipefail

# This is deliberately a host probe, not part of the normal source/package
# gate. It runs Zed in a private headless Cage compositor and fails closed if
# an observable host boundary cannot be exercised. All mutable state is kept
# in a temporary directory, including the dev-extension checkout: Zed writes
# extension.wasm and generated grammar files beside that checkout.

usage() {
  cat <<'EOF'
Usage:
  scripts/check-zed-host.sh [repo-root]

Builds (or uses RECITE_LSP_BIN/RECITE_CLI_BIN), installs the checked-in Zed
extension as a development extension, then exercises it in an isolated Linux
Zed host. The host must provide zed-editor, Cage, wtype, grim, dbus-run-session,
and a working Vulkan/Wayland headless compositor. No live DISPLAY or
WAYLAND_DISPLAY is used.

Environment:
  ZED_EDITOR          direct zed-editor binary (not the zeditor client)
  RECITE_LSP_BIN      prebuilt recite-lsp; otherwise Cargo builds it under TMPDIR
  RECITE_CLI_BIN      prebuilt recite CLI; otherwise Cargo builds it under TMPDIR
  TMPDIR              private writable parent for the bounded host probe (default /tmp)
  RECITE_ZED_TIMEOUT  per-stage timeout in seconds (default: 90)
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

extension_dir="$repo_root/editors/zed"
fixture="$repo_root/fixtures/recite/invalid/parser_marker_leading_prose.recite"
capability_fixture="$repo_root/fixtures/recite/valid/core_language_spike.recite"
grammar_revision="209ea23195f674a18be0b8f87e037273fb3296bd"
timeout_seconds="${RECITE_ZED_TIMEOUT:-90}"
keep_probe="${RECITE_ZED_KEEP:-}"
if ! [[ "$timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
  echo "RECITE_ZED_TIMEOUT must be a positive integer" >&2
  exit 2
fi

for required in "$extension_dir/extension.toml" \
  "$extension_dir/languages/recite/config.toml" \
  "$extension_dir/languages/recite/highlights.scm" \
  "$extension_dir/languages/recite/tasks.json" "$fixture" "$capability_fixture"; do
  [[ -f "$required" && ! -L "$required" ]] || {
    echo "missing or symlinked Zed host input: ${required#"$repo_root"/}" >&2
    exit 2
  }
done
proxy_script="$repo_root/tests/editor-hosts/zed/lsp_proxy.py"
[[ -f "$proxy_script" && ! -L "$proxy_script" ]] || {
  echo "missing or symlinked Zed LSP probe transport: ${proxy_script#"$repo_root"/}" >&2
  exit 2
}
assert_lsp_log="$repo_root/tests/editor-hosts/zed/assert_lsp_log.py"
[[ -f "$assert_lsp_log" && ! -L "$assert_lsp_log" ]] || {
  echo "missing or symlinked Zed LSP evidence assertion: ${assert_lsp_log#"$repo_root"/}" >&2
  exit 2
}

command_path() {
  command -v "$1" 2>/dev/null || true
}

zed_bin="${ZED_EDITOR:-}"
if [[ -z "$zed_bin" ]]; then
  zed_bin="$(command_path zed-editor)"
fi
if [[ -z "$zed_bin" ]]; then
  echo "missing direct zed-editor executable; set ZED_EDITOR" >&2
  exit 2
fi
for tool in cage wtype grim dbus-run-session sha256sum python3; do
  command_path "$tool" >/dev/null || {
    echo "missing Zed host probe tool: $tool" >&2
    exit 2
  }
done
[[ -x "$zed_bin" ]] || {
  echo "ZED_EDITOR is not executable: $zed_bin" >&2
  exit 2
}

if [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]; then
  echo "INFO: caller has a display, but the probe will unset it and use private Cage state" >&2
fi

probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/recite-zed-host.XXXXXX")"
runtime_dir="$probe_dir/runtime"
user_data="$probe_dir/user-data"
config_home="$probe_dir/config"
data_home="$probe_dir/data"
cache_home="$probe_dir/cache"
home_dir="$probe_dir/home"
project_dir="$probe_dir/project"
extension_copy="$probe_dir/extension"
bin_dir="$probe_dir/bin"
mkdir -p "$runtime_dir" "$user_data/config" "$config_home/zed" "$data_home/config" \
  "$data_home/dbus-1/services" "$cache_home" "$home_dir" "$project_dir/.zed" \
  "$bin_dir"
chmod 700 "$runtime_dir"

cleanup_status=0
evidence_failure=0
cage_pid=""
probe_processes() {
  local process_root="${cage_pid:-0}"
  # Match the unique probe path and every descendant of this run's private
  # dbus-run-session/Cage root. The ancestry check also catches a private
  # dbus-daemon or interpreter whose argv does not repeat the temporary path.
  ps -eo pid=,ppid=,args= | awk -v probe="$probe_dir" -v root="$process_root" -v self="$$" '
    {
      pid = $1
      parent[pid] = $2
      command[pid] = $0
      marked[pid] = (index($0, probe) > 0)
    }
    END {
      if (root != "0" && (root in command)) {
        marked[root] = 1
      }
      changed = 1
      while (changed) {
        changed = 0
        for (pid in command) {
          if (!marked[pid] && marked[parent[pid]]) {
            marked[pid] = 1
            changed = 1
          }
        }
      }
      for (pid in command) {
        if (marked[pid] && pid != self && command[pid] !~ /(^|[ \/])awk([ \/]|$)/ && command[pid] !~ /rg -F/) {
          print command[pid]
        }
      }
    }'
}
probe_pids() {
  probe_processes | awk '!seen[$1]++ {print $1}'
}
terminate_probe_processes() {
  local -a pids remaining
  mapfile -t pids < <(probe_pids)
  if (( ! ${#pids[@]} )); then
    return 0
  fi

  # A normal host stop must leave no process carrying this probe's private
  # path. Recovery is bounded and safe, but it is evidence failure even when
  # the recovery succeeds: a host that ignores its keyboard shutdown boundary
  # has not passed the installed-host lane.
  evidence_failure=1
  echo "ERROR: private probe processes required cleanup recovery: ${pids[*]}" >&2
  kill "${pids[@]}" 2>/dev/null || true
  for _ in {1..50}; do
    mapfile -t remaining < <(probe_pids)
    (( ! ${#remaining[@]} )) && {
      echo "INFO: private probe processes exited after bounded TERM" >&2
      return 1
    }
    sleep 0.1
  done
  if (( ${#remaining[@]} )); then
    echo "ERROR: private probe processes survived bounded TERM; sending KILL: ${remaining[*]}" >&2
    kill -KILL "${remaining[@]}" 2>/dev/null || true
  fi
  for _ in {1..50}; do
    mapfile -t remaining < <(probe_pids)
    (( ! ${#remaining[@]} )) && {
      echo "INFO: private probe processes exited after bounded KILL" >&2
      return 1
    }
    sleep 0.1
  done
  echo "ERROR: private probe process remained after bounded KILL: ${remaining[*]}" >&2
  return 1
}
cleanup() {
  cleanup_status=$?
  terminate_probe_processes || true
  if [[ -n "$cage_pid" ]]; then
    wait "$cage_pid" 2>/dev/null || true
    cage_pid=""
  fi
  if [[ -d "$probe_dir" && -z "$keep_probe" ]]; then
    # The probe owns this exact mktemp directory. Keep no screenshots,
    # extension artifacts, Cargo output, DBus services, or logs by default.
    rm -rf -- "$probe_dir"
  elif [[ -d "$probe_dir" ]]; then
    echo "INFO: keeping probe artifacts at $probe_dir" >&2
  fi
  if (( evidence_failure )) && (( cleanup_status == 0 )); then
    cleanup_status=1
  fi
  exit "$cleanup_status"
}
trap cleanup EXIT INT TERM

echo "== installed Zed Linux host provenance =="
printf 'zed_binary=%s\n' "$zed_bin"
if command -v zeditor >/dev/null 2>&1; then
  printf 'zed_cli_version=%s\n' "$(zeditor --version 2>&1 | head -1)"
else
  printf 'zed_cli_version=unavailable\n'
fi
printf 'zed_binary_sha256=%s\n' "$(sha256sum "$zed_bin" | awk '{print $1}')"
printf 'architecture=%s\n' "$(uname -m)"
printf 'kernel=%s\n' "$(uname -sr)"
if command -v pacman >/dev/null 2>&1; then
  pacman -Qi zed 2>/dev/null | awk -F': ' '/^(Name|Version|Architecture|Install Date|Packager|Installed From)/ {print tolower($1) "=" $2}' || true
fi

lsp_bin="${RECITE_LSP_BIN:-}"
cli_bin="${RECITE_CLI_BIN:-}"
if [[ -z "$lsp_bin" || -z "$cli_bin" ]]; then
  cargo_bin="$(command_path cargo)"
  [[ -n "$cargo_bin" ]] || {
    echo "RECITE_LSP_BIN and RECITE_CLI_BIN are required when cargo is unavailable" >&2
    exit 2
  }
  cargo_target="$probe_dir/cargo-target"
  echo "== build exact Recite host binaries in temporary target =="
  CARGO_TARGET_DIR="$cargo_target" "$cargo_bin" build --locked -q -p recite-lsp -p recite-cli
  [[ -z "$lsp_bin" ]] && lsp_bin="$cargo_target/debug/recite-lsp"
  [[ -z "$cli_bin" ]] && cli_bin="$cargo_target/debug/recite"
fi
for binary in "$lsp_bin" "$cli_bin"; do
  [[ -x "$binary" ]] || {
    echo "Recite host binary is not executable: $binary" >&2
    exit 2
  }
done
printf 'recite_lsp=%s\n' "$lsp_bin"
lsp_sha256="$(sha256sum "$lsp_bin" | awk '{print $1}')"
printf 'recite_lsp_sha256=%s\n' "$lsp_sha256"
printf 'recite_cli=%s\n' "$cli_bin"
cli_sha256="$(sha256sum "$cli_bin" | awk '{print $1}')"
printf 'recite_cli_sha256=%s\n' "$cli_sha256"

# Keep every language-server/task descendant under the private probe path.
# This makes process ancestry and cleanup assertions specific to this run,
# while preserving the exact hashes of the externally supplied binaries.
lsp_probe_bin="$bin_dir/recite-lsp.real"
cli_probe_bin="$bin_dir/recite.real"
proxy_probe_script="$bin_dir/lsp_proxy.py"
cp -- "$lsp_bin" "$lsp_probe_bin"
cp -- "$cli_bin" "$cli_probe_bin"
cp -- "$proxy_script" "$proxy_probe_script"
chmod 755 "$lsp_probe_bin" "$cli_probe_bin"
[[ "$(sha256sum "$lsp_probe_bin" | awk '{print $1}')" == "$lsp_sha256" ]] || {
  echo "private recite-lsp probe copy changed its source hash" >&2
  exit 1
}
[[ "$(sha256sum "$cli_probe_bin" | awk '{print $1}')" == "$cli_sha256" ]] || {
  echo "private recite CLI probe copy changed its source hash" >&2
  exit 1
}
cmp -s -- "$proxy_script" "$proxy_probe_script" || {
  echo "private LSP probe transport copy changed its checked-in source" >&2
  exit 1
}

printf 'format_version = 1\n' > "$project_dir/recite.project.toml"
printf '{"lsp":{"recite-lsp":{"binary":{"path":"%s","arguments":[]}}}}\n' "$bin_dir/recite-lsp" > "$project_dir/.zed/settings.json"
cp -- "$fixture" "$project_dir/fixture.recite"
cp -- "$capability_fixture" "$project_dir/core.recite"
python3 - "$project_dir/code-action.recite" <<'PY'
from pathlib import Path
import sys

Path(sys.argv[1]).write_text(
    ":: start default\n>\n  Hello.\n?\n  Stay.\n",
    encoding="utf-8",
)
PY
cp -R -- "$extension_dir" "$extension_copy"
rm -rf -- "$extension_copy/target"

# PATH wrappers make the exact argv and exit status of every Zed task and
# language-server launch observable without changing the checked-in commands.
# The task wrapper retains the real child process. The LSP wrapper additionally
# interposes the checked-in transport logger, forwarding all original frames
# while retaining both the logger and real server under this private path.
task_log="$probe_dir/task.log"
cat > "$bin_dir/recite" <<EOF
#!/usr/bin/env bash
set -uo pipefail
printf 'start pid=%s cwd=%s argv=' "\$\$" "\$(pwd)" >> "$task_log"
printf '%q ' "\$@" >> "$task_log"
printf '\n' >> "$task_log"
"$cli_probe_bin" "\$@"
rc=\$?
printf 'exit pid=%s status=%s\n' "\$\$" "\$rc" >> "$task_log"
exit "\$rc"
EOF
chmod 755 "$bin_dir/recite"
lsp_log="$probe_dir/lsp.log"
cat > "$bin_dir/recite-lsp" <<EOF
#!/usr/bin/env bash
set -uo pipefail
env RECITE_PROBE_LSP_REAL="$lsp_probe_bin" RECITE_PROBE_LSP_LOG="$lsp_log" \
  python3 "$proxy_probe_script" "\$@"
EOF
chmod 755 "$bin_dir/recite-lsp"

# Zed 1.18.1 reads user settings from --user-data-dir/config/settings.json.
# Keep the XDG copy as well so this remains clear if the host changes its
# precedence. The service files prevent credential/portal prompts from
# stealing the private keyboard focus; they never run a replacement service.
settings='{"disable_ai":true,"telemetry":{"diagnostics":false,"metrics":false},"cursor_blink":false,"show_sign_in":false,"auto_update":false,"session":{"trust_all_worktrees":true}}'
printf '%s\n' "$settings" > "$user_data/config/settings.json"
printf '%s\n' "$settings" > "$config_home/zed/settings.json"
for service in org.freedesktop.secrets org.freedesktop.portal.Desktop org.a11y.Bus; do
  cat > "$data_home/dbus-1/services/$service.service" <<EOF
[D-BUS Service]
Name=$service
Exec=/bin/false
EOF
done

wayland_display="wayland-0"
zed_log="$user_data/logs/Zed.log"
mkdir -p "$user_data/logs"

host_env=(
  env -u DISPLAY -u WAYLAND_DISPLAY
  "WAYLAND_DISPLAY=$wayland_display"
  WLR_BACKENDS=headless
  WLR_LIBINPUT_NO_DEVICES=1
  WLR_HEADLESS_OUTPUTS=1
  "XDG_RUNTIME_DIR=$runtime_dir"
  "XDG_CONFIG_HOME=$config_home"
  "XDG_DATA_HOME=$data_home"
  "XDG_CACHE_HOME=$cache_home"
  "XDG_DATA_DIRS=$data_home:/usr/local/share:/usr/share"
  "HOME=$home_dir"
  "PATH=$bin_dir:${PATH:-/usr/bin:/bin}"
  "RECITE_PROBE_LSP_REAL=$lsp_probe_bin"
  "RECITE_PROBE_LSP_LOG=$lsp_log"
)

press() {
  "${host_env[@]}" wtype "$@"
}
type_text() {
  "${host_env[@]}" wtype -d 4 -- "$1"
}
open_file() {
  local path="$1"
  # Zed's documented file picker action is Ctrl-P on Linux. Unlike a
  # command-palette search, this leaves the selected path in the picker rather
  # than accidentally inserting it into the current buffer.
  press -M ctrl -k p -m ctrl
  sleep 1
  # A basename gives the file finder a stable unique result in this temporary
  # project and avoids host-version differences in absolute-path matching.
  type_text "${path##*/}"
  press -k Return
  sleep 5
}
place_cursor_in_divert() {
  press -M ctrl -k f -m ctrl
  type_text '-> work'
  press -k Return
  press -k Escape
  press -k Left
  for _ in {1..5}; do
    press -k Right
  done
}
place_cursor_in_definition() {
  press -M ctrl -k f -m ctrl
  type_text ':: work'
  press -k Return
  press -k Escape
  press -k Left
  for _ in {1..5}; do
    press -k Right
  done
}
place_cursor_in_missing_id() {
  press -M ctrl -k f -m ctrl
  type_text '>'
  press -k Return
  sleep 1
  press -k Escape
  sleep 1
  press -k Left
  press -M shift -k Right -m shift
  sleep 1
}
wait_for_file() {
  local path="$1"
  local waited=0
  while [[ ! -e "$path" ]]; do
    (( waited += 1 ))
    (( waited >= timeout_seconds )) && return 1
    sleep 1
  done
}
wait_for_log() {
  local pattern="$1"
  local waited=0
  while ! [[ -f "$zed_log" ]] || ! rg -q -- "$pattern" "$zed_log"; do
    (( waited += 1 ))
    (( waited >= timeout_seconds )) && return 1
    sleep 1
  done
}
wait_for_probe_process() {
  local needle="$1"
  local waited=0
  while ! probe_processes | rg -F -- "$needle" >/dev/null; do
    (( waited += 1 ))
    (( waited >= timeout_seconds )) && return 1
    sleep 1
  done
}
capture() {
  local name="$1"
  "${host_env[@]}" grim "$probe_dir/$name.png"
  [[ -s "$probe_dir/$name.png" ]] || {
    echo "grim produced an empty screenshot: $name" >&2
    return 1
  }
  capture_hash="$(sha256sum "$probe_dir/$name.png" | awk '{print $1}')"
  printf '%s_sha256=%s\n' "$name" "$capture_hash"
}
start_host() {
  local stage="$1"
  local stage_log="$probe_dir/$stage.cage.log"
  echo "starting private Cage/Zed session: $stage"
  # dbus-run-session owns the private bus and Cage owns the private Wayland
  # socket. The direct editor binary avoids zeditor's single-instance client.
  "${host_env[@]}" dbus-run-session -- cage -d -- "$zed_bin" \
    --user-data-dir "$user_data" "$project_dir" \
    > "$stage_log" 2>&1 &
  cage_pid=$!
  wait_for_file "$runtime_dir/$wayland_display" || {
    echo "private Cage Wayland socket did not appear" >&2
    return 1
  }
  wait_for_log 'Rendered first frame' || {
    echo "Zed did not render a first frame; see $stage_log and $zed_log" >&2
    return 1
  }
  open_file "$project_dir/fixture.recite"
  capture "$stage-start"
}
stop_host() {
  # Ctrl-Q, the command palette's `zed: quit`, and Alt-F4 are real Zed/window
  # keyboard actions delivered through Wayland. Alt-F4 is the normal
  # window-close fallback on hosts where the application action is not bound.
  # Any process recovery is evidence failure, even if bounded cleanup
  # succeeds.
  press -M ctrl -k q -m ctrl || true
  press -M ctrl -M shift -k p -m shift -m ctrl || true
  sleep 1
  type_text 'zed: quit' || true
  press -k Return || true
  press -M alt -k F4 -m alt || true
  local waited=0
  while [[ -n "$(probe_processes)" ]]; do
    (( waited += 1 ))
    (( waited >= 10 )) && break
    sleep 1
  done
  if [[ -n "$(probe_processes)" ]]; then
    echo "ERROR: Zed did not stop after keyboard shutdown; recovering only private probe processes" >&2
    terminate_probe_processes || true
    if [[ -n "$cage_pid" ]]; then
      wait "$cage_pid" 2>/dev/null || true
    fi
    cage_pid=""
    return 1
  fi
  wait "$cage_pid" 2>/dev/null || true
  cage_pid=""
}

echo "== private compositor and checked-in extension install =="
start_host install
press -M ctrl -M shift -k p -m shift -m ctrl
sleep 1
type_text 'zed: install dev extension'
press -k Return
sleep 1
press -M ctrl -k a -m ctrl
type_text "$extension_copy"
capture install-path
press -k Return
wait_for_log "finished compiling extension $extension_copy"
[[ -f "$extension_copy/extension.wasm" ]] || {
  echo "Zed reported extension compilation but did not write extension.wasm" >&2
  exit 1
}
index_wait=0
until python3 - "$user_data/extensions/index.json" "$grammar_revision" <<'PY'
import json
import pathlib
import sys

index = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
entry = index.get("extensions", {}).get("recite")
if not entry or entry.get("dev") is not True:
    raise SystemExit("installed extension index does not contain recite with dev=true")
manifest = entry.get("manifest", {})
if manifest.get("id") != "recite" or manifest.get("name") != "Recite":
    raise SystemExit("installed extension index has the wrong Recite identity")
grammar = manifest.get("grammars", {}).get("recite", {})
if grammar.get("rev") != sys.argv[2]:
    raise SystemExit(f"installed extension grammar revision drifted: {grammar!r}")
if "Recite" not in manifest.get("language_servers", {}).get("recite-lsp", {}).get("languages", []):
    raise SystemExit("installed extension index lacks Recite language-server registration")
PY
do
  (( index_wait += 1 ))
  (( index_wait >= timeout_seconds )) && {
    echo "Zed did not publish the development extension in its index" >&2
    exit 1
  }
  sleep 1
done
printf 'extension_wasm_sha256=%s\n' "$(sha256sum "$extension_copy/extension.wasm" | awk '{print $1}')"
printf 'installed_extension_index=recite(dev=true),grammar_rev=%s\n' "$grammar_revision"
capture install-complete
stop_host

echo "== reload installed extension, LSP diagnostics, and keyboard workflow =="
start_host authoring
# The initial path was passed to Zed, but explicitly reopening it through the
# documented file picker makes the keyboard boundary observable as part of the
# run.
open_file "$project_dir/fixture.recite"
sleep 3
capture authoring-file
wait_for_probe_process "$bin_dir/recite-lsp" || {
  echo "Zed did not leave recite-lsp running after opening the Recite fixture" >&2
  exit 1
}
ps -eo pid=,args= > "$probe_dir/processes-after-lsp.txt"
rg -F -- "$bin_dir/recite-lsp" "$probe_dir/processes-after-lsp.txt" >/dev/null || {
  echo "recite-lsp process disappeared before process capture" >&2
  exit 1
}
echo "recite_lsp_process=observed"

# Zed exposes the LSP result surfaces as UI actions rather than a stable
# machine-readable host API. Exercise each documented action through the real
# command palette and retain a rendered screenshot for the evidence record.
host_action() {
  local action="$1"
  local screenshot="$2"
  press -M ctrl -M shift -k p -m shift -m ctrl
  sleep 1
  type_text "$action"
  press -k Return
  sleep 2
  capture "$screenshot"
  press -k Escape
  sleep 1
}
hover_action() {
  # The official Linux keymap binds Ctrl-K Ctrl-I to editor::Hover. Using the
  # direct binding avoids making the hover assertion depend on the command
  # palette retaining editor focus after completion UI closes.
  press -M ctrl -k k -m ctrl
  press -M ctrl -k i -m ctrl
  sleep 2
  capture lsp-hover
  press -k Escape
  sleep 1
}
open_file "$project_dir/core.recite"
place_cursor_in_divert
# Collapse the search selection and place the cursor inside `work`, where the
# Recite server's source-span boundary accepts navigation/rename requests.
sleep 2
host_action 'editor: show completions' lsp-completion
place_cursor_in_divert
hover_action
place_cursor_in_divert
host_action 'editor: go to definition' lsp-definition
place_cursor_in_definition
host_action 'editor: find all references' lsp-references
place_cursor_in_definition
host_action 'editor: rename' lsp-rename
open_file "$project_dir/code-action.recite"
place_cursor_in_missing_id
host_action 'editor: toggle code actions' lsp-code-actions
echo "lsp_ui_actions=diagnostics,completion,hover,definition,references,rename,code-actions dispatched"
host_action 'diagnostics: deploy' diagnostics-panel

# Return to the malformed canonical fixture for the task-failure and watch
# lifecycle checks below.
open_file "$project_dir/fixture.recite"

# Navigate both directions through the two canonical parser diagnostics. The
# screenshot hashes are a rendered keyboard-boundary assertion only; payload
# correctness is asserted from the transport log below.
capture diagnostic-navigation-start
diagnostic_start_hash="$capture_hash"
press -k F8
sleep 2
capture diagnostic-next
diagnostic_next_hash="$capture_hash"
press -M shift -k F8 -m shift
sleep 2
capture diagnostic-previous
diagnostic_previous_hash="$capture_hash"
if [[ "$diagnostic_start_hash" == "$diagnostic_next_hash" || \
  "$diagnostic_next_hash" == "$diagnostic_previous_hash" ]]; then
  echo "diagnostic navigation did not change the rendered host boundary" >&2
  exit 1
fi
echo "diagnostic_navigation=next_and_previous_keyboard_actions_observed"

# Spawn the language-provided validation task through Zed's command palette.
# Its PATH wrapper records the actual task argv and status while the child is
# still the exact recite binary built above.
press -M ctrl -M shift -k p -m shift -m ctrl
sleep 1
type_text 'task: spawn'
press -k Return
sleep 1
type_text 'Recite: validate current file'
press -k Return
sleep 6
capture task-validate
wait_for_file "$task_log"
rg -F -- 'validate --output-format structured' "$task_log" >/dev/null || {
  echo "Zed did not invoke the structured Recite validation task" >&2
  exit 1
}
rg -F -- 'status=1' "$task_log" >/dev/null || {
  echo "invalid fixture validation did not report the expected failure status" >&2
  exit 1
}
echo "task_validate=structured argv observed, status=1 observed"

# Spawn watch and stop it with Ctrl-C in the task terminal. Zed's documented
# task surface has no machine-readable cancellation API, so process absence
# after this genuine key event is the bounded lifecycle assertion.
press -M ctrl -M shift -k p -m shift -m ctrl
sleep 1
type_text 'task: spawn'
press -k Return
sleep 1
type_text 'Recite: watch worktree'
press -k Return
sleep 8
capture task-watch
rg -F -- 'watch --output-format structured' "$task_log" >/dev/null || {
  echo "Zed did not invoke the structured Recite watch task" >&2
  exit 1
}
press -M ctrl -k c -m ctrl
sleep 3
ps -eo pid=,args= > "$probe_dir/processes-after-watch-stop.txt"
if probe_processes | rg -F -- "$cli_probe_bin" | rg -e ' watch( |$)' >/dev/null; then
  echo "recite watch process remained after the host keyboard stop boundary" >&2
  exit 1
fi
echo "task_watch=structured argv observed, Ctrl-C termination observed"
python3 "$assert_lsp_log" "$lsp_log" "$project_dir"
stop_host

echo "== exact private-host process cleanup =="
if [[ -n "$(probe_processes)" ]]; then
  echo "private Zed probe process leaked its temporary path" >&2
  probe_processes >&2 || true
  exit 1
fi
echo "shutdown=Ctrl-Q+zed:quit+Alt-F4 requested; no private probe process remained"
echo "PASS: installed Zed Linux source extension, activation/rendering, LSP process, diagnostic fixture, LSP UI actions, static task failure, watch keyboard termination, and private shutdown exercised; code-action edit remains unsupported"
echo "RESIDUAL: Zed task terminals do not expose structured records as editor diagnostics; no gallery publication, macOS/Windows host, screen-reader/high-contrast, or native task cancellation API is claimed"
