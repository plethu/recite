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
cat >"$check_dir/global.json" <<'JSON'
{"sdk":{"version":"8.0.421"}}
JSON
cat >"$check_dir/NuGet.Config" <<'XML'
<?xml version="1.0" encoding="utf-8"?><configuration><packageSources><clear /></packageSources></configuration>
XML
cat >"$check_dir/Schema.csproj" <<XML
<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework><EnableDefaultCompileItems>false</EnableDefaultCompileItems><OutputType>Exe</OutputType><Nullable>disable</Nullable></PropertyGroup><ItemGroup>
<Compile Include="$repo_root/Packages/com.recite.dialogue/Editor/ReciteSchemaRegistration.cs" />
<Compile Include="$repo_root/Packages/com.recite.dialogue/Editor/ReciteSchemaToml.cs" />
<Compile Include="$repo_root/Packages/com.recite.dialogue/Tests~/Headless/UnitySchemaStubs.cs" />
<Compile Include="$repo_root/Packages/com.recite.dialogue/Tests~/Headless/ReciteUnitySchemaHeadless.cs" />
</ItemGroup></Project>
XML
(
  cd "$check_dir"
  DOTNET_CLI_HOME=/tmp/recite-dotnet-home DOTNET_CLI_TELEMETRY_OPTOUT=1 NUGET_PACKAGES=/tmp/recite-nuget dotnet run --project Schema.csproj -- schema.toml duplicate.toml
)
producer_id='0123456789abcdef0123456789abcdef'
if ! "$cli" export-schema --schema "$check_dir/schema.toml" --output "$check_dir/manifest.json" --producer-kind unity --producer-id "$producer_id" --output-format structured >"$check_dir/result.ndjson"; then
  cat "$check_dir/result.ndjson" >&2
  cat "$check_dir/schema.toml" >&2
  exit 1
fi
python3 - "$check_dir" <<'PY'
import json,sys,pathlib
root=pathlib.Path(sys.argv[1]); manifest=json.loads((root/'manifest.json').read_text())
assert manifest['conditions']['has_key']['params'][0]['type']=='registry:item'
assert manifest['effects']['grant_item']['modes']==['blocking']
assert manifest['speakers']['ada']['display_name']=='Ada \\"Élan\\"'
assert manifest['producer']['kind']=='unity'
assert manifest['producer']['id']=='0123456789abcdef0123456789abcdef'
records=[json.loads(s) for s in (root/'result.ndjson').read_text().splitlines()]
assert records[-1]['status']=='success'
PY
cp "$check_dir/manifest.json" "$check_dir/before.json"
if "$cli" export-schema --schema "$check_dir/duplicate.toml" --output "$check_dir/manifest.json" --producer-kind unity --producer-id "$producer_id" --output-format structured >"$check_dir/duplicate.ndjson"; then
  echo 'duplicate schema declaration unexpectedly exported' >&2
  exit 1
fi
cmp "$check_dir/before.json" "$check_dir/manifest.json"
python3 - "$check_dir" <<'PY'
import json,sys,pathlib
root=pathlib.Path(sys.argv[1]); records=[json.loads(s) for s in (root/'duplicate.ndjson').read_text().splitlines()]
assert records[-1]['status']=='content_diagnostics'
assert records[-1]['data']['diagnostics']
PY
echo 'Unity schema declaration export passed.'
