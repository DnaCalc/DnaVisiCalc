# DNA VisiCalc Spec v0 (Expanded Pathfinder Scope)

## 1. Purpose
This document defines the current Round-0 pathfinder scope for this repository.

It is the top-priority repo-local spec for behavior scope and intended direction. Engine details are refined in `docs/ENGINE_REQUIREMENTS.md`.

## 2. Repository Scope
This repo contains four crates with explicit boundaries:
- `dnavisicalc-core`: deterministic spreadsheet engine (library-only, no file/network/UI dependency).
- `dnavisicalc-engine`: backend boundary/loader for selecting the active core engine implementation.
- `dnavisicalc-file`: deterministic serialization adapter.
- `dnavisicalc-tui`: terminal interaction layer and automation/test harness seams.

## 3. Normative v0 Requirements

### 3.1 Engine Boundary
- Pure in-memory API.
- Externally-driven execution only (no internal timers/threads/I/O).
- Deterministic behavior for identical inputs and operation sequences.

### 3.2 Sheet and Address Model
- Single worksheet.
- VisiCalc-size bounds by default: `A1..BK254` (63 columns x 254 rows).
- A1 parsing supports:
  - relative refs (`A1`),
  - mixed absolute refs (`$A1`, `A$1`),
  - fully absolute refs (`$A$1`),
  - ranges (`A1:B7`, `A1...B7`).

### 3.3 Cell and Name Inputs
- Cells and names accept numeric, text, formula, and clear operations.
- Names are case-insensitive, normalized uppercase, and validated against identifier/function/cell-ref conflicts.

### 3.4 Formula Surface
- Arithmetic, comparison, concatenation, and logical evaluation.
- Dynamic arrays and spill semantics including spill references (`A1#`).
- Functional surface includes aggregates, logical, math/trig, financial, text, error helpers, lambda family (`LET`/`LAMBDA`/`MAP`), and reference helpers (`INDIRECT`, `OFFSET`, `ROW`, `COLUMN`).
- `INDIRECT` supports both A1 and R1C1 text references.

### 3.5 Recalc and Epoch Model
- `committed_epoch`, `stabilized_epoch`, and per-value `value_epoch`.
- `Automatic` and `Manual` recalc modes.
- Incremental dirty-closure recomputation for value-only mutations with deterministic fallback when structure changes.

### 3.6 Structural Rewrite Path (Required)
- Row/column structural mutations are in-scope:
  - `insert_row`, `delete_row`, `insert_col`, `delete_col`.
- Formula/name references are rewritten deterministically.
- Invalidated references are surfaced explicitly (for example `#REF!` behavior).
- Mixed and absolute references must preserve anchoring flags through rewrites.
- Structural mutation requests use a tri-state outcome model:
  - `Applied`: mutation accepted and committed.
  - `Rejected`: request is valid but cannot be executed due to structural/policy constraints.
  - `Invalid`: request is malformed or out of contract.
- Rejected structural requests are atomic no-ops:
  - no partial mutation,
  - no `committed_epoch` increment,
  - deterministic, user-visible rejection reason.

### 3.7 Iteration and Cycle Handling
- SCC cycle detection remains deterministic.
- Engine supports iterative cycle mode via iteration configuration (`enabled`, max iterations, convergence tolerance).

### 3.8 Volatility and Invalidation Classes
- Functions/UDFs are classified as:
  - `Standard`,
  - `Volatile`,
  - `ExternallyInvalidated`.
- Volatile refresh and externally-triggered refresh are separate pathways (`invalidate_volatile`, stream ticks, `invalidate_udf`).

### 3.9 External UDFs
- External UDF registration/unregistration is in scope.
- UDF volatility class participates in invalidation behavior.

### 3.10 Engine Entities Beyond Cells/Names
- Controls are engine-managed named-value entities with metadata.
- Charts are engine-managed sink entities producing computed chart outputs.
- Change tracking is engine-managed via opt-in journal/drain API.

### 3.11 Formatting
- Per-cell metadata formatting is in scope:
  - decimals,
  - bold/italic,
  - foreground/background palette colors.
- Formatting does not change formula semantics.

### 3.12 File Adapter Scope
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

### 3.13 TUI Scope
- Grid navigation, editing, command mode, clipboard/paste-special, formatting, and help surfaces are in scope.
- Command surface includes structural operations (`insrow`/`delrow`/`inscol`/`delcol` aliases).
- Status presentation distinguishes rejected-valid commands from malformed input/usage errors.
- TUI tool-driving automation includes fixed-size frame capture with cursor/style metadata, keystroke-driven script capture, and CLI replay/viewer flow (`docs/TUI_TESTABILITY.md`).

## 4. Acceptance Criteria
- `cargo test --workspace` passes.
- Deterministic behavior and structural rewrite semantics are test-covered.
- Engine/file/TUI contracts remain aligned across:
  - `docs/SPEC_v0.md`,
  - `docs/ENGINE_REQUIREMENTS.md`,
  - `docs/FILE_FORMAT.md`,
  - `docs/TUI_TESTABILITY.md`.

## 5. Non-goals (Round 0)
- Multi-sheet workbook semantics.
- OOXML fidelity and full Excel object model compatibility.
- Collaboration/replication protocols.
- VBA runtime hosting.
- Full XLL/COM parity.

## 6. References
- `docs/ENGINE_REQUIREMENTS.md`
- `docs/ARCHITECTURE.md`
- `docs/FILE_FORMAT.md`
- `docs/TUI_TESTABILITY.md`
- `docs/testing/TESTING_PLAN.md`
