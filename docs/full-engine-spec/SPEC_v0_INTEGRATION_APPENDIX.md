# SPEC v0 Integration Appendix

This appendix keeps repository-specific integration scope that was split out of `docs/SPEC_v0.md` to keep the core spec engine-only.

## 1. Repository Layer Scope
This repo contains four crates with explicit boundaries:
- `dnavisicalc-core`: deterministic spreadsheet engine (library-only, no file/network/UI dependency).
- `dnavisicalc-engine`: backend boundary/loader for selecting the active core engine implementation.
- `dnavisicalc-file`: deterministic serialization adapter.
- `dnavisicalc-tui`: terminal interaction layer and automation/test harness seams.

## 2. File Adapter Scope
- `DVISICALC v2` persists:
  - recalc mode,
  - iteration config,
  - dynamic-array strategy,
  - cell inputs,
  - name inputs,
  - control definitions,
  - chart definitions,
  - cell formats.
- Loader accepts both `DVISICALC v1` and `DVISICALC v2`.
- Loader applies strict validation with line-specific errors, recalculates once after apply, and restores persisted recalc mode.

## 3. TUI Scope
- Grid navigation, editing, command mode, clipboard/paste-special, formatting, and help surfaces are in scope.
- `F9` forces recalculation in any UI mode (equivalent to `:r` / `:recalc`).
- Command surface includes structural operations (`insrow`/`delrow`/`inscol`/`delcol` aliases).
- Status presentation distinguishes rejected-valid commands from malformed input/usage errors.
- TUI tool-driving automation includes fixed-size frame capture with cursor/style metadata, keystroke-driven script capture, and CLI replay/viewer flow (`docs/TUI_TESTABILITY.md`).

## 4. Repository Acceptance Criteria
- `cargo test --workspace` passes.
- Deterministic behavior and structural rewrite semantics are test-covered.
- Engine/file/TUI contracts remain aligned across:
  - `docs/SPEC_v0.md`,
  - `docs/ENGINE_REQUIREMENTS.md`,
  - `docs/FILE_FORMAT.md`,
  - `docs/TUI_TESTABILITY.md`.

## 5. Related References
- `docs/ARCHITECTURE.md`
- `docs/FILE_FORMAT.md`
- `docs/TUI_TESTABILITY.md`
- `docs/testing/TESTING_PLAN.md`
