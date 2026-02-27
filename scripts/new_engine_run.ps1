param(
    [Parameter(Mandatory=$true)][ValidateSet('rust','dotnet')] [string]$Runtime,
    [Parameter(Mandatory=$true)] [string]$ImplementationId,
    [Parameter(Mandatory=$false)] [string]$SpecPackVersion = '2026-02-27'
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repoRoot

$timestamp = (Get-Date).ToUniversalTime().ToString('yyyyMMdd-HHmmssZ')
$runId = "${timestamp}_${Runtime}_${ImplementationId}_${SpecPackVersion}"

$templateDir = Join-Path $repoRoot 'runs/templates/engine_impl'
$runDir = Join-Path $repoRoot ("runs/engine-impl/{0}" -f $runId)

if (Test-Path $runDir) {
    throw "Run directory already exists: $runDir"
}

Copy-Item $templateDir $runDir -Recurse

$manifestPath = Join-Path $runDir 'RUN_MANIFEST.yaml'
$manifest = Get-Content $manifestPath -Raw
$manifest = $manifest -replace '20260228-091500Z_dotnet_coreengine-net-01_2026-02-27', $runId
$manifest = $manifest -replace 'runtime: dotnet', ("runtime: {0}" -f $Runtime)
$manifest = $manifest -replace 'implementation_id: coreengine-net-01', ("implementation_id: {0}" -f $ImplementationId)
$manifest = $manifest -replace 'spec_pack_version: 2026-02-27', ("spec_pack_version: {0}" -f $SpecPackVersion)
$manifest = $manifest -replace 'spec_pack_path: docs/full-engine-spec/2026-02-27', ("spec_pack_path: docs/full-engine-spec/{0}" -f $SpecPackVersion)
$manifest = $manifest -replace 'context_policy_path: runs/engine-impl/<run-id>/inputs/CONTEXT_POLICY.yaml', ("context_policy_path: runs/engine-impl/{0}/inputs/CONTEXT_POLICY.yaml" -f $runId)
$manifest = $manifest -replace 'target_codebase_path: engines/dotnet/coreengine-net-01', ("target_codebase_path: engines/{0}/{1}" -f $Runtime,$ImplementationId)
$manifest = $manifest -replace 'started_at_utc: <fill>', ("started_at_utc: {0}" -f (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ'))
$manifest = $manifest -replace 'source_repo_commit: <fill-at-start>', ("source_repo_commit: {0}" -f (git rev-parse HEAD))
Set-Content $manifestPath $manifest

$specRefPath = Join-Path $runDir 'inputs/SPEC_PACK_REF.md'
if (Test-Path $specRefPath) {
    $specRef = Get-Content $specRefPath -Raw
    $specRef = $specRef -replace 'spec_pack_version: 2026-02-27', ("spec_pack_version: {0}" -f $SpecPackVersion)
    $specRef = $specRef -replace 'spec_pack_root: docs/full-engine-spec/2026-02-27', ("spec_pack_root: docs/full-engine-spec/{0}" -f $SpecPackVersion)
    Set-Content $specRefPath $specRef
}

$contextPolicyPath = Join-Path $runDir 'inputs/CONTEXT_POLICY.yaml'
if (Test-Path $contextPolicyPath) {
    $contextPolicy = Get-Content $contextPolicyPath -Raw
    $contextPolicy = $contextPolicy -replace 'docs/full-engine-spec/2026-02-27', ("docs/full-engine-spec/{0}" -f $SpecPackVersion)
    $contextPolicy = $contextPolicy -replace 'runs/engine-impl/<run-id>/inputs', ("runs/engine-impl/{0}/inputs" -f $runId)
    $contextPolicy = $contextPolicy -replace 'engines/<runtime>/<implementation-id>', ("engines/{0}/{1}" -f $Runtime,$ImplementationId)
    $contextPolicy = $contextPolicy -replace 'runs/engine-impl/<run-id>', ("runs/engine-impl/{0}" -f $runId)
    Set-Content $contextPolicyPath $contextPolicy
}

$inputHashesPath = Join-Path $runDir 'inputs/INPUT_HASHES.json'
if (Test-Path $inputHashesPath) {
    $inputHashes = Get-Content $inputHashesPath -Raw
    $inputHashes = $inputHashes -replace 'docs/full-engine-spec/2026-02-27', ("docs/full-engine-spec/{0}" -f $SpecPackVersion)
    $inputHashes = $inputHashes -replace 'runs/engine-impl/<run-id>', ("runs/engine-impl/{0}" -f $runId)
    Set-Content $inputHashesPath $inputHashes
}

Write-Host "Created run bundle: runs/engine-impl/$runId"
Write-Host "Next: fill inputs/PROMPT_INPUT.md, inputs/RUN_ADDITIONAL_REQUIREMENTS.md, inputs/CONTEXT_POLICY.yaml, and inputs/INPUT_HASHES.json before coding."
