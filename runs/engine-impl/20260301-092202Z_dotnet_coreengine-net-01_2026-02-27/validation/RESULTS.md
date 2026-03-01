# Validation Results

Record command outputs or concise summaries.

- command: native-aot publish (`dotnet publish ... -p:DvcExportVariant=native-aot`)
  status: pass
  notes: Publish succeeded with explicit `LIB`, `CppLinker`, and `CppLibCreator` on this workstation. Artifact emitted at `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`.

- command: managed-jit build (`dotnet build ... -p:DvcExportVariant=managed-jit`)
  status: pass
  notes: Build succeeded. Managed export artifact emitted at `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/Dvc.NativeNE.dll` with sidecars `Dvc.Native.dll`, `Dvc.Native.runtimeconfig.json`, `Dvc.Native.deps.json`.

- command: export parity check (`dumpbin /exports`)
  status: pass
  notes: `dvc_*` export count is `104` for both managed (`Dvc.NativeNE.dll`) and native-aot (`publish/Dvc.Native.dll`) artifacts.

- command: `cargo test -p dnavisicalc-engine --test conformance_smoke` (managed DNNE DLL)
  status: pass
  notes: `13 passed; 0 failed` with `DNAVISICALC_COREENGINE=dotnet-core` and `DNAVISICALC_COREENGINE_DLL=.../Dvc.NativeNE.dll`.

- command: `cargo test -p dnavisicalc-engine --test conformance_smoke` (native-aot DLL)
  status: pass
  notes: `13 passed; 0 failed` with `DNAVISICALC_COREENGINE=dotnet-core` and `DNAVISICALC_COREENGINE_DLL=.../publish/Dvc.Native.dll`.

## Ordering Caveat Observed During Validation

- Running native-aot publish after managed-jit build can remove managed runtime sidecars from `bin/Release/net10.0/win-x64/`; managed conformance should be run after a fresh managed-jit build.

## Command Output Logs

- `.tmp/runs/20260301-092202Z_dotnet_coreengine-net-01_2026-02-27/logs/nativeaot-publish.log`
- `.tmp/runs/20260301-092202Z_dotnet_coreengine-net-01_2026-02-27/logs/managed-build.log`
- `.tmp/runs/20260301-092202Z_dotnet_coreengine-net-01_2026-02-27/logs/conformance-managed.log`
- `.tmp/runs/20260301-092202Z_dotnet_coreengine-net-01_2026-02-27/logs/conformance-nativeaot.log`
