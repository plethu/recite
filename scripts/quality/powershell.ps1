param([ValidateSet('setup', 'format', 'lint')][string]$Mode)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path (Split-Path $PSScriptRoot)
$moduleRoot = Join-Path $repoRoot 'target/quality/powershell'
$modulePath = Join-Path $moduleRoot "PSScriptAnalyzer/$env:RECITE_PSSA_VERSION/PSScriptAnalyzer.psd1"

if ($Mode -eq 'setup') {
    if (!(Test-Path $modulePath)) {
        Save-Module PSScriptAnalyzer -RequiredVersion $env:RECITE_PSSA_VERSION -Path $moduleRoot -Repository PSGallery -Force
    }
    exit
}
Import-Module $modulePath
$settings = Join-Path $PSScriptRoot 'PSScriptAnalyzerSettings.psd1'
if ($Mode -eq 'format') {
    [Console]::Out.Write((Invoke-Formatter -ScriptDefinition ([Console]::In.ReadToEnd()) -Settings $settings))
    exit
}
$issues = Get-ChildItem "$repoRoot/scripts" -Recurse -Include '*.ps1', '*.psd1' |
    Invoke-ScriptAnalyzer -Settings $settings -Severity Error, Warning
$issues | Format-Table -AutoSize
if ($issues) { exit 1 }
