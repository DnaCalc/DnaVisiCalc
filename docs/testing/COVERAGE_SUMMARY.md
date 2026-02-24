# Coverage Summary

## Tooling
- Command: `cargo llvm-cov --workspace --all-targets --summary-only`
- Date: 2026-02-24

## Current result
- Total regions: **88.38%**
- Total lines: **88.99%**
- Total functions: **89.27%**

## Highest remaining gaps
- `dnavisicalc-tui/src/runtime.rs`
  - Real terminal loop paths are hard to execute in deterministic CI-style tests without pseudo-terminal event injection.
- `dnavisicalc-core/src/eval.rs`
  - Additional branches remain around less-common coercion/error combinations.
- `dnavisicalc-file/src/lib.rs`
  - More malformed escape/record ordering cases can be added.

## Why not 100% yet
- Some runtime/terminal plumbing remains intentionally thin and integration-heavy.
- Achieving 100% would require either:
  1. deeper terminal-driver simulation scaffolding, or
  2. architecture changes to isolate every side-effect line behind unit-testable interfaces.

## Practical quality assessment
- Core logic, parser, file format validation, and TUI state machine are now strongly covered.
- Fuzz/property suites are running across parser, file parser, command parsing, and action sequences.
- Broken-file recovery behavior is explicitly tested and non-fatal.