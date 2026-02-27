# Engine C API Specification — DNA VisiCalc

Complete C-style API specification for the `dnavisicalc-core` engine. Prefix: **`dvc_`** (DNA VisiCalc).

This document specifies the public interface. It does not implement the interface — that is the future `dnavisicalc-cabi` crate. The specification is derived from the current Rust `Engine` API and the requirements in [ENGINE_REQUIREMENTS.md](ENGINE_REQUIREMENTS.md), following the patterns in [C_API_GUIDELINES.md](C_API_GUIDELINES.md).

Design notes for controls, charts, function classification, and change tracking: [ENGINE_DESIGN_NOTES.md](ENGINE_DESIGN_NOTES.md).

## 1. Type System

### 1.1 Opaque Handle

```c
typedef struct DvcEngine DvcEngine;
```

The engine handle. All state is contained within this handle. Multiple handles may coexist independently across threads, but a single handle must not be accessed concurrently without external synchronization.

### 1.2 Status Codes

```c
typedef int32_t DvcStatus;

#define DVC_OK                      0
#define DVC_REJECT_STRUCTURAL_CONSTRAINT  1
#define DVC_REJECT_POLICY                 2
#define DVC_ERR_NULL_POINTER       -1
#define DVC_ERR_OUT_OF_BOUNDS      -2
#define DVC_ERR_INVALID_ADDRESS    -3
#define DVC_ERR_PARSE              -4
#define DVC_ERR_DEPENDENCY         -5
#define DVC_ERR_INVALID_NAME       -6
#define DVC_ERR_OUT_OF_MEMORY      -7
#define DVC_ERR_INVALID_ARGUMENT   -8
```

`DVC_OK` indicates success and applied mutation/query completion.
Positive status codes indicate valid requests that were rejected (no mutation applied).
Negative status codes indicate hard API/validation/execution errors.

### 1.2.1 Reject Kinds

```c
typedef int32_t DvcRejectKind;

#define DVC_REJECT_KIND_NONE                   0
#define DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT  1
#define DVC_REJECT_KIND_POLICY                 2
```

Reject kinds provide stable machine-readable reason classes for positive (`DVC_REJECT_*`) status returns.

### 1.3 Value Types

```c
typedef int32_t DvcValueType;

#define DVC_VALUE_NUMBER    0
#define DVC_VALUE_TEXT      1
#define DVC_VALUE_BOOL      2
#define DVC_VALUE_BLANK     3
#define DVC_VALUE_ERROR     4
```

### 1.4 Cell Error Kinds

```c
typedef int32_t DvcCellErrorKind;

#define DVC_CELL_ERR_DIV_ZERO       0
#define DVC_CELL_ERR_VALUE          1
#define DVC_CELL_ERR_NAME           2
#define DVC_CELL_ERR_UNKNOWN_NAME   3
#define DVC_CELL_ERR_REF            4
#define DVC_CELL_ERR_SPILL          5
#define DVC_CELL_ERR_CYCLE          6
#define DVC_CELL_ERR_NA             7
#define DVC_CELL_ERR_NULL           8
#define DVC_CELL_ERR_NUM            9
```

Maps directly to the Rust `CellError` enum variants.

### 1.5 Recalc Mode

```c
typedef int32_t DvcRecalcMode;

#define DVC_RECALC_AUTOMATIC  0
#define DVC_RECALC_MANUAL     1
```

### 1.6 Input Type

```c
typedef int32_t DvcInputType;

#define DVC_INPUT_EMPTY     0
#define DVC_INPUT_NUMBER    1
#define DVC_INPUT_TEXT      2
#define DVC_INPUT_FORMULA   3
```

### 1.7 Spill Role

```c
typedef int32_t DvcSpillRole;

#define DVC_SPILL_NONE      0
#define DVC_SPILL_ANCHOR    1
#define DVC_SPILL_MEMBER    2
```

### 1.8 Palette Color

```c
typedef int32_t DvcPaletteColor;

#define DVC_COLOR_NONE      -1   /* no color assigned */
#define DVC_COLOR_MIST       0
#define DVC_COLOR_SAGE       1
#define DVC_COLOR_FERN       2
#define DVC_COLOR_MOSS       3
#define DVC_COLOR_OLIVE      4
#define DVC_COLOR_SEAFOAM    5
#define DVC_COLOR_LAGOON     6
#define DVC_COLOR_TEAL       7
#define DVC_COLOR_SKY        8
#define DVC_COLOR_CLOUD      9
#define DVC_COLOR_SAND      10
#define DVC_COLOR_CLAY      11
#define DVC_COLOR_PEACH     12
#define DVC_COLOR_ROSE      13
#define DVC_COLOR_LAVENDER  14
#define DVC_COLOR_SLATE     15

#define DVC_PALETTE_COUNT   16
```

### 1.9 Data Structures

```c
typedef struct {
    uint16_t col;   /* 1-based column index */
    uint16_t row;   /* 1-based row index */
} DvcCellAddr;

typedef struct {
    DvcCellAddr start;
    DvcCellAddr end;
} DvcCellRange;

typedef struct {
    uint16_t max_columns;
    uint16_t max_rows;
} DvcSheetBounds;
```

### 1.10 Cell Value (output)

```c
typedef struct {
    DvcValueType     type;
    double           number;       /* valid when type == DVC_VALUE_NUMBER */
    int32_t          bool_val;     /* valid when type == DVC_VALUE_BOOL (0=false, 1=true) */
    DvcCellErrorKind error_kind;   /* valid when type == DVC_VALUE_ERROR */
} DvcCellValue;
```

Text values are retrieved separately via `dvc_cell_get_text` because they are variable-length. When `type == DVC_VALUE_TEXT`, the `number`, `bool_val`, and `error_kind` fields are unspecified.

### 1.11 Cell State (output)

```c
typedef struct {
    DvcCellValue value;
    uint64_t     value_epoch;
    int32_t      stale;   /* 0=current, 1=stale */
} DvcCellState;
```

### 1.12 Cell Format

```c
typedef struct {
    int32_t         has_decimals;   /* 0=auto, 1=explicit */
    uint8_t         decimals;       /* 0..9, valid when has_decimals==1 */
    int32_t         bold;           /* 0=off, 1=on */
    int32_t         italic;         /* 0=off, 1=on */
    DvcPaletteColor fg;             /* DVC_COLOR_NONE or 0..15 */
    DvcPaletteColor bg;             /* DVC_COLOR_NONE or 0..15 */
} DvcCellFormat;
```

### 1.13 Iterator Handles

```c
typedef struct DvcCellIterator DvcCellIterator;
typedef struct DvcNameIterator DvcNameIterator;
typedef struct DvcFormatIterator DvcFormatIterator;
typedef struct DvcControlIterator DvcControlIterator;
typedef struct DvcChartIterator DvcChartIterator;
typedef struct DvcChangeIterator DvcChangeIterator;
```

### 1.14 Control Kind

```c
typedef int32_t DvcControlKind;

#define DVC_CONTROL_SLIDER    0
#define DVC_CONTROL_CHECKBOX  1
#define DVC_CONTROL_BUTTON    2
```

### 1.15 Control Definition

```c
typedef struct {
    DvcControlKind kind;
    double         min;    /* Slider: minimum value (default 0.0) */
    double         max;    /* Slider: maximum value (default 100.0) */
    double         step;   /* Slider: increment step (default 1.0) */
} DvcControlDef;
```

For checkbox controls, `min`/`max`/`step` are ignored (value is always 0.0 or 1.0). For button controls, all three are ignored (value is always 0.0).

### 1.16 Chart Definition

```c
typedef struct {
    DvcCellRange source_range;
} DvcChartDef;
```

### 1.17 Chart Series Output

```c
typedef struct DvcChartOutput DvcChartOutput;
```

Opaque handle to a chart's computed output. The output is queried through accessor functions (series count, series name, series values).

### 1.18 Volatility Classification

```c
typedef int32_t DvcVolatility;

#define DVC_VOLATILITY_STANDARD                 0
#define DVC_VOLATILITY_VOLATILE                 1
#define DVC_VOLATILITY_EXTERNALLY_INVALIDATED   2
```

See [ENGINE_DESIGN_NOTES.md §4](ENGINE_DESIGN_NOTES.md#4-function-volatility-classification) for the three-category classification rationale.

### 1.19 Change Entry Type

```c
typedef int32_t DvcChangeType;

#define DVC_CHANGE_CELL_VALUE     0
#define DVC_CHANGE_NAME_VALUE     1
#define DVC_CHANGE_CHART_OUTPUT   2
#define DVC_CHANGE_SPILL_REGION   3
#define DVC_CHANGE_CELL_FORMAT    4
```

### 1.20 Iteration Config

```c
typedef struct {
    int32_t  enabled;                /* 0=disabled, 1=enabled */
    uint32_t max_iterations;         /* default: 100 */
    double   convergence_tolerance;  /* default: 0.001 */
} DvcIterationConfig;
```

### 1.21 Structural Operation Kind

```c
typedef int32_t DvcStructuralOpKind;

#define DVC_STRUCT_OP_NONE         0
#define DVC_STRUCT_OP_INSERT_ROW   1
#define DVC_STRUCT_OP_DELETE_ROW   2
#define DVC_STRUCT_OP_INSERT_COL   3
#define DVC_STRUCT_OP_DELETE_COL   4
```

### 1.22 Last-Reject Context

```c
typedef struct {
    DvcRejectKind       reject_kind;
    DvcStructuralOpKind op_kind;
    uint16_t            op_index;       /* row/col index when op_kind != NONE */
    int32_t             has_cell;       /* 0/1 */
    DvcCellAddr         cell;           /* optional blocked focal cell */
    int32_t             has_range;      /* 0/1 */
    DvcCellRange        range;          /* optional blocked range */
} DvcLastRejectContext;
```

## 2. Lifecycle Functions

### dvc_engine_create

```c
DvcStatus dvc_engine_create(DvcEngine **out);
```

Create a new engine with default sheet bounds (63 columns × 254 rows). On success, `*out` receives the handle. The engine starts in Automatic recalc mode with no cells.

**Returns:** `DVC_OK` on success; `DVC_ERR_NULL_POINTER` if `out` is NULL; `DVC_ERR_OUT_OF_MEMORY` on allocation failure.

**Maps to:** `Engine::new()`

### dvc_engine_create_with_bounds

```c
DvcStatus dvc_engine_create_with_bounds(DvcSheetBounds bounds, DvcEngine **out);
```

Create a new engine with custom sheet bounds.

**Returns:** `DVC_OK` on success; `DVC_ERR_INVALID_ARGUMENT` if bounds are zero or exceed implementation limits.

**Maps to:** `Engine::with_bounds()`

### dvc_engine_destroy

```c
DvcStatus dvc_engine_destroy(DvcEngine *engine);
```

Destroy the engine and release all resources. Passing NULL is a safe no-op returning `DVC_OK`. Any outstanding iterators for this engine become invalid.

**Returns:** `DVC_OK`.

**Maps to:** `drop(Engine)`

### dvc_engine_clear

```c
DvcStatus dvc_engine_clear(DvcEngine *engine);
```

Remove all cells, names, formats, and computed state. Increments `committed_epoch`. The engine bounds and recalc mode are preserved.

**Returns:** `DVC_OK` on success; `DVC_ERR_NULL_POINTER` if `engine` is NULL.

**Maps to:** `Engine::clear()`

## 3. Configuration and State Functions

### dvc_engine_bounds

```c
DvcStatus dvc_engine_bounds(const DvcEngine *engine, DvcSheetBounds *out);
```

Query the engine's sheet bounds.

**Maps to:** `Engine::bounds()`

### dvc_engine_get_recalc_mode

```c
DvcStatus dvc_engine_get_recalc_mode(const DvcEngine *engine, DvcRecalcMode *out);
```

**Maps to:** `Engine::recalc_mode()`

### dvc_engine_set_recalc_mode

```c
DvcStatus dvc_engine_set_recalc_mode(DvcEngine *engine, DvcRecalcMode mode);
```

**Maps to:** `Engine::set_recalc_mode()`

### dvc_engine_committed_epoch

```c
DvcStatus dvc_engine_committed_epoch(const DvcEngine *engine, uint64_t *out);
```

**Maps to:** `Engine::committed_epoch()`

### dvc_engine_stabilized_epoch

```c
DvcStatus dvc_engine_stabilized_epoch(const DvcEngine *engine, uint64_t *out);
```

**Maps to:** `Engine::stabilized_epoch()`

### dvc_engine_is_stable

```c
DvcStatus dvc_engine_is_stable(const DvcEngine *engine, int32_t *out);
```

Sets `*out` to 1 if `stabilized_epoch == committed_epoch`, 0 otherwise.

**Maps to:** comparison of `Engine::stabilized_epoch()` and `Engine::committed_epoch()`

## 4. Cell Functions (address-based)

All cell functions take a `DvcCellAddr` and validate it against the engine's bounds.

### dvc_cell_set_number

```c
DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double value);
```

Set a cell to a numeric literal. Increments `committed_epoch`. In Automatic mode, triggers recalculation.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS` if addr is invalid.

**Maps to:** `Engine::set_number()`

### dvc_cell_set_text

```c
DvcStatus dvc_cell_set_text(DvcEngine *engine, DvcCellAddr addr,
                            const char *text, uint32_t text_len);
```

Set a cell to a text value. `text` is UTF-8 encoded; `text_len` is the length in bytes (not including any null terminator). If `text_len` is 0 and `text` is not NULL, the text is treated as an empty string.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`; `DVC_ERR_NULL_POINTER` if `text` is NULL.

**Maps to:** `Engine::set_text()`

### dvc_cell_set_formula

```c
DvcStatus dvc_cell_set_formula(DvcEngine *engine, DvcCellAddr addr,
                               const char *formula, uint32_t formula_len);
```

Set a cell to a formula. The formula string (UTF-8, `formula_len` bytes) is parsed immediately. On parse failure, the cell is not modified and `DVC_ERR_PARSE` is returned.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`; `DVC_ERR_PARSE`; `DVC_ERR_DEPENDENCY` (if recalculation fails in Automatic mode).

**Maps to:** `Engine::set_formula()`

### dvc_cell_clear

```c
DvcStatus dvc_cell_clear(DvcEngine *engine, DvcCellAddr addr);
```

Remove all input from a cell. Increments `committed_epoch`.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::clear_cell()`

### dvc_cell_get_state

```c
DvcStatus dvc_cell_get_state(const DvcEngine *engine, DvcCellAddr addr,
                             DvcCellState *out);
```

Query the computed state of a cell. For empty cells with no computed value, returns `DVC_VALUE_BLANK` with `value_epoch == stabilized_epoch`.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::cell_state()`

### dvc_cell_get_text

```c
DvcStatus dvc_cell_get_text(const DvcEngine *engine, DvcCellAddr addr,
                            char *buf, uint32_t buf_len, uint32_t *out_len);
```

Retrieve the text value of a cell whose computed value is `DVC_VALUE_TEXT`. The text is written to `buf` (up to `buf_len` bytes). `*out_len` receives the total byte length of the text (excluding null terminator).

If `buf` is NULL and `out_len` is non-NULL, this is a length query only.

If the cell's value is not `DVC_VALUE_TEXT`, `*out_len` is set to 0 and `DVC_OK` is returned.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::cell_state()` → `Value::Text`

### dvc_cell_get_input_type

```c
DvcStatus dvc_cell_get_input_type(const DvcEngine *engine, DvcCellAddr addr,
                                  DvcInputType *out);
```

Query what kind of input a cell contains. Returns `DVC_INPUT_EMPTY` for cells with no input.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::cell_input()`

### dvc_cell_get_input_text

```c
DvcStatus dvc_cell_get_input_text(const DvcEngine *engine, DvcCellAddr addr,
                                  char *buf, uint32_t buf_len, uint32_t *out_len);
```

Retrieve the input text of a cell. For formulas, this is the formula source string. For text cells, this is the text value. For number cells, this is the decimal string representation. For empty cells, `*out_len` is 0.

Follows the same buffer/length protocol as `dvc_cell_get_text`.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::cell_input()`

## 5. Cell Functions (A1 string addressing)

Convenience wrappers that parse an A1-style cell reference string before delegating to the address-based functions.

```c
DvcStatus dvc_cell_set_number_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 double value);

DvcStatus dvc_cell_set_text_a1(DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               const char *text, uint32_t text_len);

DvcStatus dvc_cell_set_formula_a1(DvcEngine *engine,
                                  const char *cell_ref, uint32_t ref_len,
                                  const char *formula, uint32_t formula_len);

DvcStatus dvc_cell_clear_a1(DvcEngine *engine,
                            const char *cell_ref, uint32_t ref_len);

DvcStatus dvc_cell_get_state_a1(const DvcEngine *engine,
                                const char *cell_ref, uint32_t ref_len,
                                DvcCellState *out);

DvcStatus dvc_cell_get_text_a1(const DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               char *buf, uint32_t buf_len, uint32_t *out_len);

DvcStatus dvc_cell_get_input_type_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     DvcInputType *out);

DvcStatus dvc_cell_get_input_text_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);
```

All A1 functions may additionally return `DVC_ERR_INVALID_ADDRESS` if the cell reference string cannot be parsed.

**Maps to:** `Engine::set_number_a1()`, `set_text_a1()`, `set_formula_a1()`, `clear_cell_a1()`, `cell_state_a1()`, `cell_input_a1()`

## 6. Name Functions

### dvc_name_set_number

```c
DvcStatus dvc_name_set_number(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              double value);
```

Set a named value to a number. The name is validated and normalized to uppercase.

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME`.

**Maps to:** `Engine::set_name_number()`

### dvc_name_set_text

```c
DvcStatus dvc_name_set_text(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const char *text, uint32_t text_len);
```

**Maps to:** `Engine::set_name_text()`

### dvc_name_set_formula

```c
DvcStatus dvc_name_set_formula(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               const char *formula, uint32_t formula_len);
```

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME`; `DVC_ERR_PARSE`; `DVC_ERR_DEPENDENCY`.

**Maps to:** `Engine::set_name_formula()`

### dvc_name_clear

```c
DvcStatus dvc_name_clear(DvcEngine *engine,
                         const char *name, uint32_t name_len);
```

**Maps to:** `Engine::clear_name()`

### dvc_name_get_input_type

```c
DvcStatus dvc_name_get_input_type(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  DvcInputType *out);
```

Returns `DVC_INPUT_EMPTY` if the name does not exist.

**Maps to:** `Engine::name_input()`

### dvc_name_get_input_text

```c
DvcStatus dvc_name_get_input_text(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len);
```

Retrieve the input text of a named value. Same encoding rules as `dvc_cell_get_input_text`.

**Maps to:** `Engine::name_input()`

## 7. Recalculation Functions

### dvc_recalculate

```c
DvcStatus dvc_recalculate(DvcEngine *engine);
```

Perform a full recalculation. Evaluates all formulas in dependency order, resolves dynamic array spills, and sets `stabilized_epoch = committed_epoch`.

This function is always valid in both recalc modes:
- In `DVC_RECALC_MANUAL`, it is the explicit trigger to stabilize pending work.
- In `DVC_RECALC_AUTOMATIC`, it is an explicit force-recalc entrypoint and is still allowed.

**Returns:** `DVC_OK`; `DVC_ERR_DEPENDENCY` (circular dependency or graph construction failure).

**Maps to:** `Engine::recalculate()`

### dvc_has_volatile_cells

```c
DvcStatus dvc_has_volatile_cells(const DvcEngine *engine, int32_t *out);
```

Sets `*out` to 1 if any cell contains a **volatile** function (NOW, RAND, RANDARRAY) or a UDF registered with `DVC_VOLATILITY_VOLATILE`. Does **not** include externally-invalidated functions (STREAM, externally-invalidated UDFs). The caller uses this to decide whether to expose/trigger explicit volatile invalidation via `dvc_invalidate_volatile` (for example user action or host timer policy).

**Maps to:** `Engine::has_volatile_cells()`

### dvc_has_externally_invalidated_cells

```c
DvcStatus dvc_has_externally_invalidated_cells(const DvcEngine *engine, int32_t *out);
```

Sets `*out` to 1 if any cell contains an **externally-invalidated** function (STREAM, or a UDF registered with `DVC_VOLATILITY_EXTERNALLY_INVALIDATED`). The caller uses this to decide whether external invalidation triggers (timers, data feeds) are needed.

**Maps to:** `Engine::has_externally_invalidated_cells()`

### dvc_invalidate_volatile

```c
DvcStatus dvc_invalidate_volatile(DvcEngine *engine);
```

Mark all volatile cells (NOW, RAND, RANDARRAY, volatile UDFs) as dirty. Increments `committed_epoch`. In Automatic mode, triggers recalculation. In Manual mode, the caller must call `dvc_recalculate` afterward.

This replaces the previous pattern where the caller called `dvc_recalculate` directly — `dvc_invalidate_volatile` ensures only volatile cells and their dependents are recalculated (incremental), rather than forcing a full recalculation.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::invalidate_volatile()`

### dvc_has_stream_cells

```c
DvcStatus dvc_has_stream_cells(const DvcEngine *engine, int32_t *out);
```

Sets `*out` to 1 if any stream cells are registered, 0 otherwise.

**Maps to:** `Engine::has_stream_cells()`

### dvc_tick_streams

```c
DvcStatus dvc_tick_streams(DvcEngine *engine, double elapsed_secs,
                           int32_t *any_advanced);
```

Accumulate elapsed time for all stream cells. When a stream cell's accumulated time reaches its period, its counter advances. If any counter advanced, the specific stream cells are marked dirty and `committed_epoch` is incremented. `*any_advanced` is set to 1 if any counter advanced, 0 otherwise.

In Automatic mode, recalculation is triggered if any stream advanced. In Manual mode, the caller should call `dvc_recalculate` after `*any_advanced == 1`.

Only the affected stream cells and their dependents are recalculated (incremental). This is the key behavioral difference from volatile functions — stream ticks do not force a full recalculation.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::tick_streams()`

### dvc_invalidate_udf

```c
DvcStatus dvc_invalidate_udf(DvcEngine *engine,
                              const char *name, uint32_t name_len);
```

Mark all cells that call the named UDF as dirty. Only meaningful for UDFs registered with `DVC_VOLATILITY_EXTERNALLY_INVALIDATED`. Increments `committed_epoch`.

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME` if no UDF with that name is registered.

**Maps to:** `Engine::invalidate_udf()`

## 8. Format Functions

### dvc_cell_get_format

```c
DvcStatus dvc_cell_get_format(const DvcEngine *engine, DvcCellAddr addr,
                              DvcCellFormat *out);
```

Query the format of a cell. Cells with no explicit format return the default (no decimals, no bold, no italic, no colors).

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::cell_format()`

### dvc_cell_set_format

```c
DvcStatus dvc_cell_set_format(DvcEngine *engine, DvcCellAddr addr,
                              const DvcCellFormat *format);
```

Set the format of a cell. If the format is all-defaults, any existing format entry is removed. Increments `committed_epoch` but does not trigger recalculation (format is metadata only).

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::set_cell_format()`

### dvc_cell_get_format_a1 / dvc_cell_set_format_a1

```c
DvcStatus dvc_cell_get_format_a1(const DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 DvcCellFormat *out);

DvcStatus dvc_cell_set_format_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 const DvcCellFormat *format);
```

A1 convenience wrappers.

**Maps to:** `Engine::cell_format_a1()`, `Engine::set_cell_format_a1()`

## 9. Spill Functions

### dvc_cell_spill_role

```c
DvcStatus dvc_cell_spill_role(const DvcEngine *engine, DvcCellAddr addr,
                              DvcSpillRole *out);
```

Query whether a cell is part of a spill region. Returns `DVC_SPILL_NONE` for cells not in any spill, `DVC_SPILL_ANCHOR` for the formula cell that produced the array, `DVC_SPILL_MEMBER` for cells filled by the spill.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::spill_anchor_for_cell()`, `Engine::spill_range_for_cell()`

### dvc_cell_spill_anchor

```c
DvcStatus dvc_cell_spill_anchor(const DvcEngine *engine, DvcCellAddr addr,
                                DvcCellAddr *out, int32_t *found);
```

If the cell is a spill member, sets `*out` to the anchor cell address and `*found` to 1. Otherwise sets `*found` to 0.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::spill_anchor_for_cell()`

### dvc_cell_spill_range

```c
DvcStatus dvc_cell_spill_range(const DvcEngine *engine, DvcCellAddr addr,
                               DvcCellRange *out, int32_t *found);
```

If the cell is part of any spill region (anchor or member), sets `*out` to the full spill range and `*found` to 1. Otherwise sets `*found` to 0.

**Returns:** `DVC_OK`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::spill_range_for_cell()`

## 10. Iteration Functions

Iterators provide forward-only traversal of bulk data in deterministic order. The pattern is: create the iterator, call `next` in a loop until it returns "done", then destroy the iterator.

Iterators capture a snapshot of the data at creation time. Mutations to the engine during iteration do not affect the iterator's output, but the iterator must be destroyed before the engine is destroyed.

### Cell Input Iterator

```c
DvcStatus dvc_cell_iterate(const DvcEngine *engine, DvcCellIterator **out);

DvcStatus dvc_cell_iterator_next(DvcCellIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcInputType *input_type,
                                 int32_t *done);

DvcStatus dvc_cell_iterator_get_text(const DvcCellIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DvcStatus dvc_cell_iterator_destroy(DvcCellIterator *iter);
```

Iterates over all non-empty cells in deterministic order (sorted by address). After each successful `dvc_cell_iterator_next` (where `*done == 0`), `*addr` and `*input_type` are populated. For text and formula inputs, call `dvc_cell_iterator_get_text` to retrieve the string content. For number inputs, the numeric value is available through the cell state query.

**Maps to:** `Engine::all_cell_inputs()`

### Name Input Iterator

```c
DvcStatus dvc_name_iterate(const DvcEngine *engine, DvcNameIterator **out);

DvcStatus dvc_name_iterator_next(DvcNameIterator *iter,
                                 char *name_buf, uint32_t name_buf_len,
                                 uint32_t *name_len,
                                 DvcInputType *input_type,
                                 int32_t *done);

DvcStatus dvc_name_iterator_get_text(const DvcNameIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DvcStatus dvc_name_iterator_destroy(DvcNameIterator *iter);
```

Iterates over all named values in alphabetical order.

**Maps to:** `Engine::all_name_inputs()`

### Format Iterator

```c
DvcStatus dvc_format_iterate(const DvcEngine *engine, DvcFormatIterator **out);

DvcStatus dvc_format_iterator_next(DvcFormatIterator *iter,
                                   DvcCellAddr *addr,
                                   DvcCellFormat *format,
                                   int32_t *done);

DvcStatus dvc_format_iterator_destroy(DvcFormatIterator *iter);
```

Iterates over all cells with non-default formats in deterministic order.

**Maps to:** `Engine::all_cell_formats()`

## 11. Structural Mutation Functions

See [ENGINE_DESIGN_NOTES.md §1](ENGINE_DESIGN_NOTES.md#1-generalized-dependency-graph) for reference rewriting semantics.

Outcome model for structural functions:
- `DVC_OK`: operation applied.
- `DVC_REJECT_*`: valid request but not executable under structural/policy constraints.
- `DVC_ERR_*`: invalid arguments or execution failure.

Rejected outcomes are atomic no-ops:
- no partial mutation,
- no `committed_epoch` increment,
- details retrievable via `dvc_last_reject_kind` / `dvc_last_reject_context`.

Current v0 structural reject policy:
- If a structural edit intersects an active spilled range boundary that cannot be deterministically rewritten under current policy, the operation returns `DVC_REJECT_STRUCTURAL_CONSTRAINT`.

### dvc_insert_row

```c
DvcStatus dvc_insert_row(DvcEngine *engine, uint16_t at);
```

Insert a row at position `at` (1-based). All cells at row `at` and below shift down by one. Formulas referencing affected cells are rewritten (relative references shift, absolute references are preserved). References that shift out of bounds become `#REF!`. Increments `committed_epoch` and forces full recalculation.

**Returns:** `DVC_OK`; `DVC_REJECT_STRUCTURAL_CONSTRAINT`; `DVC_REJECT_POLICY`; `DVC_ERR_OUT_OF_BOUNDS` if `at` is 0 or exceeds sheet bounds.

**Maps to:** `Engine::insert_row()`

### dvc_delete_row

```c
DvcStatus dvc_delete_row(DvcEngine *engine, uint16_t at);
```

Delete row `at`. Cells in that row are destroyed. Cells below shift up. References to deleted cells become `#REF!`.

**Returns:** `DVC_OK`; `DVC_REJECT_STRUCTURAL_CONSTRAINT`; `DVC_REJECT_POLICY`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::delete_row()`

### dvc_insert_col

```c
DvcStatus dvc_insert_col(DvcEngine *engine, uint16_t at);
```

Insert a column at position `at` (1-based). Same rewriting semantics as `dvc_insert_row` but along the column axis.

**Returns:** `DVC_OK`; `DVC_REJECT_STRUCTURAL_CONSTRAINT`; `DVC_REJECT_POLICY`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::insert_col()`

### dvc_delete_col

```c
DvcStatus dvc_delete_col(DvcEngine *engine, uint16_t at);
```

Delete column `at`. Same semantics as `dvc_delete_row` along the column axis.

**Returns:** `DVC_OK`; `DVC_REJECT_STRUCTURAL_CONSTRAINT`; `DVC_REJECT_POLICY`; `DVC_ERR_OUT_OF_BOUNDS`.

**Maps to:** `Engine::delete_col()`

## 12. Iteration Config Functions

### dvc_engine_get_iteration_config

```c
DvcStatus dvc_engine_get_iteration_config(const DvcEngine *engine,
                                           DvcIterationConfig *out);
```

Query the current iterative calculation configuration. See §1.20 for the `DvcIterationConfig` struct definition.

**Returns:** `DVC_OK`; `DVC_ERR_NULL_POINTER`.

**Maps to:** `Engine::iteration_config()`

### dvc_engine_set_iteration_config

```c
DvcStatus dvc_engine_set_iteration_config(DvcEngine *engine,
                                           const DvcIterationConfig *config);
```

Set the iterative calculation configuration. When `enabled == 1`, circular dependencies are resolved by bounded iteration (up to `max_iterations` times, converging within `convergence_tolerance`). When `enabled == 0`, circular dependencies produce `#CYCLE!` errors.

Changing the iteration config does not trigger recalculation — the new config takes effect on the next `dvc_recalculate` call.

**Returns:** `DVC_OK`; `DVC_ERR_NULL_POINTER`; `DVC_ERR_INVALID_ARGUMENT` if `max_iterations` is 0 or `convergence_tolerance` is negative.

**Maps to:** `Engine::set_iteration_config()`

## 13. Control Functions

Controls are named values with additional metadata (kind, min, max, step). See [ENGINE_DESIGN_NOTES.md §2](ENGINE_DESIGN_NOTES.md#2-controls-as-engine-entities) for design rationale.

### dvc_control_define

```c
DvcStatus dvc_control_define(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              const DvcControlDef *def);
```

Define a control. Creates the underlying named value if it doesn't exist, with an initial value based on kind (Slider: `def->min`, Checkbox: 0.0, Button: 0.0). If the name already exists as a plain named value, it is promoted to a control. Name validation follows the same rules as `dvc_name_set_number`.

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME`; `DVC_ERR_INVALID_ARGUMENT` (for Slider: if `min > max` or `step <= 0`).

**Maps to:** `Engine::define_control()`

### dvc_control_remove

```c
DvcStatus dvc_control_remove(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              int32_t *found);
```

Remove a control definition. The underlying named value is NOT removed — it reverts to a plain name. To also remove the value, call `dvc_name_clear` afterward. `*found` is set to 1 if a control was removed, 0 if the name was not a control.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::remove_control()`

### dvc_control_set_value

```c
DvcStatus dvc_control_set_value(DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double value);
```

Set a control's value with validation. For Sliders, the value is clamped to `[min, max]`. For Checkboxes, the value must be 0.0 or 1.0. For Buttons, the value is always 0.0 (this function is a no-op for buttons).

Equivalent to `dvc_name_set_number` but with kind-specific validation. Increments `committed_epoch` and triggers recalculation in Automatic mode.

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME` (name is not a control); `DVC_ERR_INVALID_ARGUMENT` (invalid value for checkbox).

**Maps to:** `Engine::set_control_value()`

### dvc_control_get_value

```c
DvcStatus dvc_control_get_value(const DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double *out, int32_t *found);
```

Get a control's current value. `*found` is set to 1 if the name is a control, 0 otherwise.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::control_value()`

### dvc_control_get_def

```c
DvcStatus dvc_control_get_def(const DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               DvcControlDef *out, int32_t *found);
```

Get a control's definition. `*found` is set to 1 if the name is a control, 0 otherwise.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::control_definition()`

### Control Iterator

```c
DvcStatus dvc_control_iterate(const DvcEngine *engine,
                               DvcControlIterator **out);

DvcStatus dvc_control_iterator_next(DvcControlIterator *iter,
                                     char *name_buf, uint32_t name_buf_len,
                                     uint32_t *name_len,
                                     DvcControlDef *def,
                                     double *value,
                                     int32_t *done);

DvcStatus dvc_control_iterator_destroy(DvcControlIterator *iter);
```

Iterates over all controls in alphabetical order by name. Each iteration step produces the control name, definition, and current value.

**Maps to:** `Engine::all_controls()`

## 14. Chart Functions

Charts are sink nodes in the dependency graph. See [ENGINE_DESIGN_NOTES.md §3](ENGINE_DESIGN_NOTES.md#3-charts-as-engine-entities) for design rationale.

### dvc_chart_define

```c
DvcStatus dvc_chart_define(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const DvcChartDef *def);
```

Define a chart. The chart's source range cells become dependencies — the chart output is recomputed when any source cell changes. Name validation follows named-value rules. Charts occupy a separate namespace from names (a chart and a name can share the same identifier).

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME`; `DVC_ERR_OUT_OF_BOUNDS` (source range exceeds sheet bounds).

**Maps to:** `Engine::define_chart()`

### dvc_chart_remove

```c
DvcStatus dvc_chart_remove(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            int32_t *found);
```

Remove a chart definition and its computed output. `*found` is set to 1 if removed, 0 if not found.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::remove_chart()`

### dvc_chart_get_output

```c
DvcStatus dvc_chart_get_output(const DvcEngine *engine,
                                const char *name, uint32_t name_len,
                                DvcChartOutput **out, int32_t *found);
```

Get a chart's computed output. `*found` is set to 1 if the chart exists and has been computed, 0 otherwise. The output handle is valid until the next recalculation or chart mutation.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::chart_output()`

### Chart Output Accessors

```c
DvcStatus dvc_chart_output_series_count(const DvcChartOutput *output,
                                         uint32_t *out);

DvcStatus dvc_chart_output_label_count(const DvcChartOutput *output,
                                        uint32_t *out);

DvcStatus dvc_chart_output_label(const DvcChartOutput *output,
                                  uint32_t index,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len);

DvcStatus dvc_chart_output_series_name(const DvcChartOutput *output,
                                        uint32_t series_index,
                                        char *buf, uint32_t buf_len,
                                        uint32_t *out_len);

DvcStatus dvc_chart_output_series_values(const DvcChartOutput *output,
                                          uint32_t series_index,
                                          double *buf, uint32_t buf_len,
                                          uint32_t *out_count);
```

Accessors for the opaque `DvcChartOutput` handle. Labels are category labels (X-axis). Series are named data sequences (Y-axis). `dvc_chart_output_series_values` writes up to `buf_len` doubles to `buf` and sets `*out_count` to the total number of values in the series.

### Chart Iterator

```c
DvcStatus dvc_chart_iterate(const DvcEngine *engine,
                             DvcChartIterator **out);

DvcStatus dvc_chart_iterator_next(DvcChartIterator *iter,
                                   char *name_buf, uint32_t name_buf_len,
                                   uint32_t *name_len,
                                   DvcChartDef *def,
                                   int32_t *done);

DvcStatus dvc_chart_iterator_destroy(DvcChartIterator *iter);
```

Iterates over all chart definitions in alphabetical order.

**Maps to:** `Engine::all_charts()`

## 15. UDF Registration Functions

### dvc_udf_register

```c
DvcStatus dvc_udf_register(DvcEngine *engine,
                             const char *name, uint32_t name_len,
                             DvcUdfCallback callback,
                             void *user_data,
                             DvcVolatility volatility);
```

Register a user-defined function. The `name` is matched case-insensitively (stored in uppercase). If a UDF with the same name already exists, it is replaced.

```c
typedef DvcStatus (*DvcUdfCallback)(
    void *user_data,
    const DvcCellValue *args, uint32_t arg_count,
    DvcCellValue *out
);
```

The callback receives evaluated argument values and writes a single result value. The callback must not call any other `dvc_*` function on the same engine handle (reentrant calls are undefined behavior).

The `volatility` parameter controls recalculation behavior:
- `DVC_VOLATILITY_STANDARD` — recalculated only when arguments change
- `DVC_VOLATILITY_VOLATILE` — recalculated on every `dvc_invalidate_volatile` call
- `DVC_VOLATILITY_EXTERNALLY_INVALIDATED` — recalculated only when `dvc_invalidate_udf` is called for this name

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_NAME`; `DVC_ERR_NULL_POINTER` (if callback is NULL).

**Maps to:** `Engine::register_udf()`

### dvc_udf_unregister

```c
DvcStatus dvc_udf_unregister(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               int32_t *found);
```

Unregister a UDF. `*found` is set to 1 if the UDF existed, 0 otherwise. Cells using the UDF will produce `#NAME?` errors on the next recalculation.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::unregister_udf()`

## 16. Change Tracking Functions

See [ENGINE_DESIGN_NOTES.md §5](ENGINE_DESIGN_NOTES.md#5-change-journal-calcdelta-pathfinder) for the CalcDelta pathfinder design.

### dvc_change_tracking_enable

```c
DvcStatus dvc_change_tracking_enable(DvcEngine *engine);
```

Enable change tracking. Change entries accumulate during mutations and recalculations until drained.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::enable_change_tracking()`

### dvc_change_tracking_disable

```c
DvcStatus dvc_change_tracking_disable(DvcEngine *engine);
```

Disable change tracking. Pending entries are discarded.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::disable_change_tracking()`

### dvc_change_tracking_is_enabled

```c
DvcStatus dvc_change_tracking_is_enabled(const DvcEngine *engine,
                                          int32_t *out);
```

Sets `*out` to 1 if change tracking is enabled, 0 otherwise.

**Returns:** `DVC_OK`.

**Maps to:** `Engine::is_change_tracking_enabled()`

### Change Iterator

```c
DvcStatus dvc_change_iterate(DvcEngine *engine, DvcChangeIterator **out);
```

Drain all accumulated change entries and return them as an iterator. The engine's internal change buffer is cleared. The iterator yields entries in the order they were produced.

**Note:** Unlike other iterators, this function takes a mutable engine pointer because it drains (clears) the internal buffer.

```c
DvcStatus dvc_change_iterator_next(DvcChangeIterator *iter,
                                    DvcChangeType *change_type,
                                    uint64_t *epoch,
                                    int32_t *done);
```

Advance to the next change entry. `*change_type` identifies the kind of change and `*epoch` is the `committed_epoch` that produced it.

After a successful `next` call (where `*done == 0`), the following accessors retrieve type-specific details:

```c
/* For DVC_CHANGE_CELL_VALUE: */
DvcStatus dvc_change_get_cell(const DvcChangeIterator *iter,
                               DvcCellAddr *addr);

/* For DVC_CHANGE_NAME_VALUE: */
DvcStatus dvc_change_get_name(const DvcChangeIterator *iter,
                               char *buf, uint32_t buf_len,
                               uint32_t *out_len);

/* For DVC_CHANGE_CHART_OUTPUT: */
DvcStatus dvc_change_get_chart_name(const DvcChangeIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

/* For DVC_CHANGE_SPILL_REGION: */
DvcStatus dvc_change_get_spill(const DvcChangeIterator *iter,
                                DvcCellAddr *anchor,
                                DvcCellRange *old_range, int32_t *had_old,
                                DvcCellRange *new_range, int32_t *has_new);

/* For DVC_CHANGE_CELL_FORMAT: */
DvcStatus dvc_change_get_format(const DvcChangeIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcCellFormat *old_fmt,
                                 DvcCellFormat *new_fmt);
```

```c
DvcStatus dvc_change_iterator_destroy(DvcChangeIterator *iter);
```

**Maps to:** `Engine::drain_changes()`

## 17. Utility Functions

### dvc_last_error_message

```c
DvcStatus dvc_last_error_message(const DvcEngine *engine,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);
```

Retrieve a human-readable error message for the most recent failed operation on this engine handle. The message is UTF-8 encoded. If no error has occurred, `*out_len` is 0.

The error message is valid until the next API call on the same handle.

### dvc_last_error_kind

```c
DvcStatus dvc_last_error_kind(const DvcEngine *engine, DvcStatus *out);
```

Retrieve the most recent negative status code for this handle.
- `*out == DVC_OK` means no error is currently recorded.
- `*out < 0` is one of the `DVC_ERR_*` codes.

### dvc_last_reject_kind

```c
DvcStatus dvc_last_reject_kind(const DvcEngine *engine, DvcRejectKind *out);
```

Retrieve the most recent reject reason class for this handle.
- `DVC_REJECT_KIND_NONE` means no reject is currently recorded.

### dvc_last_reject_context

```c
DvcStatus dvc_last_reject_context(const DvcEngine *engine,
                                  DvcLastRejectContext *out);
```

Retrieve structured context for the most recent reject on this handle (operation kind/index and optional cell/range payload).
- If no reject is recorded, `out->reject_kind` is `DVC_REJECT_KIND_NONE`.

### dvc_cell_error_message

```c
DvcStatus dvc_cell_error_message(const DvcEngine *engine, DvcCellAddr addr,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);
```

For cells whose computed value is `DVC_VALUE_ERROR`, retrieve the error's descriptive message. If the cell is not in error state, `*out_len` is 0.

**Maps to:** `CellError::Display`

### dvc_palette_color_name

```c
DvcStatus dvc_palette_color_name(DvcPaletteColor color,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);
```

Return the canonical name string for a palette color index (e.g., 0 → "MIST", 7 → "TEAL").

**Maps to:** `PaletteColor::as_name()`

### dvc_parse_cell_ref

```c
DvcStatus dvc_parse_cell_ref(const DvcEngine *engine,
                             const char *ref_str, uint32_t ref_len,
                             DvcCellAddr *out);
```

Parse an A1-style cell reference string into a `DvcCellAddr`, validated against the engine's bounds.

**Returns:** `DVC_OK`; `DVC_ERR_INVALID_ADDRESS`.

**Maps to:** `parse_cell_ref()`

### dvc_api_version

```c
uint32_t dvc_api_version(void);
```

Return the API version as a packed integer: `(major << 16) | (minor << 8) | patch`. This is the only function that does not require an engine handle and does not return `DvcStatus`.

## 18. Thread Safety Contract

| Guarantee | Scope |
|-----------|-------|
| Multiple `DvcEngine` handles may be used concurrently on different threads | Global |
| A single `DvcEngine` handle must not be accessed concurrently | Per-handle |
| `dvc_api_version()` is safe to call from any thread at any time | Global |
| `dvc_palette_color_name()` is safe to call from any thread at any time | Global |
| Iterator handles inherit the threading contract of their parent engine | Per-handle |

No global mutable state exists. No global initialization function is required.

## 19. String Encoding Contract

- All string parameters and outputs are **UTF-8** encoded.
- All string lengths are in **bytes**, not characters or code points.
- String parameters are **not** required to be null-terminated; length is always provided explicitly.
- Output buffers follow the caller-provided buffer protocol: pass `NULL` buffer with non-NULL `out_len` to query required size.
- The `out_len` value does **not** include a null terminator. If the caller wants a null-terminated string, they must allocate `out_len + 1` bytes and append the terminator themselves.

## 20. Error Detail Contract

Each `DvcEngine` handle maintains internal last-status diagnostics:
- `last_error_message` and `last_error_kind` for negative (`DVC_ERR_*`) outcomes.
- `last_reject_kind` and `last_reject_context` for positive (`DVC_REJECT_*`) outcomes.
- These diagnostics are handle-local (not thread-local) and valid until the next API call on the same handle.

Update rules:
- On `DVC_OK`: clear error and reject diagnostics.
- On `DVC_REJECT_*`: set reject diagnostics, clear error diagnostics.
- On `DVC_ERR_*`: set error diagnostics, clear reject diagnostics.

`dvc_last_error_message` provides human-readable detail; `dvc_last_error_kind`/`dvc_last_reject_kind`/`dvc_last_reject_context` provide machine-readable detail.

## 21. Rust Implementation Notes

The future `dnavisicalc-cabi` crate will wrap the engine:

```rust
// Internal wrapper (not exposed through C API)
struct DvcEngine {
    inner: Engine,
    last_error: Option<String>,
    last_error_kind: DvcStatus,
    last_reject_kind: DvcRejectKind,
    last_reject_context: DvcLastRejectContext,
}
```

Each `#[no_mangle] extern "C"` function will:
1. Validate pointer arguments (NULL → `DVC_ERR_NULL_POINTER`)
2. Call the corresponding `Engine` method
3. Map result to `DvcStatus` outcome (`DVC_OK`, `DVC_REJECT_*`, or `DVC_ERR_*`)
4. Update diagnostics per §20 and return status

`DvcStatus` mapping from Rust errors:

| Rust Error | DvcStatus |
|-----------|-----------|
| `EngineError::Address(_)` | `DVC_ERR_INVALID_ADDRESS` |
| `EngineError::Parse(_)` | `DVC_ERR_PARSE` |
| `EngineError::Dependency(_)` | `DVC_ERR_DEPENDENCY` |
| `EngineError::Name(_)` | `DVC_ERR_INVALID_NAME` |
| `EngineError::OutOfBounds(_)` | `DVC_ERR_OUT_OF_BOUNDS` |

`DVC_REJECT_*` statuses are produced by explicit policy/constraint checks (for example structural-constraint rejections), not by `EngineError` mapping.

## 22. API Coverage Cross-Reference

### Engine methods → API functions

| Engine method | API function |
|--------------|-------------|
| `Engine::new()` | `dvc_engine_create` |
| `Engine::with_bounds()` | `dvc_engine_create_with_bounds` |
| `drop(Engine)` | `dvc_engine_destroy` |
| `Engine::clear()` | `dvc_engine_clear` |
| `Engine::bounds()` | `dvc_engine_bounds` |
| `Engine::recalc_mode()` | `dvc_engine_get_recalc_mode` |
| `Engine::set_recalc_mode()` | `dvc_engine_set_recalc_mode` |
| `Engine::committed_epoch()` | `dvc_engine_committed_epoch` |
| `Engine::stabilized_epoch()` | `dvc_engine_stabilized_epoch` |
| `Engine::set_number()` | `dvc_cell_set_number` |
| `Engine::set_text()` | `dvc_cell_set_text` |
| `Engine::set_formula()` | `dvc_cell_set_formula` |
| `Engine::clear_cell()` | `dvc_cell_clear` |
| `Engine::set_number_a1()` | `dvc_cell_set_number_a1` |
| `Engine::set_text_a1()` | `dvc_cell_set_text_a1` |
| `Engine::set_formula_a1()` | `dvc_cell_set_formula_a1` |
| `Engine::clear_cell_a1()` | `dvc_cell_clear_a1` |
| `Engine::cell_state()` | `dvc_cell_get_state` |
| `Engine::cell_state_a1()` | `dvc_cell_get_state_a1` |
| `Engine::cell_input()` | `dvc_cell_get_input_type`, `dvc_cell_get_input_text` |
| `Engine::cell_input_a1()` | `dvc_cell_get_input_type_a1`, `dvc_cell_get_input_text_a1` |
| `Engine::set_cell_input()` | via typed setters (`set_number`, `set_text`, `set_formula`) |
| `Engine::set_cell_input_a1()` | via typed A1 setters |
| `Engine::set_name_number()` | `dvc_name_set_number` |
| `Engine::set_name_text()` | `dvc_name_set_text` |
| `Engine::set_name_formula()` | `dvc_name_set_formula` |
| `Engine::set_name_input()` | via typed name setters |
| `Engine::clear_name()` | `dvc_name_clear` |
| `Engine::name_input()` | `dvc_name_get_input_type`, `dvc_name_get_input_text` |
| `Engine::recalculate()` | `dvc_recalculate` |
| `Engine::has_volatile_cells()` | `dvc_has_volatile_cells` |
| `Engine::has_externally_invalidated_cells()` | `dvc_has_externally_invalidated_cells` |
| `Engine::invalidate_volatile()` | `dvc_invalidate_volatile` |
| `Engine::has_stream_cells()` | `dvc_has_stream_cells` |
| `Engine::tick_streams()` | `dvc_tick_streams` |
| `Engine::invalidate_udf()` | `dvc_invalidate_udf` |
| `Engine::cell_format()` | `dvc_cell_get_format` |
| `Engine::cell_format_a1()` | `dvc_cell_get_format_a1` |
| `Engine::set_cell_format()` | `dvc_cell_set_format` |
| `Engine::set_cell_format_a1()` | `dvc_cell_set_format_a1` |
| `Engine::spill_anchor_for_cell()` | `dvc_cell_spill_anchor` |
| `Engine::spill_range_for_cell()` | `dvc_cell_spill_range` |
| `Engine::spill_range_for_anchor()` | `dvc_cell_spill_range` (subsumes) |
| `Engine::all_cell_inputs()` | `dvc_cell_iterate` + iterator functions |
| `Engine::all_name_inputs()` | `dvc_name_iterate` + iterator functions |
| `Engine::all_cell_formats()` | `dvc_format_iterate` + iterator functions |
| `Engine::insert_row()` | `dvc_insert_row` |
| `Engine::delete_row()` | `dvc_delete_row` |
| `Engine::insert_col()` | `dvc_insert_col` |
| `Engine::delete_col()` | `dvc_delete_col` |
| `Engine::iteration_config()` | `dvc_engine_get_iteration_config` |
| `Engine::set_iteration_config()` | `dvc_engine_set_iteration_config` |
| `Engine::define_control()` | `dvc_control_define` |
| `Engine::remove_control()` | `dvc_control_remove` |
| `Engine::set_control_value()` | `dvc_control_set_value` |
| `Engine::control_value()` | `dvc_control_get_value` |
| `Engine::control_definition()` | `dvc_control_get_def` |
| `Engine::all_controls()` | `dvc_control_iterate` + iterator functions |
| `Engine::define_chart()` | `dvc_chart_define` |
| `Engine::remove_chart()` | `dvc_chart_remove` |
| `Engine::chart_output()` | `dvc_chart_get_output` + output accessors |
| `Engine::all_charts()` | `dvc_chart_iterate` + iterator functions |
| `Engine::register_udf()` | `dvc_udf_register` |
| `Engine::unregister_udf()` | `dvc_udf_unregister` |
| `Engine::enable_change_tracking()` | `dvc_change_tracking_enable` |
| `Engine::disable_change_tracking()` | `dvc_change_tracking_disable` |
| `Engine::is_change_tracking_enabled()` | `dvc_change_tracking_is_enabled` |
| `Engine::drain_changes()` | `dvc_change_iterate` + iterator functions |
| `PaletteColor::as_name()` | `dvc_palette_color_name` |
| `parse_cell_ref()` | `dvc_parse_cell_ref` |
| Handle-local diagnostic state | `dvc_last_error_message`, `dvc_last_error_kind`, `dvc_last_reject_kind`, `dvc_last_reject_context` |

### Intentionally excluded from C API

| Engine method | Reason |
|--------------|--------|
| `Engine::calc_tree()` | AST internals; stays behind the boundary |
| `Engine::formula_source_a1()` | Subsumed by `dvc_cell_get_input_text_a1` |
| `Engine::dynamic_array_strategy()` | Implementation detail; not a public contract |
| `Engine::set_dynamic_array_strategy()` | Implementation detail |
| `Engine::spill_anchor_for_cell_a1()` | A1 variant not needed; caller can use `dvc_parse_cell_ref` + addr-based function |
| `Engine::spill_range_for_cell_a1()` | Same reasoning |
| `Engine::spill_range_for_anchor()` | Subsumed by `dvc_cell_spill_range` which works for any cell in the range |

### TUI (app.rs) call coverage

Every `self.engine.*` call in `app.rs` maps to a C API function:

| app.rs call | C API function |
|------------|---------------|
| `engine.recalculate()` | `dvc_recalculate` |
| `engine.tick_streams(elapsed)` | `dvc_tick_streams` |
| `engine.has_volatile_cells()` | `dvc_has_volatile_cells` |
| `engine.has_stream_cells()` | `dvc_has_stream_cells` |
| `engine.set_name_number(name, val)` | `dvc_name_set_number` |
| `engine.committed_epoch()` | `dvc_engine_committed_epoch` |
| `engine.cell_state(cell)` | `dvc_cell_get_state` |
| `engine.cell_format(cell)` | `dvc_cell_get_format` |
| `engine.spill_range_for_cell(cell)` | `dvc_cell_spill_range` |
| `engine.cell_input(cell)` | `dvc_cell_get_input_type` + `dvc_cell_get_input_text` |
| `engine.set_recalc_mode(mode)` | `dvc_engine_set_recalc_mode` |
| `engine.bounds()` | `dvc_engine_bounds` |
| `engine.clear_name(name)` | `dvc_name_clear` |
| `engine.clear_cell(cell)` | `dvc_cell_clear` |
| `engine.set_number(cell, n)` | `dvc_cell_set_number` |
| `engine.set_text(cell, t)` | `dvc_cell_set_text` |
| `engine.set_formula(cell, f)` | `dvc_cell_set_formula` |
| `engine.set_cell_format(cell, fmt)` | `dvc_cell_set_format` |
| `engine.spill_anchor_for_cell(cell)` | `dvc_cell_spill_anchor` |
| `engine.set_number_a1(ref, n)` | `dvc_cell_set_number_a1` |
| `engine.set_text_a1(ref, t)` | `dvc_cell_set_text_a1` |
| `engine.set_formula_a1(ref, f)` | `dvc_cell_set_formula_a1` |
| `engine.set_name_formula(name, f)` | `dvc_name_set_formula` |
| `engine.set_name_number(name, n)` | `dvc_name_set_number` |
| `engine.set_name_text(name, t)` | `dvc_name_set_text` |
| `engine.insert_row(at)` | `dvc_insert_row` |
| `engine.delete_row(at)` | `dvc_delete_row` |
| `engine.insert_col(at)` | `dvc_insert_col` |
| `engine.delete_col(at)` | `dvc_delete_col` |

Future TUI calls (after engine-backed controls/charts migration):

| app.rs call (planned) | C API function |
|------------|---------------|
| `engine.define_control(name, def)` | `dvc_control_define` |
| `engine.set_control_value(name, val)` | `dvc_control_set_value` |
| `engine.control_value(name)` | `dvc_control_get_value` |
| `engine.all_controls()` | `dvc_control_iterate` |
| `engine.define_chart(name, def)` | `dvc_chart_define` |
| `engine.chart_output(name)` | `dvc_chart_get_output` |
| `engine.enable_change_tracking()` | `dvc_change_tracking_enable` |
| `engine.drain_changes()` | `dvc_change_iterate` |

### File I/O (lib.rs) call coverage

| dnavisicalc-file call | C API function |
|----------------------|---------------|
| `engine.recalc_mode()` | `dvc_engine_get_recalc_mode` |
| `engine.all_cell_inputs()` | `dvc_cell_iterate` |
| `engine.all_name_inputs()` | `dvc_name_iterate` |
| `engine.all_cell_formats()` | `dvc_format_iterate` |
| `engine.set_recalc_mode(Manual)` | `dvc_engine_set_recalc_mode` |
| `engine.set_cell_input(cell, input)` | `dvc_cell_set_number` / `set_text` / `set_formula` |
| `engine.set_name_input(name, input)` | `dvc_name_set_number` / `set_text` / `set_formula` |
| `engine.set_cell_format(cell, fmt)` | `dvc_cell_set_format` |
| `engine.recalculate()` | `dvc_recalculate` |
| `engine.set_recalc_mode(mode)` | `dvc_engine_set_recalc_mode` |
