param(
    [string]$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\\..")).Path,
    [string]$LogPath,
    [string]$WindowTitle = "DNA_VisiCalc_Repro",
    [string]$ProbeKey = "1",
    [int]$KeyDelayMs = 120,
    [switch]$NoBuild,
    [switch]$RequireDuplicate,
    [switch]$KeepRunnerScript
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not [Environment]::UserInteractive) {
    throw "This harness requires an interactive Windows desktop session."
}

$projectRoot = (Resolve-Path $ProjectRoot).Path
if (-not $LogPath) {
    $LogPath = Join-Path $projectRoot "artifacts\\windows\\event-trace.log"
}

$cargoPath = Join-Path $env:USERPROFILE ".cargo\\bin\\cargo.exe"
if (-not (Test-Path $cargoPath)) {
    throw "cargo.exe not found at '$cargoPath'."
}

$wt = Get-Command wt.exe -ErrorAction SilentlyContinue
if (-not $wt) {
    throw "Windows Terminal (wt.exe) not found."
}

$binaryPath = Join-Path $projectRoot "target\\debug\\dnavisicalc.exe"
if (-not $NoBuild) {
    & $cargoPath build -p dnavisicalc-tui --bin dnavisicalc --manifest-path (Join-Path $projectRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path $binaryPath)) {
    throw "Binary not found at '$binaryPath'. Build first or remove -NoBuild."
}

$logDir = Split-Path $LogPath -Parent
if ($logDir) {
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
}
if (Test-Path $LogPath) {
    Remove-Item $LogPath -Force
}

$runnerScriptPath = Join-Path ([System.IO.Path]::GetTempPath()) ("dnavisicalc-runner-{0}.ps1" -f [guid]::NewGuid().ToString("N"))
$escapedLog = $LogPath.Replace("'", "''")
$escapedBin = $binaryPath.Replace("'", "''")
$escapedTitle = $WindowTitle.Replace("'", "''")
$runnerScript = @"
`$host.UI.RawUI.WindowTitle = '$escapedTitle'
`$env:DNAVISICALC_EVENT_TRACE = '$escapedLog'
& '$escapedBin'
"@
Set-Content -Path $runnerScriptPath -Value $runnerScript -Encoding ASCII

try {
    $quotedTitle = '"' + ($WindowTitle.Replace('"', '\"')) + '"'
    $quotedRunner = '"' + ($runnerScriptPath.Replace('"', '\"')) + '"'
    $wtArgString = "new-tab --title $quotedTitle -- powershell -NoProfile -ExecutionPolicy Bypass -File $quotedRunner"
    Start-Process -FilePath $wt.Source -ArgumentList $wtArgString | Out-Null

    Start-Sleep -Milliseconds 700
    $sendKeysScript = Join-Path $PSScriptRoot "send_keys.ps1"
    try {
        & $sendKeysScript -WindowTitle $WindowTitle -Keys @("e", $ProbeKey, "{ENTER}", "q") -DelayMs $KeyDelayMs -ActivationTimeoutMs 12000
    }
    catch {
        # Fallback when tab title is not exposed as a window title.
        & $sendKeysScript -WindowTitle "Windows Terminal" -Keys @("e", $ProbeKey, "{ENTER}", "q") -DelayMs $KeyDelayMs -ActivationTimeoutMs 12000
    }
    Start-Sleep -Milliseconds 1200

    $deadline = (Get-Date).AddSeconds(10)
    while ((-not (Test-Path $LogPath)) -and ((Get-Date) -lt $deadline)) {
        Start-Sleep -Milliseconds 150
    }

    if (-not (Test-Path $LogPath)) {
        throw "No event trace produced at '$LogPath'."
    }

    $lines = Get-Content $LogPath
    if ($lines.Count -eq 0) {
        throw "Event trace log is empty at '$LogPath'."
    }

    $escapedKey = [regex]::Escape($ProbeKey)
    $mappedPattern = "code=Char\('$escapedKey'\).*action=InputChar\('$escapedKey'\)"
    $mapped = @($lines | Where-Object { $_ -match $mappedPattern } | ForEach-Object { $_.ToString() })
    $pressMapped = @($mapped | Where-Object { $_ -match "kind=Press" })
    $releaseMapped = @($mapped | Where-Object { $_ -match "kind=Release" })
    $repeatMapped = @($mapped | Where-Object { $_ -match "kind=Repeat" })

    $summary = [ordered]@{
        project_root = $projectRoot
        log_path = $LogPath
        probe_key = $ProbeKey
        total_events = $lines.Count
        mapped_probe_key_events = $mapped.Count
        press_mapped = $pressMapped.Count
        release_mapped = $releaseMapped.Count
        repeat_mapped = $repeatMapped.Count
        duplicate_detected = ($mapped.Count -ge 2)
        mapped_lines = $mapped
    }

    $summary | ConvertTo-Json -Depth 4

    if ($RequireDuplicate -and ($mapped.Count -lt 2)) {
        throw "Duplicate key mapping not reproduced for key '$ProbeKey'. mapped_probe_key_events=$($mapped.Count)"
    }
}
finally {
    if (-not $KeepRunnerScript) {
        Remove-Item $runnerScriptPath -ErrorAction SilentlyContinue
    }
}
