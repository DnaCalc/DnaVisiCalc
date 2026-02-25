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

## Round 12
- Status: completed
- Scope: key-input duplication fix and resize-aware TUI behavior.
- Suites:
  - `cargo test -p dnavisicalc-tui`
  - `cargo test --workspace`
- Result:
  - Key release events no longer trigger input actions.
  - Grid viewport now adapts to terminal size and resize events.
- Fixes:
  - Key-event filtering to actionable kinds (`Press`/`Repeat`).
  - Dynamic grid dimension computation + viewport tracking in app/runtime.
- Open questions:
  - Whether to add configurable minimum column width for very small terminals.

## Round 13
- Status: completed
- Scope: dynamic array support in core and spill affordances in TUI.
- Suites:
  - `cargo test -p dnavisicalc-core`
  - `cargo test -p dnavisicalc-tui`
  - `cargo test --workspace`
- Result:
  - Added dynamic arrays with spill placement and conflict handling.
  - Added `SEQUENCE`, `RANDARRAY`, and spill-reference parsing/evaluation (`A1#`).
  - Added TUI spill role visuals and edit constraints for spill children.
- Fixes:
  - Added checked overflow handling for long column labels in address parsing.
  - Added dedicated dynamic-array and spill UX test coverage.
- Open questions:
  - How closely should broadcasting and volatile recalc behavior track Excel edge semantics in later profiles?

## Round 14
- Status: completed
- Scope: spill-interior dereference correctness fix.
- Suites:
  - `cargo test -p dnavisicalc-core`
  - `cargo test --workspace`
- Result:
  - Direct references and range aggregation over spill-interior cells now evaluate correctly.
- Fixes:
  - Evaluator now resolves spill-interior cell values during formula evaluation by deriving active spill ranges from evaluated anchors.
  - Added explicit regression tests for direct and range-based spill-interior references.
- Open questions:
  - Whether to evolve dependency extraction to model spill-derived dependencies explicitly for diagnostics.

## Round 15
- Status: completed
- Scope: dynamic-array fuzz/hardening pass.
- Suites:
  - `cargo test -p dnavisicalc-core --test dynamic_array_fuzz_prop`
  - `cargo test --workspace`
- Result:
  - Added property-based tests for:
    - interior spill-cell references,
    - `SUM(A1#)` arithmetic consistency,
    - blocked spill non-overwrite guarantees,
    - `RANDARRAY` bounds/integer guarantees.
  - One edge mismatch found and corrected in tests:
    - `A1#` is invalid for non-spilled 1x1 results (aligned assumptions).
- Fixes:
  - Added new fuzz suite: `crates/dnavisicalc-core/tests/dynamic_array_fuzz_prop.rs`.
- Open questions:
  - Should we introduce profile flags for strict Excel volatility semantics on dynamic-array functions?

## Round 16
- Status: completed
- Scope: enforce dynamic-array parity + fuzz across all three strategy implementations.
- Suites:
  - `cargo test -p dnavisicalc-core --test dynamic_array_strategy_parity`
  - `cargo test -p dnavisicalc-core --test dynamic_array_fuzz_prop`
  - `cargo test --workspace`
- Result:
  - Dynamic-array fuzz properties now execute under:
    - `OverlayInline`,
    - `OverlayPlanner`,
    - `RewriteMaterialize`.
  - Added deterministic cross-strategy property assertions to detect parity drift directly.
  - Full workspace remains green.
- Fixes:
  - Expanded `dynamic_array_fuzz_prop` to multi-strategy execution.
  - Added deterministic parity property over value grid + spill metadata.
- Open questions:
  - Should CI split strategy parity tests into a dedicated fast lane (required) and a longer nightly stress lane?

## Round 17
- Status: completed
- Scope: TUI surface friendliness pass (help UX, status persistence, file/save context).
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test -p dnavisicalc-tui`
  - `cargo test -p dnavisicalc-tui --lib`
  - `cargo test -p dnavisicalc-core`
  - `cargo test -p dnavisicalc-file`
- Result:
  - Added inline key hints and full help popup (`?`/`F1`) in navigate mode.
  - Added workbook header with file path, save-state indicator, and recalc mode.
  - Status messages now persist while navigating (save/open feedback no longer immediately overwritten).
  - Added tests for help toggling, save-state transitions, and status persistence after save.
- Fixes:
  - Mapped `?` and `F1` to help toggle in keymap.
  - Added save-state tracking via committed-epoch snapshots at save/open.
  - Updated action fuzz to include help-toggle action.
- Notes:
  - Used alternate `CARGO_TARGET_DIR` to avoid Windows lock from a concurrently running interactive binary.
- Open questions:
  - Should full-help popup include a compact command examples section with live current path substitution?

## Round 18
- Status: completed
- Scope: VisiCalc function-surface expansion + help-panel function listing + release hardening.
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test -p dnavisicalc-core`
  - `CARGO_TARGET_DIR=target_tmp cargo test -p dnavisicalc-file`
  - `CARGO_TARGET_DIR=target_tmp cargo test -p dnavisicalc-tui`
  - `CARGO_TARGET_DIR=target_tmp cargo test --workspace`
- Result:
  - Expanded core function coverage with math, trig, financial, lookup, and error helpers.
  - Help popup now renders the supported-function list from core metadata.
  - Added compatibility classification document for problematic/not-yet-implemented areas.
  - Captured reproducible README screenshot artifacts via scripted scene capture.
- Fixes:
  - Added parser lookahead regression fix for function names resembling cell refs (`LOG10(...)`).
  - Added regression tests for new functions and help-popup list rendering.
- Open questions:
  - Exact historical VisiCalc coercion/error text behavior should be validated against additional public evidence fixtures.

## Round 19
- Status: completed
- Scope: Windows Rust-free release verification, README screenshot/structure refresh, and release artifact packaging for v0.1.1.
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test --workspace`
  - `cargo run -p dnavisicalc-tui --bin capture_scenes`
  - `powershell -ExecutionPolicy Bypass -File scripts/windows/render_scene_pngs.ps1`
  - `powershell -ExecutionPolicy Bypass -File scripts/windows/build_release_v0.1.1.ps1`
- Result:
  - Verified full workspace test pass before packaging.
  - Added numerical-model screenshot scene and regenerated README screenshots.
  - Produced release zip `dnavisicalc-v0.1.1-windows-x64.zip` containing runnable app + help docs.
  - Confirmed packaged runtime flow does not require Rust/Cargo for end users.
- Fixes:
  - Added dedicated v0.1.1 release script and release docs with explicit no-Rust runtime instructions.
  - Reordered README sections to make screenshots first and move layering/layout story to the end.
- Open questions:
  - Should future release scripts be consolidated into a single parameterized script to avoid per-version duplication?

## Round 20
- Status: completed
- Scope: release packaging correction to remove launcher-induced working-directory change and publish v0.1.2 artifact.
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test --workspace`
  - `powershell -ExecutionPolicy Bypass -File scripts/windows/build_release_v0.1.2.ps1`
- Result:
  - Verified full workspace remains green.
  - Produced release zip `dnavisicalc-v0.1.2-windows-x64.zip` containing only direct runtime entry (`dnavisicalc.exe`) and docs.
  - Removed `run_dnavisicalc.bat` from packaged assets and release instructions.
- Fixes:
  - Added v0.1.2 packaging script and release docs aligned to direct executable launch behavior.
  - Updated README release section to v0.1.2 and direct executable launch instructions.
- Open questions:
  - Whether to replace versioned release scripts with one parameterized script before v0.2.0.

## Round 21
- Status: completed
- Scope: workbook names feature (named values/formulas) across parser, evaluator, engine, file format, and TUI command layer.
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test --workspace`
- Result:
  - Added workbook-level names that can hold numeric/text/formula inputs.
  - Names can be referenced from formulas and can depend on cells and other names.
  - Name cycles now resolve to explicit evaluation errors (non-panic) in dependent formulas.
  - File format now persists names with deterministic `NAME` records.
  - TUI command mode now supports `name <NAME> <value|formula>` and `name clear <NAME>`.
- Fixes:
  - Added parser support for bare identifier name references (including leading `_` names).
  - Added name validation/normalization rules and collision checks (cell refs and built-in function names are rejected).
  - Added end-to-end tests for core eval, serialization, and command behavior.
- Open questions:
  - If needed later, add a dedicated dependency-plan representation that includes names explicitly (not just runtime name evaluation with memoization/cycle detection).

## Round 22
- Status: completed
- Scope: minimal cell formatting + multi-cell selection and delete/format flows.
- Suites:
  - `CARGO_TARGET_DIR=target_tmp cargo test --workspace`
- Result:
  - Added per-cell formatting metadata (decimals, text bold/italic, fg/bg palette) with deterministic save/load support.
  - Added multi-cell selection extension via `Shift+Arrows` / `Shift+H/J/K/L`.
  - Added range clear behavior via `Delete` in navigation mode.
  - Added formatting commands that apply over current selection.
  - Verified workspace remains fully green after feature integration and UI help updates.
- Fixes:
  - Extended key mapping and app action model with selection-extension and clear-selection actions.
  - Added format persistence records (`FMT`) in DVISICALC file format and strict validation.
  - Updated render styling with a 16-color nature-soft palette and selection-aware highlighting.
- Open questions:
  - Whether to add an explicit style-preview panel for the selected range in a future UX pass.

## Round 23
- Status: completed
- Scope: keyboard and clipboard UX expansion (`F2` editing + system clipboard copy/paste + paste-type picker).
- Suites:
  - `cargo test -p dnavisicalc-tui`
  - `cargo test --workspace`
- Result:
  - Added `F2` as a direct start-edit shortcut in navigation mode.
  - Added `Ctrl+C`/`Ctrl+V` system clipboard integration in runtime.
  - Added Paste Special mode selector with types:
    - All
    - Formulas
    - Values
    - Values+KeepDestFmt
    - Formatting
  - Added dedicated keymap handling for paste-special mode (`Tab`, arrows, `j/k`, `1..5`, `Enter`, `Esc`).
  - Updated help and quick-reference docs for the new shortcuts and paste-type flow.
- Fixes:
  - Added app-level paste-special regression tests covering formula/value/format behavior.
  - Added runtime clipboard-bridge unit tests with deterministic mock clipboard coverage.
  - Added keymap matrix coverage for `F2`, clipboard shortcuts, and paste-special controls.
- Open questions:
  - Whether to add block replication (tiling) when pasting a smaller source range into a larger selected destination range.

## Round 24
- Status: completed
- Scope: Excel-style formula-surface expansion (`LET`/`LAMBDA`/`MAP` and `INDIRECT`/`OFFSET`/`ROW`/`COLUMN`).
- Suites:
  - `cargo test -p dnavisicalc-core`
  - `cargo test --workspace`
- Result:
  - Added lexical formula scope with runtime closures for `LET` and `LAMBDA`.
  - Added lambda invocation support in formula context and element-wise lambda evaluation via `MAP`.
  - Added reference helper functions `INDIRECT`, `OFFSET`, `ROW`, and `COLUMN`.
  - Added deterministic behavior and bounds/error handling for dynamic reference resolution.
  - Extended parser/evaluator test coverage with new functional and reference scenarios.
- Fixes:
  - Refactored evaluator runtime model to carry scoped local bindings in addition to workbook names.
  - Added runtime `Lambda` value representation and closure capture semantics.
  - Added regression tests for valid and invalid LET/LAMBDA/MAP and reference-helper usage.
- Open questions:
  - Whether to add full Excel-style `R1C1` handling for `INDIRECT` second argument in a future compatibility pass.

## Round 25
- Status: completed
- Scope: edge-semantics completion for `INDIRECT` R1C1 mode and array-returning `MAP` lambdas, plus expanded README screenshot showcase.
- Suites:
  - `cargo test -p dnavisicalc-core`
  - `cargo run -p dnavisicalc-tui --bin capture_scenes`
  - `powershell -ExecutionPolicy Bypass -File scripts/windows/render_scene_pngs.ps1`
  - `cargo test --workspace`
- Result:
  - `INDIRECT(text,FALSE)` now supports R1C1 references (absolute, relative, mixed, range, spill anchor with `#`).
  - `MAP` now supports scalar and array-returning lambda outputs with deterministic broadcast+tile shaping.
  - Added evaluator regression tests for R1C1 references and MAP array-return behavior.
  - Added six new polished screenshot scenes and refreshed README screenshot coverage.
- Fixes:
  - Added R1C1 parser helpers in evaluator with explicit context/bounds error handling.
  - Refactored MAP output assembly to compose block outputs from per-item lambda runtime values.
  - Improved screenshot renderer theming and per-line/per-column color treatment for richer README visuals.
- Open questions:
  - Whether to add multi-dimensional map combinators (`BYROW`/`BYCOL`/`REDUCE`/`SCAN`) in a dedicated follow-on pass.
