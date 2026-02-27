# Validation Results

## Build

- `Dvc.Core`: pass
- `Dvc.Native`: pass

## Tests

- `Dvc.Core.Tests`: pass (5/5)
- `Dvc.Interop.Tests`: pass (3/3)
- `Dvc.E2E.Tests`: pass (1/1)
- Total: pass (9/9)

## Native artifact

- NativeAOT publish (win-x64): pass after explicitly setting MSVC/Windows SDK linker environment.
- Export symbol verification: pass (`dumpbin /exports` shows explicit `dvc_*` symbols, including `dvc_engine_create`, `dvc_recalculate`, `dvc_udf_register`, `dvc_insert_row`, `dvc_chart_get_output`).

## Notes

- Initial NativeAOT publish attempt failed due toolchain resolution (`link.exe` from Git in PATH and missing LIB paths).
- Resolved by prepending MSVC linker path and setting `LIB` to MSVC + Windows SDK libs.
- Publish output includes a non-fatal console line: `The input line is too long. The syntax of the command is incorrect.`; final publish still completed successfully.
