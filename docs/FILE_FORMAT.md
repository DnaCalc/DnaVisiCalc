# DVISICALC File Format v1

This repository uses a deterministic text format for workbook persistence.

## Header
First non-empty, non-comment line must be:

```text
DVISICALC	1
```

## Records
Each record is tab-separated.

### Mode
```text
MODE	AUTO
MODE	MANUAL
```

### Cell
```text
CELL	<A1>	N	<number>
CELL	<A1>	F	<formula>
```

Where:
- `N` means numeric literal.
- `F` means formula source text.
- Formula fields use escape sequences:
  - `\\` for backslash
  - `\t` for tab
  - `\n` for newline
  - `\r` for carriage return

## Validation rules
- Header required.
- At most one `MODE` record.
- Duplicate `CELL` addresses are rejected.
- Unknown record kinds are rejected.
- Unknown `CELL` type tags are rejected.
- Numeric parse must succeed for `N` records.

## Design intent
- Easy to inspect and diff in source control.
- Explicit errors with line numbers.
- Stable, deterministic round-trip behavior.