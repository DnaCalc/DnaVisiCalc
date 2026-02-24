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
