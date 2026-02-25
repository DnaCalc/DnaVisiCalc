# Testing Plan (Living)

## Primary goals
1. Harden core correctness and parser robustness.
2. Harden file parsing/writing against malformed inputs.
3. Harden TUI behavior through deterministic scripted tests.
4. Expand fuzz/property tests for panic resistance and edge cases.
5. Measure and improve line/branch coverage.

## Round structure
Each major round records:
- test suites executed,
- failures found,
- fixes applied,
- retest results,
- open questions.

See `docs/testing/TESTING_ROUNDS.md`.

## Planned deep rounds (minimum 10)
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
- Additional hardening rounds (11-24) were executed for terminal behavior, dynamic arrays, TUI UX, function-surface expansion, release packaging, named-formula support, formatting/selection support, clipboard/paste-special UX, and lambda/indirect function-surface expansion.
- Detailed evidence and outcomes are recorded in `docs/testing/TESTING_ROUNDS.md`.

## Platform harness extension (Windows Terminal)
- Add and maintain an interactive E2E harness for terminal event anomalies:
  - `scripts/windows/repro_double_keypress.ps1`
  - `scripts/windows/send_keys.ps1`
  - `docs/testing/WINDOWS_TERMINAL_KEY_REPRO.md`
- Purpose:
  - capture raw key event streams (`Press`/`Release`/`Repeat`) alongside mapped actions,
  - reproduce environment-specific key issues before changing input logic,
  - keep a reusable route for future UI regression and terminal-compat testing.
