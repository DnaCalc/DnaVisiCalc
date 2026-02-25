# DNA VisiCalc v0.2.0

DNA VisiCalc v0.2.0 is a Windows x64 packaged release of the Round 0 pathfinder.

No Rust or Cargo installation is required to run the packaged app.

## Included asset
- `dnavisicalc-v0.2.0-windows-x64.zip`
  - `dnavisicalc.exe`
  - `README_RELEASE.txt`
  - `HELP_QUICK_REFERENCE.txt`
  - `LICENSE.txt`

## Run instructions (Windows x64)
1. Download and extract `dnavisicalc-v0.2.0-windows-x64.zip`.
2. Run `dnavisicalc.exe`.
3. Press `?` or `F1` inside the app for full help and function list.

Optional trace logging:

```powershell
$env:DNAVISICALC_EVENT_TRACE = "event-trace.log"
.\dnavisicalc.exe
```

## Help summary
- Navigation: arrows / `h j k l`, `Shift+arrows` / `Shift+HJKL`, `Enter`, `e`, `F2`, `Ctrl+C`, `Ctrl+V`, `:`, `r`, `q`, `?`, `F1`
- Paste Special (`Ctrl+V`): `All`, `Formulas`, `Values`, `Values+KeepDestFmt`, `Formatting`
- Commands: `w`, `o`, `set`, `name`, `name clear`, `fmt`, `mode auto|manual`, `recalc`, `quit`
- Functions:
  - aggregates/logical: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`
  - math/trig: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - financial/table/error: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`
  - text/arrays and lambda model: `CONCAT`, `LEN`, `SEQUENCE`, `RANDARRAY`, `LET`, `LAMBDA`, `MAP` (incl. array-returning lambda spill tiling)
  - reference helpers: `INDIRECT` (A1 + R1C1), `OFFSET`, `ROW`, `COLUMN`

## Compatibility notes
- Compatibility matrix: `docs/VISICALC_COMPATIBILITY_MATRIX.md`
- This release includes implemented features plus classification of problematic/validation-needed compatibility areas.
