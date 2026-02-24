# DNA VisiCalc v0.1

DNA VisiCalc v0.1 is the first packaged Windows x64 release of the Round 0 pathfinder.

## Included asset
- `dnavisicalc-v0.1-windows-x64.zip`
  - `dnavisicalc.exe`
  - `README_RELEASE.txt`
  - `HELP_QUICK_REFERENCE.txt`
  - `LICENSE.txt`

## Run instructions (Windows x64)
1. Download and extract `dnavisicalc-v0.1-windows-x64.zip`.
2. Run `dnavisicalc.exe`.
3. Press `?` or `F1` inside the app for full help.

Optional trace logging:

```powershell
$env:DNAVISICALC_EVENT_TRACE = "event-trace.log"
.\dnavisicalc.exe
```

## Help summary
- Navigation: arrows / `h j k l`, `Enter`, `e`, `:`, `r`, `q`, `?`, `F1`
- Commands: `w`, `o`, `set`, `mode auto|manual`, `recalc`, `quit`
- Function coverage includes:
  - aggregates/logical: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`
  - math/trig: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - financial/table/error: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`
  - text and arrays: `CONCAT`, `LEN`, `SEQUENCE`, `RANDARRAY`

## Compatibility notes
- Compatibility matrix: `docs/VISICALC_COMPATIBILITY_MATRIX.md`
- This release includes implemented features plus classification of problematic/validation-needed compatibility areas.
