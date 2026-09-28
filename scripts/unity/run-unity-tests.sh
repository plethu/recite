#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-$(git rev-parse --show-toplevel)}"
editor="${UNITY_EDITOR:-}"
if [[ -z "$editor" || ! -x "$editor" ]]; then
  echo 'UNITY_EDITOR must name an installed Unity Editor executable.' >&2
  exit 2
fi
bundle="${RECITE_UNITY_UPM_BUNDLE:-}"
if [[ -z "$bundle" ]]; then
  bundle="$(CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target/companions-unity}" "$repo_root/scripts/unity/build-upm.sh" "$repo_root" | tail -1)"
fi
[[ -f "$bundle" ]] || { echo "missing UPM bundle: $bundle" >&2; exit 2; }
scratch_root="${TMPDIR:-/tmp}"
results="${RECITE_UNITY_TEST_RESULTS:-$(mktemp -d "$scratch_root/recite-unity-results.XXXXXX")}"
mkdir -p "$results"
consumer="$(mktemp -d "$scratch_root/recite-unity-consumer.XXXXXX")"
trap 'rm -rf "$consumer"' EXIT
project="$consumer/project"
cp -a "$repo_root/tests/unity-project" "$project"
mkdir -p "$project/Assets"
mkdir -p "$consumer/extracted"
tar -xzf "$bundle" -C "$consumer/extracted"
mv "$consumer/extracted/package" "$project/Packages/com.recite.dialogue"
python3 - "$project/Packages/manifest.json" <<'PY'
import json,sys
path=sys.argv[1]
with open(path) as source: manifest=json.load(source)
manifest['dependencies']['com.recite.dialogue']='file:com.recite.dialogue'
with open(path,'w') as output: json.dump(manifest,output,indent=2)
PY
"$editor" -batchmode -nographics -quit -projectPath "$project" \
  -executeMethod Recite.Unity.Editor.Tests.ReciteTestSceneBootstrap.Create \
  -logFile "$results/fixture.log"
check_results() {
  python3 - "$1" "$2" <<'PY'
import sys,xml.etree.ElementTree as ET
root=ET.parse(sys.argv[1]).getroot()
total=int(root.get('total',root.get('tests','0')))
passed=int(root.get('passed','0'))
failed=int(root.get('failed',root.get('failures','0')))
inconclusive=int(root.get('inconclusive','0'))
skipped=int(root.get('skipped','0'))
result=root.get('result','')
if total<=0 or passed!=total or failed or inconclusive or skipped or result!='Passed':
    raise SystemExit(f'{sys.argv[2]} Unity tests did not pass: total={total} passed={passed} failed={failed} inconclusive={inconclusive} skipped={skipped} result={result}')
if sys.argv[2].endswith('Linux player') and not any(
    case.get('fullname')=='Recite.Unity.Tests.RecitePlayModeTests.ImportedResourceRunsInPlayer'
    and case.get('result')=='Passed' for case in root.iter('test-case')
):
    raise SystemExit(f'{sys.argv[2]} did not run the imported-resource player test')
print(f'{sys.argv[2]} Unity tests passed: {passed}/{total}')
PY
}
for platform in editmode playmode; do
  rm -f "$results/$platform.xml"
  "$editor" -batchmode -nographics -projectPath "$project" -runTests \
    -testPlatform "$platform" -testResults "$results/$platform.xml" \
    -logFile "$results/$platform.log"
  check_results "$results/$platform.xml" "$platform"
done
case "${RECITE_UNITY_PLAYER_MODE:-}" in
  '') ;;
  mono|il2cpp|coreclr)
    mode="$RECITE_UNITY_PLAYER_MODE"
    case "$mode" in
      mono) backend=Mono2x ;;
      il2cpp) backend=IL2CPP ;;
      coreclr)
        backend=CoreCLR
        "$editor" -batchmode -nographics -quit -projectPath "$project" \
          -executeMethod RecitePlayerBackend.ConfigureCoreClr \
          -logFile "$results/coreclr-backend.log"
        ;;
    esac
    settings="$results/$mode-settings.json"
    printf '{"scriptingBackend":"%s"}\n' "$backend" > "$settings"
    player="$results/$mode-player/PlayerWithTests"
    rm -rf "$results/$mode-player" "$results/$mode.xml"
    "$editor" -batchmode -nographics -projectPath "$project" -runTests \
      -testPlatform StandaloneLinux64 -buildTarget StandaloneLinux64 \
      -testSettingsFile "$settings" -buildPlayerPath "$results/$mode-player" \
      -recitePlayerBackend "$backend" -recitePlayerPath "$player" \
      -doNotReportTestResultsBackToEditor -logFile "$results/$mode-build.log"
    if ! rg -q 'Build Finished, Result: Success\.' "$results/$mode-build.log"; then
      echo "Unity did not report a successful $mode player build; see $results/$mode-build.log" >&2
      exit 1
    fi
    [[ -x "$player" ]] || { echo "missing Linux test player: $player" >&2; exit 1; }
    case "$mode" in
      mono)
        [[ -f "$results/$mode-player/PlayerWithTests_Data/MonoBleedingEdge/x86_64/libmonobdwgc-2.0.so" ]] || {
          echo "missing Mono runtime in $mode player" >&2; exit 1;
        }
        ;;
      il2cpp)
        [[ -f "$results/$mode-player/GameAssembly.so" ]] || {
          echo "missing IL2CPP runtime in $mode player" >&2; exit 1;
        }
        ;;
      coreclr)
        [[ -f "$results/$mode-player/CoreCLR/native/libcoreclr.so" ]] || {
          echo "missing CoreCLR runtime in $mode player" >&2; exit 1;
        }
        ;;
    esac
    timeout --signal=TERM --kill-after=5s 90s "$player" -batchmode -nographics \
      -reciteResultPath "$results/$mode.xml" -logFile "$results/$mode-player.log"
    check_results "$results/$mode.xml" "$mode Linux player"
    ;;
  *) echo 'RECITE_UNITY_PLAYER_MODE must be mono, il2cpp, or coreclr' >&2; exit 2 ;;
esac
printf 'Unity clean-consumer test results: %s\n' "$results"
