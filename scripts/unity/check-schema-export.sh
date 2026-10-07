#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-$(git rev-parse --show-toplevel)}"
cli="${RECITE_UNITY_CLI:-${CARGO_TARGET_DIR:-$repo_root/target}/debug/recite}"
[[ -x "$cli" ]] || {
  echo "missing recite CLI: $cli" >&2
  exit 1
}
check_dir="$(mktemp -d "${TMPDIR:-/tmp}/recite-unity-schema.XXXXXX")"
trap 'rm -rf "$check_dir"' EXIT
export TMPDIR="$check_dir"
headless_dir="$repo_root/Packages/com.recite.dialogue/Tests~/Headless"
export DOTNET_CLI_HOME="${DOTNET_CLI_HOME:-$check_dir/dotnet-home}"
export NUGET_PACKAGES="${NUGET_PACKAGES:-$check_dir/nuget}"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
dotnet build "$headless_dir/Schema.csproj" --artifacts-path "$check_dir/artifacts" \
  --configfile "$headless_dir/NuGet.Config" --disable-build-servers --nologo -v:minimal
(
  cd "$check_dir"
  dotnet "$check_dir/artifacts/bin/Schema/debug/Schema.dll" schema.toml duplicate.toml
)
producer_id='0123456789abcdef0123456789abcdef'
if ! "$cli" export-schema --schema "$check_dir/schema.toml" --output "$check_dir/manifest.json" --producer-kind unity --producer-id "$producer_id" --output-format structured >"$check_dir/result.ndjson"; then
  cat "$check_dir/result.ndjson" >&2
  cat "$check_dir/schema.toml" >&2
  exit 1
fi
jq -se --arg name 'Ada \"Élan\"' --arg producer "$producer_id" '
  length == 1 and (.[0] |
  .conditions.has_key.params[0].type == "registry:item" and
  .effects.grant_item.modes == ["blocking"] and
  .speakers.ada.display_name == $name and
  .producer.kind == "unity" and .producer.id == $producer)
' "$check_dir/manifest.json" >/dev/null
jq -se 'last.status == "success"' "$check_dir/result.ndjson" >/dev/null
cp "$check_dir/manifest.json" "$check_dir/before.json"
if "$cli" export-schema --schema "$check_dir/duplicate.toml" --output "$check_dir/manifest.json" --producer-kind unity --producer-id "$producer_id" --output-format structured >"$check_dir/duplicate.ndjson"; then
  echo 'duplicate schema declaration unexpectedly exported' >&2
  exit 1
fi
cmp "$check_dir/before.json" "$check_dir/manifest.json"
jq -se 'last.status == "content_diagnostics" and (last.data.diagnostics | length > 0)' \
  "$check_dir/duplicate.ndjson" >/dev/null
echo 'Unity schema declaration export passed.'
