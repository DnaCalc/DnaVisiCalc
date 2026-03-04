[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '../..')
Set-Location $repoRoot

function Require-Command {
    param([Parameter(Mandatory = $true)][string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found on PATH: $Name"
    }
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Title,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    Write-Host "== $Title =="
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Title failed with exit code $LASTEXITCODE"
    }
}

function Get-Vs18VcvarsPath {
    $preferred = 'C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat'
    if (Test-Path $preferred) {
        return $preferred
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path $vswhere)) {
        throw "VS 18 Insiders toolchain not found (missing preferred path and vswhere)."
    }

    $instances = & $vswhere -all -prerelease -products * -format json | ConvertFrom-Json
    $candidate = $instances |
        Where-Object { $_.installationPath -like '*\Microsoft Visual Studio\18\Insiders' } |
        Select-Object -First 1
    if (-not $candidate) {
        throw "Visual Studio 18 Insiders was requested but no matching installation was found."
    }

    $vcvars = Join-Path $candidate.installationPath 'VC\Auxiliary\Build\vcvars64.bat'
    if (-not (Test-Path $vcvars)) {
        throw "Found VS 18 Insiders at '$($candidate.installationPath)' but vcvars64.bat is missing."
    }

    return $vcvars
}

function Invoke-InVs18Toolchain {
    param(
        [Parameter(Mandatory = $true)][string]$Title,
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string]$VcvarsPath
    )

    $oldPath = $env:PATH
    try {
        # Keep cmd.exe PATH small enough for vcvars updates and avoid Git's GNU link.exe collision.
        $basePath = @(
            'C:\Windows\System32'
            'C:\Windows'
            'C:\Program Files\dotnet'
            'C:\Program Files\Git\cmd'
        ) | Where-Object { Test-Path $_ }
        $env:PATH = ($basePath -join ';')

        $wrapped = "call `"$VcvarsPath`" >nul && where link && $Command"
        Invoke-Checked -Title $Title -Action { cmd /d /s /c $wrapped }
    }
    finally {
        $env:PATH = $oldPath
    }
}

function Resolve-NewestFile {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Filter,
        [string]$FullNameRegex = '.*'
    )

    $item = Get-ChildItem -Path $Root -Recurse -Filter $Filter -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match $FullNameRegex } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $item) {
        throw "Could not resolve artifact '$Filter' under '$Root' (regex '$FullNameRegex')."
    }
    return $item.FullName
}

Require-Command cargo
Require-Command dotnet
Require-Command x86_64-w64-mingw32-gcc

$vcvarsPath = Get-Vs18VcvarsPath
Write-Host "Using VS 18 toolchain bootstrap: $vcvarsPath"

Invoke-Checked -Title 'Rust release core engine' -Action {
    cargo build -p dnavisicalc-coreengine-rust --release
}
Invoke-Checked -Title 'Rust-FML release core engine' -Action {
    cargo build -p dnavisicalc-coreengine-rust-fml --release
}
Invoke-Checked -Title 'Rust release perf harness' -Action {
    cargo build -p dnavisicalc-engine --bin engine_perf_compare --release
}

$dotnetProj = 'engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj'
$dotnetBinRoot = 'engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/x64/Release/net10.0/win-x64'
$nativeStableDir = Join-Path $dotnetBinRoot 'publish-native-aot'
New-Item -ItemType Directory -Force -Path $nativeStableDir | Out-Null

Invoke-InVs18Toolchain -Title '.NET native-aot release' -VcvarsPath $vcvarsPath -Command "dotnet publish $dotnetProj -c Release -r win-x64 -p:DvcExportVariant=native-aot -v minimal"
$nativePublishDll = Join-Path $dotnetBinRoot 'publish\Dvc.Native.dll'
if (-not (Test-Path $nativePublishDll)) {
    throw "NativeAOT publish output not found: $nativePublishDll"
}
Copy-Item -Path $nativePublishDll -Destination (Join-Path $nativeStableDir 'Dvc.Native.dll') -Force

# DNNE export (Dvc.NativeNE.dll) requires Dvc.Native.runtimeconfig.json next to the export DLL.
Invoke-InVs18Toolchain -Title '.NET managed-jit release' -VcvarsPath $vcvarsPath -Command "dotnet publish $dotnetProj -c Release -r win-x64 -p:DvcExportVariant=managed-jit -v minimal"
$managedExportDll = Resolve-NewestFile -Root 'engines/dotnet/coreengine-net-01/src/Dvc.Native/bin' -Filter 'Dvc.NativeNE.dll'
$managedExportDir = Split-Path $managedExportDll -Parent
$managedRuntimeConfig = Join-Path $dotnetBinRoot 'publish\Dvc.Native.runtimeconfig.json'
if (-not (Test-Path $managedRuntimeConfig)) {
    throw "Managed runtimeconfig not found at expected path: $managedRuntimeConfig"
}
Copy-Item -Path $managedRuntimeConfig -Destination (Join-Path $managedExportDir 'Dvc.Native.runtimeconfig.json') -Force

Invoke-Checked -Title 'OCaml optimized release + C ABI checks' -Action {
    cmd /d /s /c 'engines\ocaml\coreengine-ocaml-01\build_release.cmd'
}

New-Item -ItemType Directory -Force -Path 'engines/c/coreengine-c-01/dist' | Out-Null
Invoke-Checked -Title 'C optimized DLL' -Action {
    x86_64-w64-mingw32-gcc -O2 -std=c11 -I engines/c/coreengine-c-01/src -shared -o engines/c/coreengine-c-01/dist/dvc_coreengine_c01.dll engines/c/coreengine-c-01/src/dvc_engine.c '-Wl,--out-implib,engines/c/coreengine-c-01/dist/libdvc_coreengine_c01.dll.a' '-Wl,--output-def,engines/c/coreengine-c-01/dist/dvc_coreengine_c01.def'
}
Invoke-Checked -Title 'C ABI test binaries' -Action {
    x86_64-w64-mingw32-gcc -O2 -std=c11 -I engines/c/coreengine-c-01/src -o engines/c/coreengine-c-01/dist/api_smoke.exe engines/c/coreengine-c-01/tests/api_smoke.c engines/c/coreengine-c-01/dist/libdvc_coreengine_c01.dll.a
    x86_64-w64-mingw32-gcc -O2 -std=c11 -I engines/c/coreengine-c-01/src -o engines/c/coreengine-c-01/dist/api_closure.exe engines/c/coreengine-c-01/tests/api_closure.c engines/c/coreengine-c-01/dist/libdvc_coreengine_c01.dll.a
    x86_64-w64-mingw32-gcc -O2 -std=c11 -I engines/c/coreengine-c-01/src -o engines/c/coreengine-c-01/dist/api_conformance_ct.exe engines/c/coreengine-c-01/tests/api_conformance_ct.c engines/c/coreengine-c-01/dist/libdvc_coreengine_c01.dll.a
}
Invoke-Checked -Title 'C ABI conformance executables' -Action {
    Push-Location 'engines/c/coreengine-c-01/dist'
    try {
        .\api_smoke.exe
        if ($LASTEXITCODE -ne 0) { throw "api_smoke.exe failed with exit code $LASTEXITCODE" }
        .\api_closure.exe
        if ($LASTEXITCODE -ne 0) { throw "api_closure.exe failed with exit code $LASTEXITCODE" }
        .\api_conformance_ct.exe
        if ($LASTEXITCODE -ne 0) { throw "api_conformance_ct.exe failed with exit code $LASTEXITCODE" }
    }
    finally {
        Pop-Location
    }
}

$artifacts = [ordered]@{
    rust_release_dll = (Resolve-Path 'target/release/dnavisicalc_coreengine_rust.dll').Path
    rust_fml_release_dll = (Resolve-Path 'target/release/dnavisicalc_coreengine_rust_fml.dll').Path
    dotnet_managed_jit_dll = (Resolve-Path $managedExportDll).Path
    dotnet_native_aot_dll = (Resolve-Path (Join-Path $nativeStableDir 'Dvc.Native.dll')).Path
    ocaml_release_dll = (Resolve-Path 'engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll').Path
    c_release_dll = (Resolve-Path 'engines/c/coreengine-c-01/dist/dvc_coreengine_c01.dll').Path
    perf_harness_release_exe = (Resolve-Path 'target/release/engine_perf_compare.exe').Path
}

$outDir = '.tmp'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$artifactJson = Join-Path $outDir 'engine_artifacts_optimized_latest.json'
$artifacts | ConvertTo-Json -Depth 3 | Set-Content -Path $artifactJson -Encoding UTF8

Write-Host ''
Write-Host 'Artifacts:'
$artifacts.GetEnumerator() | ForEach-Object {
    Write-Host (" - {0}: {1}" -f $_.Key, $_.Value)
}
Write-Host "Wrote artifact manifest: $artifactJson"
Write-Host 'BUILD_OK_ALL'
