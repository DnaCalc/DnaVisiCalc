[CmdletBinding()]
param(
    [string]$Label = 'conformance',
    [string]$ArtifactManifest = '.tmp/engine_artifacts_optimized_latest.json',
    [string]$OutRoot = '.tmp'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false

if (-not (Test-Path $ArtifactManifest)) {
    throw "Artifact manifest not found: $ArtifactManifest. Run scripts/windows/build_coreengines_optimized.ps1 first."
}

$manifest = Get-Content $ArtifactManifest -Raw | ConvertFrom-Json
$runId = Get-Date -Format 'yyyyMMddTHHmmssZ'
$runDir = Join-Path $OutRoot ("engine_conformance_matrix_{0}_{1}" -f $Label, $runId)
New-Item -ItemType Directory -Force -Path $runDir | Out-Null

$engines = @(
    @{
        name = 'rust-release'
        env_engine = 'rust-core'
        dll = $manifest.rust_release_dll
    },
    @{
        name = 'dotnet-managed-jit'
        env_engine = 'dotnet-core'
        dll = $manifest.dotnet_managed_jit_dll
    },
    @{
        name = 'dotnet-native-aot'
        env_engine = 'dotnet-core'
        dll = $manifest.dotnet_native_aot_dll
    },
    @{
        name = 'c-native'
        env_engine = 'dotnet-core'
        dll = $manifest.c_release_dll
    },
    @{
        name = 'ocaml-core'
        env_engine = 'dotnet-core'
        dll = $manifest.ocaml_release_dll
    }
)

$results = @()

foreach ($engine in $engines) {
    Write-Host ("== conformance / engine {0} ==" -f $engine.name)
    $outFile = Join-Path $runDir ("{0}.out" -f $engine.name)
    $errFile = Join-Path $runDir ("{0}.err" -f $engine.name)

    $oldEngine = $env:DNAVISICALC_COREENGINE
    $oldDll = $env:DNAVISICALC_COREENGINE_DLL
    try {
        $env:DNAVISICALC_COREENGINE = $engine.env_engine
        $env:DNAVISICALC_COREENGINE_DLL = $engine.dll

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $cmd = "cargo test -p dnavisicalc-engine --test conformance_smoke -- --test-threads=1 1> `"$outFile`" 2> `"$errFile`""
        cmd /d /s /c $cmd | Out-Null
        $exitCode = $LASTEXITCODE
        $sw.Stop()

        $text = if (Test-Path $outFile) { Get-Content $outFile -Raw } else { '' }
        $pass = $false
        $passedCount = $null
        if ($text -match 'test result:\s+ok\.\s+([0-9]+)\s+passed') {
            $pass = $true
            $passedCount = [int]$Matches[1]
        }

        $results += [pscustomobject]@{
            engine = $engine.name
            env_engine = $engine.env_engine
            dll = $engine.dll
            exit_code = $exitCode
            passed = $pass
            passed_tests = $passedCount
            wall_ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)
            out_file = $outFile
            err_file = $errFile
        }
    }
    finally {
        $env:DNAVISICALC_COREENGINE = $oldEngine
        $env:DNAVISICALC_COREENGINE_DLL = $oldDll
    }
}

$csvPath = Join-Path $runDir 'results.csv'
$jsonPath = Join-Path $runDir 'results.json'
$metaPath = Join-Path $runDir 'meta.json'

$results | Export-Csv -Path $csvPath -NoTypeInformation -Encoding UTF8
$results | ConvertTo-Json -Depth 5 | Set-Content -Path $jsonPath -Encoding UTF8

$allPassed = @($results | Where-Object { -not $_.passed -or $_.exit_code -ne 0 }).Count -eq 0
$meta = [ordered]@{
    label = $Label
    run_id = $runId
    run_dir = (Resolve-Path $runDir).Path
    artifact_manifest = (Resolve-Path $ArtifactManifest).Path
    all_passed = $allPassed
    engine_count = $engines.Count
}
$meta | ConvertTo-Json -Depth 4 | Set-Content -Path $metaPath -Encoding UTF8

Write-Host ''
Write-Host "Run directory: $runDir"
Write-Host "CSV: $csvPath"
Write-Host "JSON: $jsonPath"
Write-Host "Meta: $metaPath"
if ($allPassed) {
    Write-Host 'CONFORMANCE_OK_ALL'
} else {
    Write-Host 'CONFORMANCE_FAIL'
}
