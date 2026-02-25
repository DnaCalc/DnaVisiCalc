# DNA VisiCalc v0.1.1 (Windows x64) - Run Instructions

No Rust or Cargo installation is required to run this release.

## 1. Download
- Open: `https://github.com/DnaCalc/DnaVisiCalc/releases/tag/v0.1.1`
- Download asset: `dnavisicalc-v0.1.1-windows-x64.zip`

## 2. Extract
- Extract the zip to a writable folder, for example:
  - `C:\Tools\dnavisicalc-v0.1.1\`

## 3. Run
- Start the app with either:
  - `dnavisicalc.exe`, or
  - `run_dnavisicalc.bat`

PowerShell option:

```powershell
cd C:\Tools\dnavisicalc-v0.1.1
.\dnavisicalc.exe
```

## 4. In-app help
- Press `?` or `F1` to open full help (keys, commands, supported functions).

## 5. Optional event trace (Windows terminal/input diagnostics)

```powershell
$env:DNAVISICALC_EVENT_TRACE = "event-trace.log"
.\dnavisicalc.exe
```

This writes a key-event trace file in the current folder.
