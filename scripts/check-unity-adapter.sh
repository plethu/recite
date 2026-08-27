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
bridge="$runtime_dir/Native/ReciteNativeBridge.cs"
header="$repo_root/include/recite.h"
headless_test="$package_dir/Tests~/Headless/ReciteUnityHeadless.cs"

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
[[ -f "$sample_dir/Dialogue/basic.recitec.meta" ]] || fail "Unity package is missing compiled sample TextAsset metadata"
if [[ -f "$sample_dir/BasicDialogue.unity" ]]; then
  grep -q 'compiledAsset:' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not wire a compiled Recite TextAsset"
  grep -q 'runner:' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not wire BasicDialogueDriver to ReciteDialogueRunner"
  grep -q 'm_MethodName: OnReciteOutput' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not route Recite output to BasicDialogueDriver"
  grep -q 'm_MethodName: OnReciteError' "$sample_dir/BasicDialogue.unity" || fail "sample scene does not route Recite errors to BasicDialogueDriver"
fi

if command -v dotnet >/dev/null 2>&1; then
  tmpdir="$(mktemp -d /tmp/recite-unity-check.XXXXXX)"
  {
    printf '%s\n' '<Project Sdk="Microsoft.NET.Sdk">'
    printf '%s\n' '  <PropertyGroup>'
    printf '%s\n' '    <TargetFramework>net8.0</TargetFramework>'
    printf '%s\n' '    <Nullable>disable</Nullable>'
    printf '%s\n' '    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>'
    printf '%s\n' '    <OutputType>Exe</OutputType>'
    printf '%s\n' '  </PropertyGroup>'
    printf '%s\n' '  <ItemGroup>'
    find "$runtime_dir" -path "$runtime_dir/GameObjects" -prune -o -name '*.cs' -type f -print | sort | while IFS= read -r file; do
      printf '    <Compile Include="%s" />\n' "$file"
    done
    printf '    <Compile Include="%s" />\n' "$headless_test"
    printf '%s\n' '  </ItemGroup>'
    printf '%s\n' '</Project>'
  } > "$tmpdir/UnityRuntimeSubset.csproj"

  if [[ ! -f "$headless_test" ]]; then
    fail "missing Unity headless package test"
  elif ! DOTNET_CLI_HOME=/tmp/recite-dotnet-home NUGET_PACKAGES=/tmp/recite-nuget DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 dotnet build "$tmpdir/UnityRuntimeSubset.csproj" --nologo -v:minimal >/tmp/recite-unity-dotnet-build.log 2>&1; then
    cat /tmp/recite-unity-dotnet-build.log >&2
    fail "Unity runtime subset dotnet build failed"
  elif ! DOTNET_CLI_HOME=/tmp/recite-dotnet-home NUGET_PACKAGES=/tmp/recite-nuget DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 dotnet run --project "$tmpdir/UnityRuntimeSubset.csproj" --no-build --no-restore >/tmp/recite-unity-headless-test.log 2>&1; then
    cat /tmp/recite-unity-headless-test.log >&2
    fail "Unity headless package test failed"
  fi
else
  fail "dotnet is required for the Unity runtime subset build"
fi

if (( failures > 0 )); then
  echo "Found ${failures} Unity adapter check failure(s)." >&2
  exit 1
fi

echo "Unity adapter package check passed."
