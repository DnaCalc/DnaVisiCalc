# Windows Terminal Key Repro Harness

This harness is for reproducing and documenting terminal key handling bugs (for example: one typed key appearing twice in the app).

## What it does

1. Launches `dnavisicalc` in Windows Terminal (`wt.exe`).
2. Enables runtime key-event tracing via `DNAVISICALC_EVENT_TRACE`.
3. Drives the terminal using Windows `SendKeys` automation.
4. Produces a trace log and JSON summary that can be checked into bug reports.

## Scripts

- `scripts/windows/send_keys.ps1`
- `scripts/windows/repro_double_keypress.ps1`

## Prerequisites

- Windows desktop session (interactive, not headless service session).
- Windows Terminal (`wt.exe`).
- PowerShell.
- Built app binary (`target/debug/dnavisicalc.exe`) or Cargo available to build.

## Run the repro harness

From repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/repro_double_keypress.ps1 -RequireDuplicate
```

Optional flags:

- `-NoBuild` (skip `cargo build`).
- `-ProbeKey 1` (key to detect in trace).
- `-LogPath artifacts/windows/my-trace.log`.
- `-KeyDelayMs 120`.

## Output

The script prints a JSON summary including:

- `mapped_probe_key_events`
- `press_mapped`
- `release_mapped`
- `repeat_mapped`
- `duplicate_detected`
- `mapped_lines`

The raw trace log is written to:

- `artifacts/windows/event-trace.log` (default)

Each line includes:

- event sequence and timestamp
- app mode
- key kind/code/modifiers/state
- mapped action (or `None`)

## Cargo test hook (opt-in)

There is an opt-in integration test:

- `crates/dnavisicalc-tui/tests/windows_terminal_repro_harness.rs`

It only runs when:

- `DNAVISICALC_RUN_WINDOWS_TERMINAL_E2E=1`

Example:

```powershell
$env:DNAVISICALC_RUN_WINDOWS_TERMINAL_E2E = "1"
cargo test -p dnavisicalc-tui windows_terminal_sendkeys_harness_runs_when_enabled -- --nocapture
```
