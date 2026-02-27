# DVISICALC File Format

This document specifies the deterministic text format used by `dnavisicalc-file`.

## 1. Versioning

Current writer version:
- `DVISICALC v2`

Reader compatibility:
- accepts `DVISICALC v1` and `DVISICALC v2`.

Header line (first non-empty, non-comment line):

```text
DVISICALC	2
```

(`DVISICALC\t1` remains accepted on load.)

## 2. Record Model (v2)

Each record is tab-separated.

### 2.1 Mode

```text
MODE	AUTO
MODE	MANUAL
```

### 2.2 Iteration Config

```text
ITER	<enabled:0|1>	<max_iterations>	<convergence_tolerance>
```

### 2.3 Dynamic Array Strategy

```text
DYNARR	<OVERLAY_INLINE|OVERLAY_PLANNER|REWRITE_MATERIALIZE>
```

### 2.4 Cell Input

```text
CELL	<A1>	N	<number>
CELL	<A1>	T	<text>
CELL	<A1>	F	<formula>
```

### 2.5 Name Input

```text
NAME	<IDENT>	N	<number>
NAME	<IDENT>	T	<text>
NAME	<IDENT>	F	<formula>
```

### 2.6 Controls

```text
CONTROL	<NAME>	<SLIDER|CHECKBOX|BUTTON>	<min>	<max>	<step>
```

Control value is persisted through the corresponding `NAME` record.

### 2.7 Charts

```text
CHART	<NAME>	<START_A1>	<END_A1>
```

Chart outputs are derived on recalculation and are not persisted.

### 2.8 Cell Format

```text
FMT	<A1>	<decimals|->	<bold:0|1>	<italic:0|1>	<fg|->	<bg|->
```

Palette names:
- `MIST`, `SAGE`, `FERN`, `MOSS`, `OLIVE`, `SEAFOAM`, `LAGOON`, `TEAL`
- `SKY`, `CLOUD`, `SAND`, `CLAY`, `PEACH`, `ROSE`, `LAVENDER`, `SLATE`

## 3. Escaping Rules

Text and formula fields use escapes:
- `\\` backslash
- `\t` tab
- `\n` newline
- `\r` carriage return

Unknown escapes are parse errors.

## 4. Parsing Rules

- Blank lines are ignored.
- Lines whose trimmed form starts with `#` are ignored.
- Record fields are split on literal tab characters.
- Record order is flexible.
- Unknown record kinds are rejected.
- `v1` documents reject `v2`-only record kinds (`ITER`, `DYNARR`, `CONTROL`, `CHART`).

## 5. Validation Rules

- Header is required.
- Supported versions are `1..=2`.
- At most one `MODE` record.
- At most one `ITER` record.
- At most one `DYNARR` record.
- Duplicate `CELL` addresses are rejected.
- Duplicate `NAME` identifiers are rejected.
- Duplicate `CONTROL` names are rejected.
- Duplicate `CHART` names are rejected.
- Duplicate `FMT` addresses are rejected.
- `CELL`/`NAME` type tags must be `N`, `T`, or `F`.
- Numeric parsing for numeric fields must succeed.
- `FMT.decimals` is `-` or `0..9`.
- `FMT.bold`/`FMT.italic` are `0` or `1`.
- `FMT.fg`/`FMT.bg` are `-` or known palette names.

## 6. Save/Load Semantics

### 6.1 Deterministic save order
`save_to_string` writes:
1. Header
2. `MODE`
3. `ITER`
4. `DYNARR`
5. all `CELL` records in deterministic engine enumeration order
6. all `NAME` records in deterministic engine enumeration order
7. all `CONTROL` records in deterministic engine enumeration order
8. all `CHART` records in deterministic engine enumeration order
9. all `FMT` records in deterministic engine enumeration order

### 6.2 Load/apply flow
`load_from_str`:
1. Parses and validates all records.
2. Creates a new engine.
3. Forces manual mode during apply.
4. Applies iteration and dynamic-array config.
5. Applies cells, names, formats, controls, and charts.
6. Recalculates once.
7. Restores persisted recalc mode.

## 7. Persisted Scope and Explicit Omissions

Persisted today:
- recalc mode
- iteration config
- dynamic-array strategy
- cell inputs
- name inputs
- control definitions
- chart definitions
- cell formats

Not persisted:
- chart outputs
- change journal state
- UDF registrations
- structural operation history/oplog

## 8. Design Intent
- Human-inspectable and source-control diffable.
- Strict and line-specific parse diagnostics.
- Deterministic round-trip behavior for the persisted surface.
- Backward read compatibility for prior version (`v1`).
