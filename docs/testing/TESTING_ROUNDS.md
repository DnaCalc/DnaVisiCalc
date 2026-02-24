# Testing Rounds Log

## Round 1
- Status: completed
- Scope: workspace baseline compile/test after layering refactor.
- Suites:
  - `cargo test --workspace`
- Result:
  - All tests passing.
- Fixes:
  - N/A (baseline validation pass).
- Open questions:
  - How strict should formula compatibility be for malformed-but-common user inputs?

## Round 2
- Status: completed
- Scope: file parser robustness and diagnostics hardening.
- Suites:
  - `cargo test -p dnavisicalc-file`
- Result:
  - Added malformed and edge-case coverage for file loader.
  - All file tests passing.
- Fixes:
  - Added UTF-8 BOM tolerant header parsing.
  - Upgraded load failures to line-specific parse diagnostics for cell-application failures.
- Open questions:
  - Should duplicate cell records be reject-only (current) or last-write-wins?

## Round 3
- Status: completed
- Scope: formula/parser fuzz hardening.
- Suites:
  - `cargo test -p dnavisicalc-core`
  - Included new property tests (`parser_fuzz_prop`).
- Result:
  - No panics found under randomized formula-like fuzz inputs.
  - All core tests passing.
- Fixes:
  - Added parser and setter panic-resistance property suites.
  - Added deep parenthesis stress test.
- Open questions:
  - Should parser include an explicit nesting-depth guard to bound pathological formulas?

## Round 4
- Status: completed
- Scope: core engine property/invariant testing.
- Suites:
  - `cargo test -p dnavisicalc-core`
  - Included new `engine_prop_tests`.
- Result:
  - Manual recalc, deterministic ordering, and serialization stability invariants validated under property tests.
- Fixes:
  - Added property tests for stale/recalc behavior and set-order determinism.
  - Added stable-sort coverage for `all_cell_inputs`.
- Open questions:
  - Should deterministic ordering be explicitly specified for equal-priority graph nodes beyond current sorted-key behavior?

## Round 5
- Status: completed
- Scope: TUI scripted behavior matrix.
- Suites:
  - `cargo test -p dnavisicalc-tui --test app_matrix_tests`
- Result:
  - Command, edit, navigation, and save/open flows verified via script-driven tests.
- Fixes:
  - Added matrix tests and clarified edit behavior during test driving.
  - Added in-memory file insertion API to strengthen adapter testing.
- Open questions:
  - Should edit mode default to replacement or append semantics when a cell already has content?

## Round 6
- Status: completed
- Scope: TUI command fuzz hardening.
- Suites:
  - `cargo test -p dnavisicalc-tui --test command_fuzz_prop`
- Result:
  - Random command streams did not panic and returned stable outcomes (`Continue`/`Quit`).
- Fixes:
  - Added property-based command-input fuzz suite.
- Open questions:
  - Should command parsing adopt quoted argument support for paths with spaces?
