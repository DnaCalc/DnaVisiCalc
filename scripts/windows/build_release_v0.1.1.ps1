Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$version = "v0.1.1"
$targetDir = "target_release_v0_1_1"
$packageName = "dnavisicalc-$version-windows-x64"
$releaseRoot = Join-Path "artifacts/release" $version
$stagingDir = Join-Path $releaseRoot $packageName
$zipPath = Join-Path $releaseRoot "$packageName.zip"

New-Item -ItemType Directory -Force -Path $releaseRoot | Out-Null
if (Test-Path $stagingDir) {
    Remove-Item -Recurse -Force $stagingDir
}
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null

$env:CARGO_TARGET_DIR = $targetDir
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release -p dnavisicalc-tui --bin dnavisicalc

$exePath = Join-Path $targetDir "release\dnavisicalc.exe"
if (-not (Test-Path $exePath)) {
    throw "Release binary not found: $exePath"
}

Copy-Item $exePath (Join-Path $stagingDir "dnavisicalc.exe")
Copy-Item "LICENSE" (Join-Path $stagingDir "LICENSE.txt")
Copy-Item "docs/release/WINDOWS_RUN_v0.1.1.md" (Join-Path $stagingDir "README_RELEASE.txt")
Copy-Item "docs/release/HELP_QUICK_REFERENCE_v0.1.1.md" (Join-Path $stagingDir "HELP_QUICK_REFERENCE.txt")

$launcherPath = Join-Path $stagingDir "run_dnavisicalc.bat"
$launcher = @"
@echo off
setlocal
cd /d %~dp0
dnavisicalc.exe
"@
Set-Content -Path $launcherPath -Value $launcher -Encoding ASCII

if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $zipPath -CompressionLevel Optimal

Write-Output "Created release zip: $zipPath"
