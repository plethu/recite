#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-}"
if [[ -n "$repo_root" ]]; then
  repo_root="$(git -C "$repo_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

package_dir="$repo_root/Packages/com.recite.dialogue"
runtime_dir="$package_dir/Runtime"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target/companions-unity}"
export CARGO_TARGET_DIR="$target_dir"
bridge="$runtime_dir/Native/ReciteNativeBridge.cs"
header="$repo_root/include/recite.h"

failures=0
fail() {
  echo "$*" >&2
  failures=$((failures + 1))
}

[[ -f "$package_dir/package.json" ]] || fail "missing Unity package manifest"
[[ -f "$runtime_dir/Recite.Dialogue.asmdef" ]] || fail "missing runtime asmdef"
[[ -f "$bridge" ]] || fail "missing native bridge"

if [[ -f "$bridge" && -f "$header" ]]; then
  mapfile -t header_symbols < <(grep -E '^ReciteStatus recite_|^void recite_|^const char \*recite_' "$header" | grep -Eo 'recite_[a-z_]+' | sort -u)
  for symbol in "${header_symbols[@]}"; do
    if ! grep -q "EntryPoint = \"$symbol\"" "$bridge"; then
      fail "native bridge does not declare P/Invoke symbol $symbol"
    fi
  done

  for version in MAJOR MINOR PATCH; do
    header_value="$(grep -E "^#define RECITE_FFI_VERSION_${version} " "$header" | awk '{print $3}')"
    bridge_name="$(tr '[:upper:]' '[:lower:]' <<<"$version")"
    bridge_name="$(tr '[:lower:]' '[:upper:]' <<<"${bridge_name:0:1}")${bridge_name:1}"
    if ! grep -q "Abi${bridge_name} = ${header_value};" "$bridge"; then
      fail "native bridge ABI ${version} constant does not match include/recite.h"
    fi
  done

  required_bridge_patterns=(
    "internal struct ReciteBuffer"
    "internal IntPtr Data;"
    "internal UIntPtr Len;"
    "internal struct ReciteConditionQuery"
    "internal IntPtr FunctionName;"
    "internal IntPtr ArgsMsgpack;"
    "internal UIntPtr ArgsLen;"
    "internal struct ReciteConditionResult"
    "internal byte Ok;"
    "internal IntPtr ValueMsgpack;"
    "internal UIntPtr ValueLen;"
    "internal IntPtr ErrorMessage;"
    "UnmanagedFunctionPointer(CallingConvention.Cdecl)"
    "delegate ReciteConditionResult ReciteConditionFn(IntPtr query, IntPtr userdata);"
    "delegate ReciteLocaleResult ReciteLocaleFn(IntPtr query, IntPtr userdata);"
    "EntryPoint = \"recite_locale_evaluate_plural_rule\""
    "EntryPoint = \"recite_locale_validate_translation_placeholders\""
    "AssetLoad(byte[] bytes, UIntPtr len, out ulong assetHandle)"
    "AssetFree(ulong assetHandle)"
    "SessionCreate(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle)"
    "SessionBegin(ulong sessionHandle, out ReciteBuffer batch)"
    "SessionStart(ulong assetHandle, byte[] startBlock, byte[] locale, out ulong sessionHandle, out ReciteBuffer batch)"
    "SessionRegisterCondition(ulong sessionHandle, byte[] name, ReciteConditionFn handler, IntPtr userdata)"
    "SessionChoose(ulong sessionHandle, byte[] choiceId, out ReciteBuffer batch)"
    "SessionAcknowledgeEffect(ulong sessionHandle, byte[] effectRequestId, byte ackCompleted, byte[] failureReason, out ReciteBuffer batch)"
    "SessionSnapshot(ulong sessionHandle, out ReciteBuffer snapshot)"
    "SessionRestore(ulong assetHandle, byte[] snapshotBytes, UIntPtr snapshotLen, out ulong sessionHandle, out ReciteBuffer batch)"
    "SessionFree(ulong sessionHandle)"
    "BufferFree(ref ReciteBuffer buffer)"
  )
  for pattern in "${required_bridge_patterns[@]}"; do
    if ! grep -qF "$pattern" "$bridge"; then
      fail "native bridge is missing expected ABI shape: $pattern"
    fi
  done

  status_file="$runtime_dir/ReciteAdapterError.cs"
  while IFS= read -r line; do
    symbol="$(sed -E 's/[[:space:]]*(RECITE_STATUS_[A-Z_]+).*/\1/' <<<"$line")"
    value="$(sed -E 's/.*=[[:space:]]*(-?[0-9]+),?/\1/' <<<"$line")"
    pascal="$(sed -E 's/^RECITE_STATUS_//' <<<"$symbol" | awk -F_ '{ out=""; for (i=1;i<=NF;i++) out=out toupper(substr($i,1,1)) tolower(substr($i,2)); print out }')"
    case "$pascal" in
      Ok) ;;
      Assetloadordecode) pascal="AssetLoadOrDecode" ;;
      Staleorincompatible) pascal="StaleOrIncompatible" ;;
      Schemamismatch) pascal="SchemaMismatch" ;;
      Noactivesession) pascal="NoActiveSession" ;;
      Sessionalreadyactive) pascal="SessionAlreadyActive" ;;
      Unknownstartblock) pascal="UnknownStartBlock" ;;
      Invalidchoice) pascal="InvalidChoice" ;;
      Unavailablechoice) pascal="UnavailableChoice" ;;
      Stalechoice) pascal="StaleChoice" ;;
      Missingconditionhandler) pascal="MissingConditionHandler" ;;
      Conditionevaluation) pascal="ConditionEvaluation" ;;
      Invalidconditionresult) pascal="InvalidConditionResult" ;;
      Effectacknowledgement) pascal="EffectAcknowledgement" ;;
      Rejectedrefresh) pascal="RejectedRefresh" ;;
      Saveloadincompatibility) pascal="SaveLoadIncompatibility" ;;
      Localisation) ;;
      Missingprojectionhandler) pascal="MissingProjectionHandler" ;;
      Projectionevaluation) pascal="ProjectionEvaluation" ;;
      Invalidprojectionresult) pascal="InvalidProjectionResult" ;;
      Invalidhandle) pascal="InvalidHandle" ;;
      Dialoguefault) pascal="DialogueFault" ;;
    esac
    if ! grep -q "^[[:space:]]*${pascal} = ${value}" "$status_file"; then
      fail "ReciteStatus.${pascal} does not match $symbol = $value"
    fi
  done < <(grep -E '^[[:space:]]*RECITE_STATUS_[A-Z_]+ = -?[0-9]+,' "$header")
fi

file="$runtime_dir/ConditionCallbacks.cs"
for pattern in "GCHandleType.Normal" "MonoPInvokeCallback"; do
  if [[ ! -f "$file" ]] || ! grep -qF "$pattern" "$file"; then
    fail "$file is missing IL2CPP-safe callback ownership: $pattern"
  fi
done

while IFS= read -r file; do
  if grep -q 'UnityEditor' "$file"; then
    fail "$file imports UnityEditor in runtime code"
  fi
done < <(find "$runtime_dir" -name '*.cs' -type f)

sample_dir="$package_dir/Samples~/BasicDialogue"
[[ -f "$sample_dir/BasicDialogue.unity" ]] || fail "Unity package is missing the BasicDialogue sample scene"
[[ -f "$sample_dir/BasicDialogueDriver.cs" ]] || fail "Unity package is missing the BasicDialogue sample driver"
[[ -f "$sample_dir/Dialogue/basic.recite" ]] || fail "Unity package is missing sample source dialogue"
[[ -f "$sample_dir/Dialogue/basic.recitec" ]] || fail "Unity package is missing compiled sample dialogue"
[[ -f "$sample_dir/Dialogue/basic.recitec.meta" ]] || fail "Unity package is missing compiled sample importer metadata"
if [[ -f "$sample_dir/BasicDialogue.unity" ]]; then
  grep -q 'compiledAsset:' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not wire a compiled Recite asset"
  grep -q 'runner:' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not wire BasicDialogueDriver to ReciteDialogueRunner"
  grep -q 'm_MethodName: OnReciteOutput' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not route Recite output to BasicDialogueDriver"
  grep -q 'm_MethodName: OnReciteError' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not route Recite errors to BasicDialogueDriver"
fi

if command -v dotnet >/dev/null 2>&1; then
  if ! cargo build -p recite-ffi -p recite-cli --quiet; then
    fail "recite-ffi native library build failed"
  fi
  tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/recite-unity-check.XXXXXX")"
  trap 'rm -rf "$tmpdir"' EXIT
  export TMPDIR="$tmpdir"
  export DOTNET_CLI_HOME="$tmpdir/dotnet-home" NUGET_PACKAGES="$tmpdir/nuget"
  export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
  headless_dir="$package_dir/Tests~/Headless"
  assembly="$tmpdir/artifacts/bin/HeadlessRuntime/debug/HeadlessRuntime.dll"
  export LD_LIBRARY_PATH="$target_dir/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

  cp "$sample_dir/Dialogue/basic.recite" "$tmpdir/basic.recite"
  if "$target_dir/debug/recite" compile -o "$tmpdir/revision.recitec" "$tmpdir/basic.recite"; then
    cp "$tmpdir/revision.recitec" "$tmpdir/old.recitec"
    sed -i 's/The relay wakes./The relay stirs./' "$tmpdir/basic.recite"
    "$target_dir/debug/recite" compile -o "$tmpdir/revision.recitec" "$tmpdir/basic.recite" || fail "Unity revision fixture failed to compile"
  else
    fail "Unity baseline revision fixture failed to compile"
  fi

  "$target_dir/debug/recite" compile -o "$tmpdir/plural.recitec" \
    "$repo_root/fixtures/recite/valid/adapter_conformance/plural_runtime.recite" || fail "Unity plural fixture failed to compile"
  "$target_dir/debug/recite" compile -o "$tmpdir/conformance.recitec" \
    "$repo_root/fixtures/recite/valid/adapter_conformance/runtime_surface.recite" || fail "Unity conformance fixture failed to compile"
  "$target_dir/debug/recite" compile --schema "$repo_root/fixtures/schema/valid/generated_manifest.json" \
    -o "$tmpdir/reasons.recitec" "$repo_root/fixtures/recite/valid/adapter_conformance/availability_reasons.recite" \
    || fail "Unity availability fixture failed to compile"
  export RECITE_UNITY_REASONS_ASSET="$tmpdir/reasons.recitec"

  cp "$repo_root/fixtures/recite/valid/adapter_conformance/restore_schema_prompt.recite" "$tmpdir/schema-restore.recite"
  for schema in a b; do
    cat >"$tmpdir/schema-$schema.toml" <<TOML
schema_version = 1
[producer]
id = "unity-schema-restore"
[speakers.test]
display_name = "$schema"
TOML
    "$target_dir/debug/recite" export-schema --schema "$tmpdir/schema-$schema.toml" \
      --output "$tmpdir/schema-$schema.json" || fail "Unity schema $schema fixture failed to export"
    "$target_dir/debug/recite" compile --schema "$tmpdir/schema-$schema.json" \
      -o "$tmpdir/schema-shared.recitec" "$tmpdir/schema-restore.recite" || fail "Unity schema $schema fixture failed to compile"
    cp "$tmpdir/schema-shared.recitec" "$tmpdir/schema-$schema.recitec"
  done

  export RECITE_UNITY_SAMPLE_ASSET="$sample_dir/Dialogue/basic.recitec"
  export RECITE_UNITY_REVISION_OLD="$tmpdir/old.recitec" RECITE_UNITY_REVISION_NEW="$tmpdir/revision.recitec"
  export RECITE_UNITY_PLURAL_ASSET="$tmpdir/plural.recitec" RECITE_UNITY_CONFORMANCE_ASSET="$tmpdir/conformance.recitec"
  export RECITE_UNITY_SCHEMA_A="$tmpdir/schema-a.recitec" RECITE_UNITY_SCHEMA_B="$tmpdir/schema-b.recitec"
  if ! dotnet build "$headless_dir/HeadlessRuntime.csproj" --artifacts-path "$tmpdir/artifacts" \
    --configfile "$headless_dir/NuGet.Config" --disable-build-servers --nologo -v:minimal >"$tmpdir/build.log" 2>&1; then
    cat "$tmpdir/build.log" >&2
    fail "Unity runtime subset dotnet build failed"
  elif ! dotnet "$assembly" >"$tmpdir/test.log" 2>&1; then
    cat "$tmpdir/test.log" >&2
    fail "Unity headless package test failed"
  elif [[ "${RECITE_UNITY_PERF:-}" == 1 ]]; then
    dotnet "$assembly" --perf || fail "Unity managed performance probe failed"
  fi
  if ! RECITE_UNITY_CLI="$target_dir/debug/recite" "$repo_root/scripts/unity/check-schema-export.sh" "$repo_root"; then
    fail "Unity schema export check failed"
  fi
else
  fail "dotnet is required for the Unity runtime subset build"
fi

if ((failures > 0)); then
  echo "Found ${failures} Unity adapter check failure(s)." >&2
  exit 1
fi

if [[ -n "${UNITY_EDITOR:-}" ]]; then
  "$repo_root/scripts/unity/run-unity-tests.sh" "$repo_root"
else
  echo "Unity Editor unavailable; EditMode and PlayMode suites were not run."
fi

echo "Unity adapter package check passed."
