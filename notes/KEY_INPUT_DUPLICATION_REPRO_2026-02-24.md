# KEY_INPUT_DUPLICATION_REPRO_2026-02-24.md

## Issue
- In interactive use, a single character keypress can appear twice (example: typing `1` yields `11`).

## Reproduction scaffolding added

1. Runtime key-event trace capture
- Env var: `DNAVISICALC_EVENT_TRACE=<path>`
- Captures per key event:
  - mode, kind (`Press`/`Release`/`Repeat`), key code, modifiers, mapped action.

2. Windows Terminal automation scripts
- `scripts/windows/repro_double_keypress.ps1`
- `scripts/windows/send_keys.ps1`
- Uses `wt.exe` + `WScript.Shell.SendKeys` to drive the app.

3. Code-level repro assertion
- `keymap` test demonstrates current mapping behavior:
  - both `KeyEventKind::Press` and `KeyEventKind::Release` map to `Action::InputChar`.
- This is a direct mechanism that can produce duplicate characters when terminal emits both events.

## Current status
- Repro infrastructure is in place.
- No input-mapping fix has been applied yet.
- In this automation environment, foreground-window activation for SendKeys failed; harness remains intended for local interactive desktop execution.

## Next step (after confirmed high-level repro)
- Apply input filtering policy (likely key-kind filtering) and re-run both:
  - code-level repro tests,
  - Windows Terminal automation harness,
  - full workspace tests.
