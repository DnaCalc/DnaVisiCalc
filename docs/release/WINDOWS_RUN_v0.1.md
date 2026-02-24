# DNA VisiCalc v0.1 (Windows x64) - Run Instructions

## 1. Download
- Open: `https://github.com/DnaCalc/DnaVisiCalc/releases/tag/v0.1`
- Download asset: `dnavisicalc-v0.1-windows-x64.zip`

## 2. Extract
- Extract the zip to a writable folder, for example:
  - `C:\Tools\dnavisicalc-v0.1\`

## 3. Run
- Double-click `dnavisicalc.exe` or run from PowerShell:

```powershell
cd C:\Tools\dnavisicalc-v0.1
.\dnavisicalc.exe
```

## 4. In-app help
- Press `?` or `F1` to open full help (keys, commands, and supported function list).

## 5. Optional event trace (Windows terminal/input diagnostics)

```powershell
$env:DNAVISICALC_EVENT_TRACE = "event-trace.log"
.\dnavisicalc.exe
```

This writes a key-event trace file in the current folder.
