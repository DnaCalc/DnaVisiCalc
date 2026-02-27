# Testing Plan (Living)

## Primary goals
1. Harden core correctness and parser robustness.
2. Harden file parsing/writing against malformed inputs.
3. Harden TUI behavior through deterministic scripted tests and replayable capture artifacts.
4. Expand fuzz/property tests for panic resistance and edge cases.
5. Measure and improve line/branch coverage.

## Round structure
Each major round records:
- test suites executed,
- failures found,
- fixes applied,
- retest results,
- open questions.

See `TESTING_ROUNDS.md`.

## Baseline rounds (completed minimum set)
1. Workspace baseline + full test run.
2. File parser malformed corpus expansion.
3. Formula parser fuzzing and panic hardening.
4. Engine property tests (staleness/cycles/bounds).
5. TUI scripted behavior matrix.
6. TUI command parser fault injection.
7. File round-trip and cross-mode persistence stress.
8. Randomized end-to-end action fuzzing in script runner.
9. Broken-file CLI/TUI load handling and recovery.
10. Coverage-driven gap closure and final regression run.

## Execution status
- All 10 planned rounds have been executed in this iteration.
- Additional hardening rounds (11-25) were executed for terminal behavior, dynamic arrays, TUI UX, function-surface expansion, release packaging, named-formula support, formatting/selection support, clipboard/paste-special UX, lambda/indirect function-surface expansion, and R1C1/dynamic-map screenshot+semantics polish.
- Detailed evidence and outcomes are recorded in `TESTING_ROUNDS.md`.

## Current cross-doc test contract
- Core behavior contract: `../SPEC_v0.md` and `../ENGINE_REQUIREMENTS.md`.
- File-format contract: `../FILE_FORMAT.md`.
- TUI automation contract: `../TUI_TESTABILITY.md`.

## Next planned rounds (verification against implemented expanded scope)
1. TUI capture contract tests:
   - verify width/height selection, full-color span payloads, and cursor metadata.
2. Keystroke script determinism tests:
   - verify identical scripts reproduce identical timeline hashes.
3. Frame image export integrity tests:
   - verify SVG exports are stable for fixed inputs and viewport sizes.
4. CLI playback viewer behavior tests:
   - play/pause/frame-step/speed and keystroke-overlay behavior.
5. File format v2 compatibility tests:
   - expand corpus for `ITER`/`DYNARR`/`CONTROL`/`CHART` edge cases and v1-read compatibility.

## Platform harness extension (Windows Terminal)
- Add and maintain an interactive E2E harness for terminal event anomalies:
  - `scripts/windows/repro_double_keypress.ps1`
  - `scripts/windows/send_keys.ps1`
  - `docs/testing/WINDOWS_TERMINAL_KEY_REPRO.md`
- Purpose:
  - capture raw key event streams (`Press`/`Release`/`Repeat`) alongside mapped actions,
  - reproduce environment-specific key issues before changing input logic,
  - keep a reusable route for future UI regression and terminal-compat testing.
