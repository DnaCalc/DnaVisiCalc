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

## Round 7
- Status: completed
- Scope: file round-trip stress and malformed-input fuzzing.
- Suites:
  - `cargo test -p dnavisicalc-file --test roundtrip_prop`
- Result:
  - Randomized workbook snapshots round-tripped successfully.
  - Random text inputs did not panic the loader.
- Fixes:
  - Added property-based round-trip tests for mixed numeric/formula cells and mode persistence.
  - Added random malformed-text fuzz suite for file parser panic resistance.
- Open questions:
  - Should numeric serialization be canonicalized (fixed precision/format) for stricter cross-platform diff stability?

## Round 8
- Status: completed
- Scope: end-to-end TUI action fuzzing.
- Suites:
  - `cargo test -p dnavisicalc-tui --test action_fuzz_prop`
- Result:
  - Random action sequences maintained in-bounds cursor invariants and did not panic.
- Fixes:
  - Added randomized UI action sequence fuzz tests over mixed navigation/edit/command actions.
- Open questions:
  - Should quit actions be ignored while in edit mode unless explicitly confirmed?

## Round 9
- Status: completed
- Scope: broken-file handling and recovery in UI flows.
- Suites:
  - `cargo test -p dnavisicalc-tui --test broken_file_recovery`
- Result:
  - Opening malformed or missing files is non-fatal and app remains operational.
- Fixes:
  - Added explicit recovery tests for malformed file open paths.
  - Added runtime entrypoint test seam (`run_with_options`) and binary smoke test with test-exit flag.
- Open questions:
  - Should open failures preserve previous status message history for easier user debugging?

## Round 10
- Status: completed
- Scope: coverage-driven gap closure.
- Suites:
  - `cargo llvm-cov --workspace --all-targets --summary-only`
  - plus targeted matrix tests across core/file/tui modules.
- Result:
  - Coverage improved to:
    - Regions: **88.38%**
    - Lines: **88.99%**
    - Functions: **89.27%**
  - 100% was not reached in this pass.
- Fixes:
  - Added address, engine API, eval matrix, file error matrix, keymap matrix, IO filesystem, and command-matrix tests.
  - Added runtime test seams and binary startup smoke coverage.
  - Added coverage summary doc: `docs/testing/COVERAGE_SUMMARY.md`.
- Open questions:
  - Is it worth introducing a pseudo-terminal integration harness to close runtime coverage gaps?
  - Should round-trip numeric serialization be canonicalized for deterministic textual diffs across platforms?

## Round 11
- Status: completed
- Scope: reproduce and scaffold terminal key duplication behavior (Windows Terminal path).
- Suites:
  - `cargo test --workspace`
  - targeted key mapping repro test in `keymap.rs`
  - Windows harness script dry-run:
    - `scripts/windows/repro_double_keypress.ps1`
- Result:
  - Reproduction scaffold added in code: `char_press_and_release_both_map_to_input_char_in_edit_mode`.
  - Runtime key-event tracing added (`DNAVISICALC_EVENT_TRACE`).
  - Windows Terminal + SendKeys automation harness added for high-level repro capture.
- Fixes:
  - No behavior fix applied yet (repro-first rule).
  - Added trace and automation infrastructure for future terminal regressions.
- Open questions:
  - In this CI/sandbox context, foreground window activation for SendKeys may fail (non-interactive desktop characteristics).
  - Should we add a ConPTY-driven harness to avoid foreground-window dependence?
