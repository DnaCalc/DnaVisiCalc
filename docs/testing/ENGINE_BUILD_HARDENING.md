# Engine Build Hardening (Windows)

This note captures the recurring Windows build failures we hit during cross-engine
performance work, and the stable fix path.

## Canonical build entrypoint

Use:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/windows/build_coreengines_optimized.ps1
```

The script builds optimized artifacts for:

- Rust (`release`)
- .NET managed-JIT (`Release`, DNNE export DLL)
- .NET NativeAOT (`Release`, native shared library)
- OCaml (`build_release.cmd`, including C ABI smoke/closure/conformance)
- C (`-O2`, plus C ABI smoke/closure/conformance executables)

It also writes:

- `.tmp/engine_artifacts_optimized_latest.json` with resolved DLL/exe paths for
  perf/conformance runners.
- For managed-JIT (`Dvc.NativeNE.dll`), stages `Dvc.Native.runtimeconfig.json`
  next to the export DLL so C-hosted loading works.
- For NativeAOT, snapshots `publish/Dvc.Native.dll` to
  `publish-native-aot/Dvc.Native.dll` so subsequent managed publish does not
  overwrite the perf target.

## Root causes and permanent mitigations

1. Wrong linker selected for .NET NativeAOT  
Cause: `C:\Program Files\Git\usr\bin\link.exe` shadowed MSVC `link.exe`.  
Symptoms: GNU-style `link` errors (`extra operand '/DEF:...'`, `--help` output).

Mitigation in script:
- Force Visual Studio 2026 Insiders (`C:\Program Files\Microsoft Visual Studio\18\Insiders`).
- Bootstrap with `vcvars64.bat`.
- Use a controlled minimal `PATH` before entering the VS toolchain bootstrap.

2. `cmd.exe` line length / syntax failures during VS env bootstrap  
Cause: very large inherited `PATH` exceeded `cmd`/batch expansion limits.  
Symptoms: `The input line is too long.` and `The syntax of the command is incorrect.`

Mitigation in script:
- Shrink `PATH` to a known minimal set before invoking `vcvars64.bat`.
- Restore original `PATH` after each VS-scoped command.

3. PowerShell parsing pitfalls for GCC linker flags  
Cause: unquoted comma-separated `-Wl,...` flags were parsed as argument lists.  
Symptoms: PowerShell parser errors (`Missing argument in parameter list`).

Mitigation in script:
- Quote linker flags as single arguments.

4. Silent continuation after failed external commands  
Cause: `$ErrorActionPreference = 'Stop'` does not automatically throw on non-zero
exit from external tools.

Mitigation in script:
- Wrap each step with explicit `$LASTEXITCODE` checks and fail fast.

5. Managed-JIT export missing runtimeconfig sidecar  
Cause: `dotnet publish` writes `Dvc.Native.runtimeconfig.json` to `publish/`, while
the native export DLL (`Dvc.NativeNE.dll`) lives one directory above.  
Symptoms: `The specified runtimeconfig.json ... does not exist`.

Mitigation in script:
- Copy `publish/Dvc.Native.runtimeconfig.json` next to `Dvc.NativeNE.dll`.

6. NativeAOT and managed publish output collision  
Cause: both variants write into the same `.../win-x64/publish` directory by
default, and whichever publish runs second overwrites the first variant output.

Mitigation in script:
- Run NativeAOT publish first and copy its DLL to `publish-native-aot/`.
- Run managed publish after that.

## Toolchain preference policy

Per local policy on this machine, build tooling should prefer Visual Studio 2026
Insiders (`18\Insiders`) for .NET native toolchain steps.

The build script enforces that preference and only falls back to `vswhere`
discovery for that same installation path family.
