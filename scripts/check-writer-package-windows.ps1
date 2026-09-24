param([Parameter(Mandatory = $true)][string]$ArtifactDirectory)

$ErrorActionPreference = 'Stop'
$installer = @(Get-ChildItem $ArtifactDirectory -Filter '*.exe' -Recurse)
if ($installer.Count -ne 1) { throw 'Expected one NSIS installer' }
$installRoot = Join-Path $env:RUNNER_TEMP 'recite-writer-install-smoke'
$protocol = 'HKCU:\Software\Classes\recite'
$createdSentinel = !(Test-Path $protocol)

if ($createdSentinel) {
    New-Item $protocol -Force | Out-Null
    Set-Item $protocol -Value 'Recite package smoke sentinel'
    New-ItemProperty $protocol -Name 'URL Protocol' -Value '' -PropertyType String | Out-Null
    $commandKey = Join-Path $protocol 'shell\open\command'
    New-Item $commandKey -Force | Out-Null
    Set-Item $commandKey -Value '"C:\recite-smoke-other-app.exe" "%1"'
}

function Get-ProtocolSnapshot {
    $snapshotFile = Join-Path $env:RUNNER_TEMP ([Guid]::NewGuid().ToString() + '.reg')
    try {
        & reg.exe export 'HKCU\Software\Classes\recite' $snapshotFile /y | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Could not export protocol registry key' }
        return Get-Content $snapshotFile -Raw
    }
    finally {
        Remove-Item $snapshotFile -ErrorAction SilentlyContinue
    }
}

$before = $null
try {
    $before = Get-ProtocolSnapshot
    # NSIS /D must be the last argument. The runner account is disposable.
    $process = Start-Process $installer[0].FullName -ArgumentList @('/S', '/NS', "/D=$installRoot") -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "NSIS install failed: $($process.ExitCode)" }
    $binary = Join-Path $installRoot 'recite-writer.exe'
    if (!(Test-Path $binary)) { throw 'Installer omitted writer binary' }
    if ((& $binary --help | Out-String) -notmatch '--project') { throw 'Installed --help failed' }
    if ((& $binary --version | Out-String) -notmatch '0\.0\.0') { throw 'Installed --version failed' }
    if ((Get-ProtocolSnapshot) -ne $before) { throw 'Installer changed another recite:// association' }

    $process = Start-Process $installer[0].FullName -ArgumentList @('/S', '/NS', "/D=$installRoot") -Wait -PassThru
    if ($process.ExitCode -ne 0 -or !(Test-Path $binary)) { throw 'NSIS same-version reinstall failed' }
    if ((Get-ProtocolSnapshot) -ne $before) { throw 'Reinstall changed another recite:// association' }
}
finally {
    try {
        $uninstaller = Join-Path $installRoot 'uninstall.exe'
        if (Test-Path $uninstaller) {
            $process = Start-Process $uninstaller -ArgumentList @('/S', "_?=$installRoot") -Wait -PassThru
            if ($process.ExitCode -ne 0) { throw "NSIS uninstall failed: $($process.ExitCode)" }
        }
        if ($null -ne $before -and (Get-ProtocolSnapshot) -ne $before) {
            throw 'Uninstall changed another recite:// association'
        }
        if (Test-Path (Join-Path $installRoot 'recite-writer.exe')) { throw 'NSIS uninstall left writer binary behind' }
    }
    finally {
        if ($createdSentinel) { Remove-Item $protocol -Recurse -Force }
    }
}
