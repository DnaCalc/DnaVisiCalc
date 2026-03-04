[CmdletBinding()]
param(
    [string]$Label = 'ramp',
    [string]$ArtifactManifest = '.tmp/engine_artifacts_optimized_latest.json',
    [string]$OutRoot = '.tmp',
    [double]$WallLimitMs = 10000.0
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Parse-BenchmarkOutput {
    param(
        [Parameter(Mandatory = $true)][string]$Text
    )

    $linePattern = '(?im)^(?<engine>[a-z0-9\-]+): setup=(?<setup>[0-9.]+)ms initial_recalc=(?<initial>[0-9.]+)ms recalc\[min/p50/p95/mean/max\]=(?<min>[0-9.]+)/(?<p50>[0-9.]+)/(?<p95>[0-9.]+)/(?<mean>[0-9.]+)/(?<max>[0-9.]+)ms committed_epoch=(?<epoch>[0-9]+)'
    $headerPattern = 'iterations=(?<iter>[0-9]+), full_data=(?<full>true|false), formula_region=(?<cols>[0-9]+)x(?<rows>[0-9]+), mutation=(?<mutation>[^\r\n]+)'

    $header = [ordered]@{
        iterations = $null
        full_data = $null
        formula_cols = $null
        formula_rows = $null
        mutation = $null
    }
    if ($Text -match $headerPattern) {
        $header.iterations = [int]$Matches['iter']
        $header.full_data = $Matches['full']
        $header.formula_cols = [int]$Matches['cols']
        $header.formula_rows = [int]$Matches['rows']
        $header.mutation = $Matches['mutation']
    }

    $row = [ordered]@{
        parsed = $false
        parsed_engine_label = $null
        setup_ms = $null
        initial_recalc_ms = $null
        recalc_min_ms = $null
        recalc_p50_ms = $null
        recalc_p95_ms = $null
        recalc_mean_ms = $null
        recalc_max_ms = $null
        committed_epoch = $null
        iterations = $header.iterations
        full_data = $header.full_data
        formula_cols = $header.formula_cols
        formula_rows = $header.formula_rows
        mutation = $header.mutation
    }

    if ($Text -match $linePattern) {
        $row.parsed = $true
        $row.parsed_engine_label = $Matches['engine']
        $row.setup_ms = [double]$Matches['setup']
        $row.initial_recalc_ms = [double]$Matches['initial']
        $row.recalc_min_ms = [double]$Matches['min']
        $row.recalc_p50_ms = [double]$Matches['p50']
        $row.recalc_p95_ms = [double]$Matches['p95']
        $row.recalc_mean_ms = [double]$Matches['mean']
        $row.recalc_max_ms = [double]$Matches['max']
        $row.committed_epoch = [uint64]$Matches['epoch']
    }

    return [pscustomobject]$row
}

if (-not (Test-Path $ArtifactManifest)) {
    throw "Artifact manifest not found: $ArtifactManifest. Run scripts/windows/build_coreengines_optimized.ps1 first."
}

$manifest = Get-Content $ArtifactManifest -Raw | ConvertFrom-Json
$perfExe = $manifest.perf_harness_release_exe
if (-not (Test-Path $perfExe)) {
    throw "Perf harness executable not found: $perfExe"
}

$runId = Get-Date -Format 'yyyyMMddTHHmmssZ'
$runDir = Join-Path $OutRoot ("engine_ramp_until_two_over_10s_{0}_{1}" -f $Label, $runId)
New-Item -ItemType Directory -Force -Path $runDir | Out-Null

$engines = @(
    @{
        name = 'rust-release'
        args = @('--backend', 'rust-core', '--rust-dll', $manifest.rust_release_dll)
    },
    @{
        name = 'rust-fml'
        args = @('--backend', 'rust-core', '--rust-dll', $manifest.rust_fml_release_dll)
    },
    @{
        name = 'dotnet-managed-jit'
        args = @('--backend', 'dotnet-core', '--dotnet-dll', $manifest.dotnet_managed_jit_dll)
    },
    @{
        name = 'dotnet-native-aot'
        args = @('--backend', 'dotnet-core', '--dotnet-dll', $manifest.dotnet_native_aot_dll)
    },
    @{
        name = 'c-native'
        args = @('--backend', 'dotnet-core', '--dotnet-dll', $manifest.c_release_dll)
    },
    @{
        name = 'ocaml-core'
        args = @('--include-ocaml', '--backend', 'ocaml-core', '--ocaml-dll', $manifest.ocaml_release_dll)
    }
)

$jobs = @(
    @{ id='r01'; args=@('--iterations','20','--full-data','false','--formula-cols','20','--formula-rows','90') },
    @{ id='r02'; args=@('--iterations','30','--full-data','false','--formula-cols','28','--formula-rows','130') },
    @{ id='r03'; args=@('--iterations','40','--full-data','true','--formula-cols','36','--formula-rows','170') },
    @{ id='r04'; args=@('--iterations','60','--full-data','true','--formula-cols','44','--formula-rows','210') },
    @{ id='r05'; args=@('--iterations','80','--full-data','true','--formula-cols','52','--formula-rows','230') },
    @{ id='r06'; args=@('--iterations','100','--full-data','true','--formula-cols','60','--formula-rows','246') },
    @{ id='r07'; args=@('--iterations','120','--full-data','true','--formula-cols','63','--formula-rows','254') },
    @{ id='r08'; args=@('--iterations','160','--full-data','true','--formula-cols','63','--formula-rows','254') },
    @{ id='r09'; args=@('--iterations','220','--full-data','true','--formula-cols','63','--formula-rows','254') },
    @{ id='r10'; args=@('--iterations','300','--full-data','true','--formula-cols','63','--formula-rows','254') },
    @{ id='r11'; args=@('--iterations','420','--full-data','true','--formula-cols','63','--formula-rows','254') }
)

$results = @()
$halted = $false
$haltReason = ''

foreach ($job in $jobs) {
    if ($halted) { break }
    $jobRows = @()

    foreach ($engine in $engines) {
        Write-Host ("== job {0} / engine {1} ==" -f $job.id, $engine.name)

        $outFile = Join-Path $runDir ("{0}_{1}.out" -f $job.id, $engine.name)
        $errFile = Join-Path $runDir ("{0}_{1}.err" -f $job.id, $engine.name)
        $cli = @($job.args + $engine.args)

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & $perfExe @cli 2> $errFile | Tee-Object -FilePath $outFile | Out-Null
        $exitCode = $LASTEXITCODE
        $sw.Stop()
        $wallMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 3)

        $text = if (Test-Path $outFile) { Get-Content $outFile -Raw } else { '' }
        $parsed = Parse-BenchmarkOutput -Text $text

        $row = [pscustomobject]@{
            job = $job.id
            engine = $engine.name
            parsed_engine_label = $parsed.parsed_engine_label
            parsed = $parsed.parsed
            exit_code = $exitCode
            wall_ms = $wallMs
            over_limit = ($wallMs -gt $WallLimitMs)
            setup_ms = $parsed.setup_ms
            initial_recalc_ms = $parsed.initial_recalc_ms
            recalc_min_ms = $parsed.recalc_min_ms
            recalc_p50_ms = $parsed.recalc_p50_ms
            recalc_p95_ms = $parsed.recalc_p95_ms
            recalc_mean_ms = $parsed.recalc_mean_ms
            recalc_max_ms = $parsed.recalc_max_ms
            committed_epoch = $parsed.committed_epoch
            iterations = $parsed.iterations
            full_data = $parsed.full_data
            formula_cols = $parsed.formula_cols
            formula_rows = $parsed.formula_rows
            mutation = $parsed.mutation
            out_file = $outFile
            err_file = $errFile
        }
        $results += $row
        $jobRows += $row
    }

    $overCount = @($jobRows | Where-Object { $_.over_limit }).Count
    if ($overCount -ge 2) {
        $halted = $true
        $enginesOver = ($jobRows | Where-Object { $_.over_limit } | Select-Object -ExpandProperty engine) -join ', '
        $haltReason = ">=2 engines exceeded ${WallLimitMs}ms at $($job.id): $enginesOver"
    }
}

$csvPath = Join-Path $runDir 'results.csv'
$jsonPath = Join-Path $runDir 'results.json'
$metaPath = Join-Path $runDir 'meta.json'

$results | Export-Csv -Path $csvPath -NoTypeInformation -Encoding UTF8
$results | ConvertTo-Json -Depth 6 | Set-Content -Path $jsonPath -Encoding UTF8

$meta = [ordered]@{
    label = $Label
    run_id = $runId
    run_dir = (Resolve-Path $runDir).Path
    artifact_manifest = (Resolve-Path $ArtifactManifest).Path
    wall_limit_ms = $WallLimitMs
    halted = $halted
    halt_reason = $haltReason
    jobs_planned = $jobs.Count
    result_rows = $results.Count
}
$meta | ConvertTo-Json -Depth 5 | Set-Content -Path $metaPath -Encoding UTF8

Write-Host ''
Write-Host "Run directory: $runDir"
Write-Host "CSV: $csvPath"
Write-Host "JSON: $jsonPath"
Write-Host "Meta: $metaPath"
if ($halted) {
    Write-Host "HALTED: $haltReason"
} else {
    Write-Host "RAMP_COMPLETED_WITHOUT_HALT"
}
