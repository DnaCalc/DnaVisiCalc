use std::ffi::c_void;
use std::ptr;

use dnavisicalc_core::{
    AddressError, CellError, CellFormat, CellInput, CellRange, CellRef, CellState, Engine,
    EngineError, IterationConfig, NameInput, PaletteColor, RecalcMode, SheetBounds, Value,
};

const DVC_OK: i32 = 0;

const DVC_ERR_NULL_POINTER: i32 = -1;
const DVC_ERR_OUT_OF_BOUNDS: i32 = -2;
const DVC_ERR_INVALID_ADDRESS: i32 = -3;
const DVC_ERR_PARSE: i32 = -4;
const DVC_ERR_DEPENDENCY: i32 = -5;
const DVC_ERR_INVALID_NAME: i32 = -6;
const DVC_ERR_INVALID_ARGUMENT: i32 = -8;
const DVC_ERR_UNSUPPORTED: i32 = DVC_ERR_INVALID_ARGUMENT;

const DVC_VALUE_NUMBER: i32 = 0;
const DVC_VALUE_TEXT: i32 = 1;
const DVC_VALUE_BOOL: i32 = 2;
const DVC_VALUE_BLANK: i32 = 3;
const DVC_VALUE_ERROR: i32 = 4;

const DVC_ERROR_DIV_ZERO: i32 = 0;
const DVC_ERROR_VALUE: i32 = 1;
const DVC_ERROR_NAME: i32 = 2;
const DVC_ERROR_UNKNOWN_NAME: i32 = 3;
const DVC_ERROR_REF: i32 = 4;
const DVC_ERROR_SPILL: i32 = 5;
const DVC_ERROR_CYCLE: i32 = 6;
const DVC_ERROR_NA: i32 = 7;
const DVC_ERROR_NULL: i32 = 8;
const DVC_ERROR_NUM: i32 = 9;

const DVC_RECALC_AUTOMATIC: i32 = 0;
const DVC_RECALC_MANUAL: i32 = 1;

const DVC_INPUT_EMPTY: i32 = 0;
const DVC_INPUT_NUMBER: i32 = 1;
const DVC_INPUT_TEXT: i32 = 2;
const DVC_INPUT_FORMULA: i32 = 3;

const DVC_PALETTE_NONE: i32 = -1;

const DVC_CONTROL_SLIDER: i32 = 0;

const DVC_API_VERSION_PACKED: u32 = (0u32 << 16) | (1u32 << 8);

type DvcEngineHandle = *mut c_void;
type DvcIteratorHandle = *mut c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcCellAddr {
    col: u16,
    row: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcCellRange {
    start: DvcCellAddr,
    end: DvcCellAddr,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcSheetBounds {
    max_columns: u16,
    max_rows: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcCellValue {
    value_type: i32,
    number: f64,
    bool_val: i32,
    error_kind: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcCellState {
    value: DvcCellValue,
    value_epoch: u64,
    stale: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcCellFormatRaw {
    has_decimals: i32,
    decimals: u8,
    _padding: [u8; 3],
    bold: i32,
    italic: i32,
    fg: i32,
    bg: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcIterationConfigRaw {
    enabled: i32,
    max_iterations: u32,
    convergence_tolerance: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcControlDefRaw {
    kind: i32,
    min: f64,
    max: f64,
    step: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcChartDefRaw {
    source_range: DvcCellRange,
}

struct EngineHandle {
    engine: Engine,
    last_error: String,
}

impl EngineHandle {
    fn clear_error(&mut self) {
        self.last_error.clear();
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.last_error = message.into();
    }
}

#[derive(Clone)]
struct CellIteratorEntry {
    addr: DvcCellAddr,
    input_type: i32,
    text: String,
}

struct CellIteratorState {
    entries: Vec<CellIteratorEntry>,
    index: usize,
    current_text: String,
}

#[derive(Clone)]
struct NameIteratorEntry {
    name: String,
    input_type: i32,
    text: String,
}

struct NameIteratorState {
    entries: Vec<NameIteratorEntry>,
    index: usize,
    current_text: String,
}

#[derive(Clone)]
struct FormatIteratorEntry {
    addr: DvcCellAddr,
    format: DvcCellFormatRaw,
}

struct FormatIteratorState {
    entries: Vec<FormatIteratorEntry>,
    index: usize,
}

enum IteratorHandle {
    Cell(CellIteratorState),
    Name(NameIteratorState),
    Format(FormatIteratorState),
}

unsafe fn engine_handle_mut<'a>(engine: DvcEngineHandle) -> Result<&'a mut EngineHandle, i32> {
    if engine.is_null() {
        return Err(DVC_ERR_NULL_POINTER);
    }
    let ptr = engine.cast::<EngineHandle>();
    // SAFETY: The pointer is created from `Box<EngineHandle>` in `dvc_engine_create*`.
    unsafe { ptr.as_mut().ok_or(DVC_ERR_NULL_POINTER) }
}

unsafe fn iterator_handle_mut<'a>(
    iterator: DvcIteratorHandle,
) -> Result<&'a mut IteratorHandle, i32> {
    if iterator.is_null() {
        return Err(DVC_ERR_NULL_POINTER);
    }
    let ptr = iterator.cast::<IteratorHandle>();
    // SAFETY: The pointer is created from `Box<IteratorHandle>` in `dvc_*_iterate`.
    unsafe { ptr.as_mut().ok_or(DVC_ERR_NULL_POINTER) }
}

fn status_for_address_error(err: &AddressError) -> i32 {
    match err {
        AddressError::ColumnOutOfBounds { .. } | AddressError::RowOutOfBounds { .. } => {
            DVC_ERR_OUT_OF_BOUNDS
        }
        AddressError::Empty
        | AddressError::InvalidFormat(_)
        | AddressError::InvalidColumnLabel(_) => DVC_ERR_INVALID_ADDRESS,
    }
}

fn status_for_engine_error(err: &EngineError) -> i32 {
    match err {
        EngineError::Address(addr) => status_for_address_error(addr),
        EngineError::Parse(_) => DVC_ERR_PARSE,
        EngineError::Dependency(_) => DVC_ERR_DEPENDENCY,
        EngineError::Name(_) => DVC_ERR_INVALID_NAME,
        EngineError::OutOfBounds(_) => DVC_ERR_OUT_OF_BOUNDS,
    }
}

fn engine_result<T>(handle: &mut EngineHandle, result: Result<T, EngineError>) -> Result<T, i32> {
    match result {
        Ok(value) => {
            handle.clear_error();
            Ok(value)
        }
        Err(err) => {
            let status = status_for_engine_error(&err);
            handle.set_error(err.to_string());
            Err(status)
        }
    }
}

macro_rules! engine_call {
    ($handle:expr, $expr:expr) => {{
        let result = $expr;
        engine_result($handle, result)
    }};
}

fn fail(handle: &mut EngineHandle, status: i32, message: impl Into<String>) -> i32 {
    handle.set_error(message);
    status
}

fn read_utf8(input: *const u8, len: u32) -> Result<String, i32> {
    if len == 0 {
        return Ok(String::new());
    }
    if input.is_null() {
        return Err(DVC_ERR_NULL_POINTER);
    }
    // SAFETY: The caller provides a valid `(ptr, len)` byte slice for C ABI strings.
    let bytes = unsafe { std::slice::from_raw_parts(input, len as usize) };
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_string()),
        Err(_) => Err(DVC_ERR_INVALID_ARGUMENT),
    }
}

fn write_utf8(text: &str, buf: *mut u8, buf_len: u32, out_len: *mut u32) -> i32 {
    if out_len.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let bytes = text.as_bytes();
    let required = match u32::try_from(bytes.len()) {
        Ok(value) => value,
        Err(_) => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: `out_len` is validated non-null above.
    unsafe {
        *out_len = required;
    }
    if required == 0 {
        return DVC_OK;
    }
    if buf.is_null() {
        return if buf_len == 0 {
            DVC_OK
        } else {
            DVC_ERR_NULL_POINTER
        };
    }
    if buf_len < required {
        return DVC_ERR_INVALID_ARGUMENT;
    }
    // SAFETY: `buf` is non-null and has at least `required` bytes capacity.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf, required as usize);
    }
    DVC_OK
}

fn read_text_arg(
    handle: &mut EngineHandle,
    value: *const u8,
    len: u32,
    label: &str,
) -> Result<String, i32> {
    match read_utf8(value, len) {
        Ok(text) => Ok(text),
        Err(status) => {
            handle.set_error(format!("{label} must be valid UTF-8"));
            Err(status)
        }
    }
}

fn cell_from_addr(handle: &mut EngineHandle, addr: DvcCellAddr) -> Result<CellRef, i32> {
    match CellRef::new(addr.col, addr.row, handle.engine.bounds()) {
        Ok(cell) => Ok(cell),
        Err(err) => {
            let status = status_for_address_error(&err);
            handle.set_error(err.to_string());
            Err(status)
        }
    }
}

fn cell_to_addr(cell: CellRef) -> DvcCellAddr {
    DvcCellAddr {
        col: cell.col,
        row: cell.row,
    }
}

fn range_to_raw(range: CellRange) -> DvcCellRange {
    DvcCellRange {
        start: cell_to_addr(range.start),
        end: cell_to_addr(range.end),
    }
}

fn raw_to_iteration_config(raw: DvcIterationConfigRaw) -> IterationConfig {
    IterationConfig {
        enabled: raw.enabled != 0,
        max_iterations: raw.max_iterations,
        convergence_tolerance: raw.convergence_tolerance,
    }
}

fn iteration_config_to_raw(config: IterationConfig) -> DvcIterationConfigRaw {
    DvcIterationConfigRaw {
        enabled: if config.enabled { 1 } else { 0 },
        max_iterations: config.max_iterations,
        convergence_tolerance: config.convergence_tolerance,
    }
}

fn palette_to_raw(color: Option<PaletteColor>) -> i32 {
    match color {
        None => DVC_PALETTE_NONE,
        Some(PaletteColor::Mist) => 0,
        Some(PaletteColor::Sage) => 1,
        Some(PaletteColor::Fern) => 2,
        Some(PaletteColor::Moss) => 3,
        Some(PaletteColor::Olive) => 4,
        Some(PaletteColor::Seafoam) => 5,
        Some(PaletteColor::Lagoon) => 6,
        Some(PaletteColor::Teal) => 7,
        Some(PaletteColor::Sky) => 8,
        Some(PaletteColor::Cloud) => 9,
        Some(PaletteColor::Sand) => 10,
        Some(PaletteColor::Clay) => 11,
        Some(PaletteColor::Peach) => 12,
        Some(PaletteColor::Rose) => 13,
        Some(PaletteColor::Lavender) => 14,
        Some(PaletteColor::Slate) => 15,
    }
}

fn palette_from_raw(raw: i32) -> Option<PaletteColor> {
    match raw {
        0 => Some(PaletteColor::Mist),
        1 => Some(PaletteColor::Sage),
        2 => Some(PaletteColor::Fern),
        3 => Some(PaletteColor::Moss),
        4 => Some(PaletteColor::Olive),
        5 => Some(PaletteColor::Seafoam),
        6 => Some(PaletteColor::Lagoon),
        7 => Some(PaletteColor::Teal),
        8 => Some(PaletteColor::Sky),
        9 => Some(PaletteColor::Cloud),
        10 => Some(PaletteColor::Sand),
        11 => Some(PaletteColor::Clay),
        12 => Some(PaletteColor::Peach),
        13 => Some(PaletteColor::Rose),
        14 => Some(PaletteColor::Lavender),
        15 => Some(PaletteColor::Slate),
        _ => None,
    }
}

fn format_to_raw(format: CellFormat) -> DvcCellFormatRaw {
    DvcCellFormatRaw {
        has_decimals: if format.decimals.is_some() { 1 } else { 0 },
        decimals: format.decimals.unwrap_or(0),
        _padding: [0; 3],
        bold: if format.bold { 1 } else { 0 },
        italic: if format.italic { 1 } else { 0 },
        fg: palette_to_raw(format.fg),
        bg: palette_to_raw(format.bg),
    }
}

fn format_from_raw(raw: DvcCellFormatRaw) -> CellFormat {
    CellFormat {
        decimals: if raw.has_decimals != 0 {
            Some(raw.decimals)
        } else {
            None
        },
        bold: raw.bold != 0,
        italic: raw.italic != 0,
        fg: palette_from_raw(raw.fg),
        bg: palette_from_raw(raw.bg),
    }
}

fn value_to_text(value: &Value) -> String {
    match value {
        Value::Number(number) => number.to_string(),
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Blank => String::new(),
        Value::Error(err) => err.excel_tag().to_string(),
    }
}

fn cell_error_kind(error: &CellError) -> i32 {
    match error {
        CellError::DivisionByZero => DVC_ERROR_DIV_ZERO,
        CellError::Value(_) => DVC_ERROR_VALUE,
        CellError::Name(_) => DVC_ERROR_NAME,
        CellError::UnknownName(_) => DVC_ERROR_UNKNOWN_NAME,
        CellError::Ref(_) => DVC_ERROR_REF,
        CellError::Spill(_) => DVC_ERROR_SPILL,
        CellError::Cycle(_) => DVC_ERROR_CYCLE,
        CellError::Na => DVC_ERROR_NA,
        CellError::Null => DVC_ERROR_NULL,
        CellError::Num(_) => DVC_ERROR_NUM,
    }
}

fn cell_error_message(error: &CellError) -> String {
    match error {
        CellError::DivisionByZero => "division by zero".to_string(),
        CellError::Value(msg)
        | CellError::Name(msg)
        | CellError::UnknownName(msg)
        | CellError::Ref(msg)
        | CellError::Spill(msg)
        | CellError::Num(msg) => msg.clone(),
        CellError::Cycle(path) => path.join(" -> "),
        CellError::Na => "#N/A".to_string(),
        CellError::Null => "null intersection".to_string(),
    }
}

fn value_to_raw(value: &Value) -> DvcCellValue {
    match value {
        Value::Number(number) => DvcCellValue {
            value_type: DVC_VALUE_NUMBER,
            number: *number,
            bool_val: 0,
            error_kind: 0,
        },
        Value::Text(_) => DvcCellValue {
            value_type: DVC_VALUE_TEXT,
            number: 0.0,
            bool_val: 0,
            error_kind: 0,
        },
        Value::Bool(flag) => DvcCellValue {
            value_type: DVC_VALUE_BOOL,
            number: 0.0,
            bool_val: if *flag { 1 } else { 0 },
            error_kind: 0,
        },
        Value::Blank => DvcCellValue {
            value_type: DVC_VALUE_BLANK,
            number: 0.0,
            bool_val: 0,
            error_kind: 0,
        },
        Value::Error(err) => DvcCellValue {
            value_type: DVC_VALUE_ERROR,
            number: 0.0,
            bool_val: 0,
            error_kind: cell_error_kind(err),
        },
    }
}

fn cell_state_to_raw(state: &CellState) -> DvcCellState {
    DvcCellState {
        value: value_to_raw(&state.value),
        value_epoch: state.value_epoch,
        stale: if state.stale { 1 } else { 0 },
    }
}

fn input_type_and_text(input: Option<CellInput>) -> (i32, String) {
    match input {
        None => (DVC_INPUT_EMPTY, String::new()),
        Some(CellInput::Number(number)) => (DVC_INPUT_NUMBER, number.to_string()),
        Some(CellInput::Text(text)) => (DVC_INPUT_TEXT, text),
        Some(CellInput::Formula(formula)) => (DVC_INPUT_FORMULA, formula),
    }
}

fn name_input_type_and_text(input: Option<NameInput>) -> (i32, String) {
    match input {
        None => (DVC_INPUT_EMPTY, String::new()),
        Some(NameInput::Number(number)) => (DVC_INPUT_NUMBER, number.to_string()),
        Some(NameInput::Text(text)) => (DVC_INPUT_TEXT, text),
        Some(NameInput::Formula(formula)) => (DVC_INPUT_FORMULA, formula),
    }
}

fn should_advance_string_entry(buf: *mut u8, buf_len: u32, text: &str) -> bool {
    if buf.is_null() {
        return false;
    }
    let required = match u32::try_from(text.as_bytes().len()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    buf_len >= required
}

unsafe fn unsupported_with_engine(engine: DvcEngineHandle, symbol: &str) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    fail(
        handle,
        DVC_ERR_UNSUPPORTED,
        format!("{symbol} is not implemented in dnavisicalc_coreengine_rust"),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_api_version() -> u32 {
    DVC_API_VERSION_PACKED
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_create(out: *mut DvcEngineHandle) -> i32 {
    if out.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let boxed = Box::new(EngineHandle {
        engine: Engine::new(),
        last_error: String::new(),
    });
    // SAFETY: `out` is validated non-null and points to writable memory from caller.
    unsafe {
        *out = Box::into_raw(boxed).cast::<c_void>();
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_create_with_bounds(
    bounds: DvcSheetBounds,
    out: *mut DvcEngineHandle,
) -> i32 {
    if out.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    if bounds.max_columns == 0 || bounds.max_rows == 0 {
        return DVC_ERR_INVALID_ARGUMENT;
    }
    let boxed = Box::new(EngineHandle {
        engine: Engine::with_bounds(SheetBounds {
            max_columns: bounds.max_columns,
            max_rows: bounds.max_rows,
        }),
        last_error: String::new(),
    });
    // SAFETY: `out` is validated non-null and points to writable memory from caller.
    unsafe {
        *out = Box::into_raw(boxed).cast::<c_void>();
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_destroy(engine: DvcEngineHandle) -> i32 {
    if engine.is_null() {
        return DVC_OK;
    }
    // SAFETY: `engine` was allocated by `Box::into_raw` in create APIs.
    unsafe {
        drop(Box::from_raw(engine.cast::<EngineHandle>()));
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_clear(engine: DvcEngineHandle) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    handle.engine.clear();
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_bounds(
    engine: DvcEngineHandle,
    out_bounds: *mut DvcSheetBounds,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_bounds.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_bounds: out_bounds is null",
        );
    }
    let bounds = handle.engine.bounds();
    // SAFETY: `out_bounds` is validated non-null.
    unsafe {
        *out_bounds = DvcSheetBounds {
            max_columns: bounds.max_columns,
            max_rows: bounds.max_rows,
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_get_recalc_mode(
    engine: DvcEngineHandle,
    out_mode: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_mode.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_get_recalc_mode: out_mode is null",
        );
    }
    let mode = match handle.engine.recalc_mode() {
        RecalcMode::Automatic => DVC_RECALC_AUTOMATIC,
        RecalcMode::Manual => DVC_RECALC_MANUAL,
    };
    // SAFETY: `out_mode` is validated non-null.
    unsafe {
        *out_mode = mode;
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_set_recalc_mode(engine: DvcEngineHandle, mode: i32) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let mapped = match mode {
        DVC_RECALC_AUTOMATIC => RecalcMode::Automatic,
        DVC_RECALC_MANUAL => RecalcMode::Manual,
        _ => {
            return fail(
                handle,
                DVC_ERR_INVALID_ARGUMENT,
                format!("dvc_engine_set_recalc_mode: unknown mode {mode}"),
            );
        }
    };
    handle.engine.set_recalc_mode(mapped);
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_committed_epoch(
    engine: DvcEngineHandle,
    out_epoch: *mut u64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_epoch.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_committed_epoch: out_epoch is null",
        );
    }
    // SAFETY: `out_epoch` is validated non-null.
    unsafe {
        *out_epoch = handle.engine.committed_epoch();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_stabilized_epoch(
    engine: DvcEngineHandle,
    out_epoch: *mut u64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_epoch.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_stabilized_epoch: out_epoch is null",
        );
    }
    // SAFETY: `out_epoch` is validated non-null.
    unsafe {
        *out_epoch = handle.engine.stabilized_epoch();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_recalculate(engine: DvcEngineHandle) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.recalculate()) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_has_stream_cells(engine: DvcEngineHandle, out: *mut i32) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_has_stream_cells: out is null",
        );
    }
    // SAFETY: `out` is validated non-null.
    unsafe {
        *out = if handle.engine.has_stream_cells() {
            1
        } else {
            0
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_tick_streams(
    engine: DvcEngineHandle,
    elapsed_secs: f64,
    any_advanced: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if any_advanced.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_tick_streams: any_advanced is null",
        );
    }
    if !elapsed_secs.is_finite() || elapsed_secs < 0.0 {
        return fail(
            handle,
            DVC_ERR_INVALID_ARGUMENT,
            "dvc_tick_streams: elapsed_secs must be finite and >= 0",
        );
    }
    let advanced = handle.engine.tick_streams(elapsed_secs);
    // SAFETY: `any_advanced` is validated non-null.
    unsafe {
        *any_advanced = if advanced { 1 } else { 0 };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_get_iteration_config(
    engine: DvcEngineHandle,
    out_cfg: *mut DvcIterationConfigRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_cfg.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_get_iteration_config: out_cfg is null",
        );
    }
    let cfg = iteration_config_to_raw(handle.engine.iteration_config());
    // SAFETY: `out_cfg` is validated non-null.
    unsafe {
        *out_cfg = cfg;
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_engine_set_iteration_config(
    engine: DvcEngineHandle,
    cfg: *const DvcIterationConfigRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if cfg.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_set_iteration_config: cfg is null",
        );
    }
    // SAFETY: `cfg` is validated non-null.
    let config = unsafe { raw_to_iteration_config(*cfg) };
    handle.engine.set_iteration_config(config);
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_last_error_message(
    engine: DvcEngineHandle,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    write_utf8(&handle.last_error, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_number(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    value: f64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_number(cell, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_text(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    text: *const u8,
    text_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let value = match read_text_arg(handle, text, text_len, "dvc_cell_set_text") {
        Ok(value) => value,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_text(cell, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_formula(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    formula: *const u8,
    formula_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let value = match read_text_arg(handle, formula, formula_len, "dvc_cell_set_formula") {
        Ok(value) => value,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_formula(cell, &value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_clear(engine: DvcEngineHandle, addr: DvcCellAddr) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.clear_cell(cell)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_state(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_state: *mut DvcCellState,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_state.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_get_state: out_state is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let state = match engine_call!(handle, handle.engine.cell_state(cell)) {
        Ok(state) => state,
        Err(status) => return status,
    };
    // SAFETY: `out_state` is validated non-null.
    unsafe {
        *out_state = cell_state_to_raw(&state);
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_text(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let state = match engine_call!(handle, handle.engine.cell_state(cell)) {
        Ok(state) => state,
        Err(status) => return status,
    };
    let status = write_utf8(&value_to_text(&state.value), buf, buf_len, out_len);
    if status == DVC_OK {
        handle.clear_error();
    } else {
        handle.set_error("dvc_cell_get_text: invalid output buffer");
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_input_type(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_type: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_type.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_get_input_type: out_type is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let input = match engine_call!(handle, handle.engine.cell_input(cell)) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let (input_type, _) = input_type_and_text(input);
    // SAFETY: `out_type` is validated non-null.
    unsafe {
        *out_type = input_type;
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_input_text(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let input = match engine_call!(handle, handle.engine.cell_input(cell)) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let (_, text) = input_type_and_text(input);
    let status = write_utf8(&text, buf, buf_len, out_len);
    if status == DVC_OK {
        handle.clear_error();
    } else {
        handle.set_error("dvc_cell_get_input_text: invalid output buffer");
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_error_message(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let state = match engine_call!(handle, handle.engine.cell_state(cell)) {
        Ok(state) => state,
        Err(status) => return status,
    };
    let message = match &state.value {
        Value::Error(err) => cell_error_message(err),
        _ => String::new(),
    };
    let status = write_utf8(&message, buf, buf_len, out_len);
    if status == DVC_OK {
        handle.clear_error();
    } else {
        handle.set_error("dvc_cell_error_message: invalid output buffer");
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_set_number(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    value: f64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_name_set_number") {
        Ok(name) => name,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_name_number(&name, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_set_text(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    text: *const u8,
    text_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_name_set_text(name)") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let text = match read_text_arg(handle, text, text_len, "dvc_name_set_text(text)") {
        Ok(text) => text,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_name_text(&name, text)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_set_formula(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    formula: *const u8,
    formula_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_name_set_formula(name)") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let formula = match read_text_arg(
        handle,
        formula,
        formula_len,
        "dvc_name_set_formula(formula)",
    ) {
        Ok(formula) => formula,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_name_formula(&name, &formula)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_clear(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_name_clear") {
        Ok(name) => name,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.clear_name(&name)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_get_input_type(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    out_type: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_type.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_name_get_input_type: out_type is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_name_get_input_type") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let input = match engine_call!(handle, handle.engine.name_input(&name)) {
        Ok(input) => input,
        Err(status) => return status,
    };
    let (input_type, _) = name_input_type_and_text(input);
    // SAFETY: `out_type` is validated non-null.
    unsafe {
        *out_type = input_type;
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_get_input_text(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_name_get_input_text") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let input = match engine_call!(handle, handle.engine.name_input(&name)) {
        Ok(input) => input,
        Err(status) => return status,
    };
    let (_, text) = name_input_type_and_text(input);
    let status = write_utf8(&text, buf, buf_len, out_len);
    if status == DVC_OK {
        handle.clear_error();
    } else {
        handle.set_error("dvc_name_get_input_text: invalid output buffer");
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_format(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_format: *mut DvcCellFormatRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_format.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_get_format: out_format is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let format = match engine_call!(handle, handle.engine.cell_format(cell)) {
        Ok(format) => format,
        Err(status) => return status,
    };
    // SAFETY: `out_format` is validated non-null.
    unsafe {
        *out_format = format_to_raw(format);
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_format(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    format: *const DvcCellFormatRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if format.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_set_format: format is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    // SAFETY: `format` is validated non-null.
    let converted = unsafe { format_from_raw(*format) };
    match engine_call!(handle, handle.engine.set_cell_format(cell, converted)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_spill_anchor(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_anchor: *mut DvcCellAddr,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_anchor.is_null() || found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_spill_anchor: out_anchor/found is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let anchor = match engine_call!(handle, handle.engine.spill_anchor_for_cell(cell)) {
        Ok(anchor) => anchor,
        Err(status) => return status,
    };
    // SAFETY: outputs are validated non-null.
    unsafe {
        if let Some(anchor) = anchor {
            *found = 1;
            *out_anchor = cell_to_addr(anchor);
        } else {
            *found = 0;
            *out_anchor = DvcCellAddr::default();
        }
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_spill_range(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_range: *mut DvcCellRange,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_range.is_null() || found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_spill_range: out_range/found is null",
        );
    }
    let cell = match cell_from_addr(handle, addr) {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let range = match engine_call!(handle, handle.engine.spill_range_for_cell(cell)) {
        Ok(range) => range,
        Err(status) => return status,
    };
    // SAFETY: outputs are validated non-null.
    unsafe {
        if let Some(range) = range {
            *found = 1;
            *out_range = range_to_raw(range);
        } else {
            *found = 0;
            *out_range = DvcCellRange::default();
        }
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_insert_row(engine: DvcEngineHandle, at: u16) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.insert_row(at)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_delete_row(engine: DvcEngineHandle, at: u16) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.delete_row(at)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_insert_col(engine: DvcEngineHandle, at: u16) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.insert_col(at)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_delete_col(engine: DvcEngineHandle, at: u16) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.delete_col(at)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_parse_cell_ref(
    engine: DvcEngineHandle,
    text: *const u8,
    text_len: u32,
    out_addr: *mut DvcCellAddr,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_addr.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_parse_cell_ref: out_addr is null",
        );
    }
    let text = match read_text_arg(handle, text, text_len, "dvc_parse_cell_ref") {
        Ok(text) => text,
        Err(status) => return status,
    };
    let parsed = match CellRef::from_a1_with_bounds(&text, handle.engine.bounds()) {
        Ok(cell) => cell,
        Err(err) => {
            return fail(
                handle,
                status_for_address_error(&err),
                format!("invalid cell reference '{text}': {err}"),
            );
        }
    };
    // SAFETY: `out_addr` is validated non-null.
    unsafe {
        *out_addr = cell_to_addr(parsed);
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_iterate(
    engine: DvcEngineHandle,
    out_iter: *mut DvcIteratorHandle,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_iter.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_iterate: out_iter is null",
        );
    }
    let entries = handle
        .engine
        .all_cell_inputs()
        .into_iter()
        .map(|(cell, input)| {
            let (input_type, text) = input_type_and_text(Some(input));
            CellIteratorEntry {
                addr: cell_to_addr(cell),
                input_type,
                text,
            }
        })
        .collect();
    let iterator = IteratorHandle::Cell(CellIteratorState {
        entries,
        index: 0,
        current_text: String::new(),
    });
    // SAFETY: `out_iter` is validated non-null.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_iterator_next(
    iterator: DvcIteratorHandle,
    out_addr: *mut DvcCellAddr,
    out_type: *mut i32,
    done: *mut i32,
) -> i32 {
    if out_addr.is_null() || out_type.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Cell(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: pointers are validated non-null above.
        unsafe {
            *done = 1;
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    state.index += 1;
    state.current_text = entry.text;
    // SAFETY: pointers are validated non-null above.
    unsafe {
        *out_addr = entry.addr;
        *out_type = entry.input_type;
        *done = 0;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_iterator_get_text(
    iterator: DvcIteratorHandle,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Cell(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    write_utf8(&state.current_text, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
    if iterator.is_null() {
        return DVC_OK;
    }
    // SAFETY: `iterator` is allocated by `Box::into_raw` in iterate APIs.
    unsafe {
        drop(Box::from_raw(iterator.cast::<IteratorHandle>()));
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_iterate(
    engine: DvcEngineHandle,
    out_iter: *mut DvcIteratorHandle,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_iter.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_name_iterate: out_iter is null",
        );
    }
    let entries = handle
        .engine
        .all_name_inputs()
        .into_iter()
        .map(|(name, input)| {
            let (input_type, text) = name_input_type_and_text(Some(input));
            NameIteratorEntry {
                name,
                input_type,
                text,
            }
        })
        .collect();
    let iterator = IteratorHandle::Name(NameIteratorState {
        entries,
        index: 0,
        current_text: String::new(),
    });
    // SAFETY: `out_iter` is validated non-null.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_iterator_next(
    iterator: DvcIteratorHandle,
    name_buf: *mut u8,
    name_buf_len: u32,
    name_len: *mut u32,
    out_type: *mut i32,
    done: *mut i32,
) -> i32 {
    if name_len.is_null() || out_type.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Name(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: output pointers are validated non-null above.
        unsafe {
            *done = 1;
            *name_len = 0;
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    // SAFETY: output pointers are validated non-null above.
    unsafe {
        *out_type = entry.input_type;
        *done = 0;
    }
    let status = write_utf8(&entry.name, name_buf, name_buf_len, name_len);
    if status != DVC_OK {
        return status;
    }
    if should_advance_string_entry(name_buf, name_buf_len, &entry.name) {
        state.current_text = entry.text;
        state.index += 1;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_iterator_get_text(
    iterator: DvcIteratorHandle,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Name(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    write_utf8(&state.current_text, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_name_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
    if iterator.is_null() {
        return DVC_OK;
    }
    // SAFETY: `iterator` is allocated by `Box::into_raw` in iterate APIs.
    unsafe {
        drop(Box::from_raw(iterator.cast::<IteratorHandle>()));
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_format_iterate(
    engine: DvcEngineHandle,
    out_iter: *mut DvcIteratorHandle,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_iter.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_format_iterate: out_iter is null",
        );
    }
    let entries = handle
        .engine
        .all_cell_formats()
        .into_iter()
        .map(|(cell, format)| FormatIteratorEntry {
            addr: cell_to_addr(cell),
            format: format_to_raw(format),
        })
        .collect();
    let iterator = IteratorHandle::Format(FormatIteratorState { entries, index: 0 });
    // SAFETY: `out_iter` is validated non-null.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_format_iterator_next(
    iterator: DvcIteratorHandle,
    out_addr: *mut DvcCellAddr,
    out_format: *mut DvcCellFormatRaw,
    done: *mut i32,
) -> i32 {
    if out_addr.is_null() || out_format.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Format(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: `done` is validated non-null.
        unsafe {
            *done = 1;
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    state.index += 1;
    // SAFETY: pointers are validated non-null above.
    unsafe {
        *out_addr = entry.addr;
        *out_format = entry.format;
        *done = 0;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_format_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
    if iterator.is_null() {
        return DVC_OK;
    }
    // SAFETY: `iterator` is allocated by `Box::into_raw` in iterate APIs.
    unsafe {
        drop(Box::from_raw(iterator.cast::<IteratorHandle>()));
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_define(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    _def: *const DvcControlDefRaw,
) -> i32 {
    unsafe { unsupported_with_engine(engine, "dvc_control_define") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_remove(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    found: *mut i32,
) -> i32 {
    if !found.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *found = 0;
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_control_remove") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_set_value(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    _value: f64,
) -> i32 {
    unsafe { unsupported_with_engine(engine, "dvc_control_set_value") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_get_value(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    out_value: *mut f64,
    found: *mut i32,
) -> i32 {
    if !out_value.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_value = 0.0;
        }
    }
    if !found.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *found = 0;
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_control_get_value") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_get_def(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    out_def: *mut DvcControlDefRaw,
    found: *mut i32,
) -> i32 {
    if !out_def.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_def = DvcControlDefRaw {
                kind: DVC_CONTROL_SLIDER,
                min: 0.0,
                max: 0.0,
                step: 0.0,
            };
        }
    }
    if !found.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *found = 0;
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_control_get_def") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterate(
    engine: DvcEngineHandle,
    out_iter: *mut DvcIteratorHandle,
) -> i32 {
    if !out_iter.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_iter = ptr::null_mut();
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_control_iterate") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterator_next(
    _iterator: DvcIteratorHandle,
    _name_buf: *mut u8,
    _name_buf_len: u32,
    name_len: *mut u32,
    out_def: *mut DvcControlDefRaw,
    out_value: *mut f64,
    done: *mut i32,
) -> i32 {
    if !name_len.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *name_len = 0;
        }
    }
    if !out_def.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_def = DvcControlDefRaw::default();
        }
    }
    if !out_value.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_value = 0.0;
        }
    }
    if !done.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *done = 1;
        }
    }
    DVC_ERR_UNSUPPORTED
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterator_destroy(_iterator: DvcIteratorHandle) -> i32 {
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_define(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    _def: *const DvcChartDefRaw,
) -> i32 {
    unsafe { unsupported_with_engine(engine, "dvc_chart_define") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_remove(
    engine: DvcEngineHandle,
    _name: *const u8,
    _name_len: u32,
    found: *mut i32,
) -> i32 {
    if !found.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *found = 0;
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_chart_remove") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterate(
    engine: DvcEngineHandle,
    out_iter: *mut DvcIteratorHandle,
) -> i32 {
    if !out_iter.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_iter = ptr::null_mut();
        }
    }
    unsafe { unsupported_with_engine(engine, "dvc_chart_iterate") }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterator_next(
    _iterator: DvcIteratorHandle,
    _name_buf: *mut u8,
    _name_buf_len: u32,
    name_len: *mut u32,
    out_def: *mut DvcChartDefRaw,
    done: *mut i32,
) -> i32 {
    if !name_len.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *name_len = 0;
        }
    }
    if !out_def.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *out_def = DvcChartDefRaw::default();
        }
    }
    if !done.is_null() {
        // SAFETY: Caller-provided out pointer is checked non-null.
        unsafe {
            *done = 1;
        }
    }
    DVC_ERR_UNSUPPORTED
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterator_destroy(_iterator: DvcIteratorHandle) -> i32 {
    DVC_OK
}
