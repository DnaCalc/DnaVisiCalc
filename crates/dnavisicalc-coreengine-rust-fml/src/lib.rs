use std::ffi::c_void;
use std::ptr;

use dnavisicalc_core::{
    AddressError, CellError, CellFormat, CellInput, CellRange, CellRef, CellState, ChangeEntry,
    ChartDefinition, ChartOutput, ControlDefinition, ControlKind, DiagnosticCode, Engine,
    EngineError, IterationConfig, NameInput, PaletteColor, RecalcMode, SheetBounds, UdfHandler,
    Value, Volatility,
};

const DVC_OK: i32 = 0;
const DVC_REJECT_STRUCTURAL_CONSTRAINT: i32 = 1;
const DVC_REJECT_POLICY: i32 = 2;

const DVC_ERR_NULL_POINTER: i32 = -1;
const DVC_ERR_OUT_OF_BOUNDS: i32 = -2;
const DVC_ERR_INVALID_ADDRESS: i32 = -3;
const DVC_ERR_PARSE: i32 = -4;
const DVC_ERR_DEPENDENCY: i32 = -5;
const DVC_ERR_INVALID_NAME: i32 = -6;
const DVC_ERR_INVALID_ARGUMENT: i32 = -8;

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

const DVC_SPILL_NONE: i32 = 0;
const DVC_SPILL_ANCHOR: i32 = 1;
const DVC_SPILL_MEMBER: i32 = 2;

const DVC_PALETTE_NONE: i32 = -1;

const DVC_CONTROL_SLIDER: i32 = 0;
const DVC_CONTROL_CHECKBOX: i32 = 1;
const DVC_CONTROL_BUTTON: i32 = 2;

const DVC_VOLATILITY_STANDARD: i32 = 0;
const DVC_VOLATILITY_VOLATILE: i32 = 1;
const DVC_VOLATILITY_EXTERNALLY_INVALIDATED: i32 = 2;

const DVC_CHANGE_CELL_VALUE: i32 = 0;
const DVC_CHANGE_NAME_VALUE: i32 = 1;
const DVC_CHANGE_CHART_OUTPUT: i32 = 2;
const DVC_CHANGE_SPILL_REGION: i32 = 3;
const DVC_CHANGE_CELL_FORMAT: i32 = 4;
const DVC_CHANGE_DIAGNOSTIC: i32 = 5;

const DVC_DIAG_CIRCULAR_REFERENCE_DETECTED: i32 = 0;

const DVC_STRUCT_OP_INSERT_ROW: i32 = 1;
const DVC_STRUCT_OP_DELETE_ROW: i32 = 2;
const DVC_STRUCT_OP_INSERT_COL: i32 = 3;
const DVC_STRUCT_OP_DELETE_COL: i32 = 4;

const DVC_REJECT_KIND_NONE: i32 = 0;
const DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT: i32 = 1;
const DVC_REJECT_KIND_POLICY: i32 = 2;

const DVC_API_VERSION_PACKED: u32 = (0u32 << 16) | (1u32 << 8);

type DvcEngineHandle = *mut c_void;
type DvcIteratorHandle = *mut c_void;
type DvcChartOutputHandle = *mut c_void;
type DvcUdfCallback = unsafe extern "C" fn(
    user_data: *mut c_void,
    args: *const DvcCellValue,
    arg_count: u32,
    out: *mut DvcCellValue,
) -> i32;

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

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DvcLastRejectContextRaw {
    reject_kind: i32,
    op_kind: i32,
    op_index: u16,
    has_cell: i32,
    cell: DvcCellAddr,
    has_range: i32,
    range: DvcCellRange,
}

struct CApiUdf {
    callback: DvcUdfCallback,
    user_data: *mut c_void,
    volatility: Volatility,
}

impl std::fmt::Debug for CApiUdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CApiUdf(<callback>)")
    }
}

impl UdfHandler for CApiUdf {
    fn call(&self, args: &[Value]) -> Value {
        let mut raw_args = Vec::with_capacity(args.len());
        for value in args {
            raw_args.push(udf_value_to_raw(value));
        }
        let mut out = DvcCellValue::default();
        let arg_ptr = if raw_args.is_empty() {
            std::ptr::null()
        } else {
            raw_args.as_ptr()
        };
        // SAFETY: The callback/user_data pair is provided by the embedding host.
        let status = unsafe {
            (self.callback)(
                self.user_data,
                arg_ptr,
                raw_args.len() as u32,
                std::ptr::addr_of_mut!(out),
            )
        };
        if status != DVC_OK {
            return Value::Error(CellError::Value(format!(
                "udf callback failed with status {status}"
            )));
        }
        udf_value_from_raw(out)
    }

    fn volatility(&self) -> Volatility {
        self.volatility
    }
}

struct ChartOutputHandle {
    output: ChartOutput,
}

struct EngineHandle {
    engine: Engine,
    last_error: String,
    last_error_kind: i32,
    last_reject_kind: i32,
    last_reject_context: DvcLastRejectContextRaw,
    chart_output_handles: Vec<*mut ChartOutputHandle>,
}

impl EngineHandle {
    fn mark_ok(&mut self) {
        self.last_error.clear();
        self.last_error_kind = DVC_OK;
        self.last_reject_kind = DVC_REJECT_KIND_NONE;
        self.last_reject_context = DvcLastRejectContextRaw::default();
    }

    fn mark_error(&mut self, status: i32, message: impl Into<String>) {
        self.last_error = message.into();
        self.last_error_kind = status;
        self.last_reject_kind = DVC_REJECT_KIND_NONE;
        self.last_reject_context = DvcLastRejectContextRaw::default();
    }

    fn mark_reject(&mut self, reject_status: i32, reject_context: DvcLastRejectContextRaw) {
        let reject_kind = match reject_status {
            DVC_REJECT_STRUCTURAL_CONSTRAINT => DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT,
            DVC_REJECT_POLICY => DVC_REJECT_KIND_POLICY,
            _ => DVC_REJECT_KIND_NONE,
        };
        self.last_error.clear();
        self.last_error_kind = DVC_OK;
        self.last_reject_kind = reject_kind;
        self.last_reject_context = reject_context;
    }

    fn clear_error(&mut self) {
        self.mark_ok();
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.mark_error(DVC_ERR_INVALID_ARGUMENT, message);
    }

    fn register_chart_output_handle(&mut self, output: ChartOutput) -> DvcChartOutputHandle {
        let ptr = Box::into_raw(Box::new(ChartOutputHandle { output }));
        self.chart_output_handles.push(ptr);
        ptr.cast::<c_void>()
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        for ptr in self.chart_output_handles.drain(..) {
            // SAFETY: pointers were created via Box::into_raw in register_chart_output_handle.
            unsafe {
                drop(Box::from_raw(ptr));
            }
        }
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

#[derive(Clone)]
struct ControlIteratorEntry {
    name: String,
    def: DvcControlDefRaw,
    value: f64,
}

struct ControlIteratorState {
    entries: Vec<ControlIteratorEntry>,
    index: usize,
}

#[derive(Clone)]
struct ChartIteratorEntry {
    name: String,
    def: DvcChartDefRaw,
}

struct ChartIteratorState {
    entries: Vec<ChartIteratorEntry>,
    index: usize,
}

struct ChangeIteratorState {
    entries: Vec<ChangeEntry>,
    index: usize,
    current: Option<ChangeEntry>,
}

enum IteratorHandle {
    Cell(CellIteratorState),
    Name(NameIteratorState),
    Format(FormatIteratorState),
    Control(ControlIteratorState),
    Chart(ChartIteratorState),
    Change(ChangeIteratorState),
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
            handle.mark_error(status, err.to_string());
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
    handle.mark_error(status, message);
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
            handle.mark_error(status, format!("{label} must be valid UTF-8"));
            Err(status)
        }
    }
}

fn cell_from_addr(handle: &mut EngineHandle, addr: DvcCellAddr) -> Result<CellRef, i32> {
    match CellRef::new(addr.col, addr.row, handle.engine.bounds()) {
        Ok(cell) => Ok(cell),
        Err(err) => {
            let status = status_for_address_error(&err);
            handle.mark_error(status, err.to_string());
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

fn control_def_to_raw(def: ControlDefinition) -> DvcControlDefRaw {
    DvcControlDefRaw {
        kind: match def.kind {
            ControlKind::Slider => DVC_CONTROL_SLIDER,
            ControlKind::Checkbox => DVC_CONTROL_CHECKBOX,
            ControlKind::Button => DVC_CONTROL_BUTTON,
        },
        min: def.min,
        max: def.max,
        step: def.step,
    }
}

fn control_def_from_raw(raw: DvcControlDefRaw) -> ControlDefinition {
    let kind = match raw.kind {
        DVC_CONTROL_CHECKBOX => ControlKind::Checkbox,
        DVC_CONTROL_BUTTON => ControlKind::Button,
        _ => ControlKind::Slider,
    };
    ControlDefinition {
        kind,
        min: raw.min,
        max: raw.max,
        step: raw.step,
    }
}

fn chart_def_to_raw(def: ChartDefinition) -> DvcChartDefRaw {
    DvcChartDefRaw {
        source_range: range_to_raw(def.source_range),
    }
}

fn chart_def_from_raw(
    handle: &mut EngineHandle,
    raw: DvcChartDefRaw,
) -> Result<ChartDefinition, i32> {
    let start = cell_from_addr(handle, raw.source_range.start)?;
    let end = cell_from_addr(handle, raw.source_range.end)?;
    Ok(ChartDefinition {
        source_range: CellRange::new(start, end),
    })
}

fn volatility_from_raw(raw: i32) -> Result<Volatility, i32> {
    match raw {
        DVC_VOLATILITY_STANDARD => Ok(Volatility::Standard),
        DVC_VOLATILITY_VOLATILE => Ok(Volatility::Volatile),
        DVC_VOLATILITY_EXTERNALLY_INVALIDATED => Ok(Volatility::ExternallyInvalidated),
        _ => Err(DVC_ERR_INVALID_ARGUMENT),
    }
}

fn udf_value_to_raw(value: &Value) -> DvcCellValue {
    value_to_raw(value)
}

fn udf_value_from_raw(raw: DvcCellValue) -> Value {
    match raw.value_type {
        DVC_VALUE_NUMBER => Value::Number(raw.number),
        DVC_VALUE_TEXT => Value::Text(String::new()),
        DVC_VALUE_BOOL => Value::Bool(raw.bool_val != 0),
        DVC_VALUE_BLANK => Value::Blank,
        DVC_VALUE_ERROR => Value::Error(match raw.error_kind {
            DVC_ERROR_DIV_ZERO => CellError::DivisionByZero,
            DVC_ERROR_VALUE => CellError::Value("udf value error".to_string()),
            DVC_ERROR_NAME => CellError::Name("udf name error".to_string()),
            DVC_ERROR_UNKNOWN_NAME => CellError::UnknownName("udf unknown name".to_string()),
            DVC_ERROR_REF => CellError::Ref("udf ref error".to_string()),
            DVC_ERROR_SPILL => CellError::Spill("udf spill error".to_string()),
            DVC_ERROR_CYCLE => CellError::Cycle(vec!["udf cycle".to_string()]),
            DVC_ERROR_NA => CellError::Na,
            DVC_ERROR_NULL => CellError::Null,
            DVC_ERROR_NUM => CellError::Num("udf num error".to_string()),
            _ => CellError::Value("udf error".to_string()),
        }),
        _ => Value::Blank,
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

fn parse_a1_cell(
    handle: &mut EngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    label: &str,
) -> Result<CellRef, i32> {
    let text = read_text_arg(handle, cell_ref, cell_ref_len, label)?;
    match CellRef::from_a1_with_bounds(&text, handle.engine.bounds()) {
        Ok(cell) => Ok(cell),
        Err(err) => {
            let status = status_for_address_error(&err);
            handle.mark_error(status, err.to_string());
            Err(status)
        }
    }
}

fn intersects_spill_range(range: CellRange, op_kind: i32, at: u16) -> bool {
    match op_kind {
        DVC_STRUCT_OP_INSERT_ROW | DVC_STRUCT_OP_DELETE_ROW => {
            at >= range.start.row && at <= range.end.row
        }
        DVC_STRUCT_OP_INSERT_COL | DVC_STRUCT_OP_DELETE_COL => {
            at >= range.start.col && at <= range.end.col
        }
        _ => false,
    }
}

fn find_structural_reject_context(
    handle: &EngineHandle,
    op_kind: i32,
    at: u16,
) -> Option<DvcLastRejectContextRaw> {
    for (cell, _) in handle.engine.all_cell_inputs() {
        let Ok(range_opt) = handle.engine.spill_range_for_cell(cell) else {
            continue;
        };
        let Some(range) = range_opt else {
            continue;
        };
        if intersects_spill_range(range, op_kind, at) {
            return Some(DvcLastRejectContextRaw {
                reject_kind: DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT,
                op_kind,
                op_index: at,
                has_cell: 1,
                cell: cell_to_addr(cell),
                has_range: 1,
                range: range_to_raw(range),
            });
        }
    }
    None
}

fn maybe_reject_structural(handle: &mut EngineHandle, op_kind: i32, at: u16) -> Option<i32> {
    let context = find_structural_reject_context(handle, op_kind, at)?;
    handle.mark_reject(DVC_REJECT_STRUCTURAL_CONSTRAINT, context);
    Some(DVC_REJECT_STRUCTURAL_CONSTRAINT)
}

fn palette_color_name(raw: i32) -> Option<&'static str> {
    match raw {
        0 => Some("MIST"),
        1 => Some("SAGE"),
        2 => Some("FERN"),
        3 => Some("MOSS"),
        4 => Some("OLIVE"),
        5 => Some("SEAFOAM"),
        6 => Some("LAGOON"),
        7 => Some("TEAL"),
        8 => Some("SKY"),
        9 => Some("CLOUD"),
        10 => Some("SAND"),
        11 => Some("CLAY"),
        12 => Some("PEACH"),
        13 => Some("ROSE"),
        14 => Some("LAVENDER"),
        15 => Some("SLATE"),
        _ => None,
    }
}

fn change_entry_type_and_epoch(entry: &ChangeEntry) -> (i32, u64) {
    match entry {
        ChangeEntry::CellValue { epoch, .. } => (DVC_CHANGE_CELL_VALUE, *epoch),
        ChangeEntry::NameValue { epoch, .. } => (DVC_CHANGE_NAME_VALUE, *epoch),
        ChangeEntry::ChartOutput { epoch, .. } => (DVC_CHANGE_CHART_OUTPUT, *epoch),
        ChangeEntry::SpillRegion { epoch, .. } => (DVC_CHANGE_SPILL_REGION, *epoch),
        ChangeEntry::CellFormat { epoch, .. } => (DVC_CHANGE_CELL_FORMAT, *epoch),
        ChangeEntry::Diagnostic { epoch, .. } => (DVC_CHANGE_DIAGNOSTIC, *epoch),
    }
}

unsafe fn chart_output_handle<'a>(
    output: DvcChartOutputHandle,
) -> Result<&'a ChartOutputHandle, i32> {
    if output.is_null() {
        return Err(DVC_ERR_NULL_POINTER);
    }
    let ptr = output.cast::<ChartOutputHandle>();
    // SAFETY: The pointer is created from Box::into_raw in dvc_chart_get_output.
    unsafe { ptr.as_ref().ok_or(DVC_ERR_NULL_POINTER) }
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
        last_error_kind: DVC_OK,
        last_reject_kind: DVC_REJECT_KIND_NONE,
        last_reject_context: DvcLastRejectContextRaw::default(),
        chart_output_handles: Vec::new(),
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
        last_error_kind: DVC_OK,
        last_reject_kind: DVC_REJECT_KIND_NONE,
        last_reject_context: DvcLastRejectContextRaw::default(),
        chart_output_handles: Vec::new(),
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
pub unsafe extern "C" fn dvc_engine_is_stable(
    engine: DvcEngineHandle,
    out_stable: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_stable.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_engine_is_stable: out_stable is null",
        );
    }
    // SAFETY: `out_stable` is validated non-null above.
    unsafe {
        *out_stable = if handle.engine.stabilized_epoch() == handle.engine.committed_epoch() {
            1
        } else {
            0
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_has_volatile_cells(engine: DvcEngineHandle, out: *mut i32) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_has_volatile_cells: out is null",
        );
    }
    // SAFETY: `out` is validated non-null above.
    unsafe {
        *out = if handle.engine.has_volatile_cells() {
            1
        } else {
            0
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_has_externally_invalidated_cells(
    engine: DvcEngineHandle,
    out: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_has_externally_invalidated_cells: out is null",
        );
    }
    // SAFETY: `out` is validated non-null above.
    unsafe {
        *out = if handle.engine.has_externally_invalidated_cells() {
            1
        } else {
            0
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_invalidate_volatile(engine: DvcEngineHandle) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.invalidate_volatile()) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_invalidate_udf(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_invalidate_udf name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.invalidate_udf(&name)) {
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
pub unsafe extern "C" fn dvc_last_error_kind(engine: DvcEngineHandle, out_status: *mut i32) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_status.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_last_error_kind: out_status is null",
        );
    }
    // SAFETY: `out_status` is validated non-null above.
    unsafe {
        *out_status = handle.last_error_kind;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_last_reject_kind(engine: DvcEngineHandle, out_kind: *mut i32) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_kind.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_last_reject_kind: out_kind is null",
        );
    }
    // SAFETY: `out_kind` is validated non-null above.
    unsafe {
        *out_kind = handle.last_reject_kind;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_last_reject_context(
    engine: DvcEngineHandle,
    out_ctx: *mut DvcLastRejectContextRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_ctx.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_last_reject_context: out_ctx is null",
        );
    }
    // SAFETY: `out_ctx` is validated non-null above.
    unsafe {
        *out_ctx = handle.last_reject_context;
    }
    DVC_OK
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
pub unsafe extern "C" fn dvc_cell_set_number_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    value: f64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_set_number_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_number(cell, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_text_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    text: *const u8,
    text_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_set_text_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let value = match read_text_arg(handle, text, text_len, "dvc_cell_set_text_a1 text") {
        Ok(value) => value,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_text(cell, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_set_formula_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    formula: *const u8,
    formula_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_set_formula_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let value = match read_text_arg(
        handle,
        formula,
        formula_len,
        "dvc_cell_set_formula_a1 formula",
    ) {
        Ok(value) => value,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_formula(cell, &value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_clear_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_clear_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.clear_cell(cell)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_state_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
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
            "dvc_cell_get_state_a1: out_state is null",
        );
    }
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_get_state_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let state = match engine_call!(handle, handle.engine.cell_state(cell)) {
        Ok(state) => state,
        Err(status) => return status,
    };
    // SAFETY: `out_state` is validated non-null above.
    unsafe {
        *out_state = cell_state_to_raw(&state);
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_text_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_get_text_a1") {
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
        handle.set_error("dvc_cell_get_text_a1: invalid output buffer");
    }
    status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_input_type_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
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
            "dvc_cell_get_input_type_a1: out_type is null",
        );
    }
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_get_input_type_a1") {
        Ok(cell) => cell,
        Err(status) => return status,
    };
    let input = match engine_call!(handle, handle.engine.cell_input(cell)) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let (input_type, _) = input_type_and_text(input);
    // SAFETY: `out_type` is validated non-null above.
    unsafe {
        *out_type = input_type;
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_cell_get_input_text_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_get_input_text_a1") {
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
        handle.set_error("dvc_cell_get_input_text_a1: invalid output buffer");
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
pub unsafe extern "C" fn dvc_cell_get_format_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
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
            "dvc_cell_get_format_a1: out_format is null",
        );
    }
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_get_format_a1") {
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
pub unsafe extern "C" fn dvc_cell_set_format_a1(
    engine: DvcEngineHandle,
    cell_ref: *const u8,
    cell_ref_len: u32,
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
            "dvc_cell_set_format_a1: format is null",
        );
    }
    let cell = match parse_a1_cell(handle, cell_ref, cell_ref_len, "dvc_cell_set_format_a1") {
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
pub unsafe extern "C" fn dvc_cell_spill_role(
    engine: DvcEngineHandle,
    addr: DvcCellAddr,
    out_role: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_role.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_cell_spill_role: out_role is null",
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
    let role = match anchor {
        None => DVC_SPILL_NONE,
        Some(anchor_cell) if anchor_cell == cell => DVC_SPILL_ANCHOR,
        Some(_) => DVC_SPILL_MEMBER,
    };
    // SAFETY: `out_role` is validated non-null above.
    unsafe {
        *out_role = role;
    }
    handle.clear_error();
    DVC_OK
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
    if let Some(status) = maybe_reject_structural(handle, DVC_STRUCT_OP_INSERT_ROW, at) {
        return status;
    }
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
    if let Some(status) = maybe_reject_structural(handle, DVC_STRUCT_OP_DELETE_ROW, at) {
        return status;
    }
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
    if let Some(status) = maybe_reject_structural(handle, DVC_STRUCT_OP_INSERT_COL, at) {
        return status;
    }
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
    if let Some(status) = maybe_reject_structural(handle, DVC_STRUCT_OP_DELETE_COL, at) {
        return status;
    }
    match engine_call!(handle, handle.engine.delete_col(at)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_palette_color_name(
    color: i32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let Some(name) = palette_color_name(color) else {
        if !out_len.is_null() {
            // SAFETY: out_len is checked non-null.
            unsafe {
                *out_len = 0;
            }
        }
        return DVC_ERR_INVALID_ARGUMENT;
    };
    write_utf8(name, buf, buf_len, out_len)
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
    name: *const u8,
    name_len: u32,
    def: *const DvcControlDefRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if def.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_control_define: def is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_control_define name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    // SAFETY: `def` is validated non-null above.
    let converted = unsafe { control_def_from_raw(*def) };
    match engine_call!(handle, handle.engine.define_control(&name, converted)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_remove(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_control_remove: found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_control_remove name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let removed = handle.engine.remove_control(&name);
    // SAFETY: `found` is validated non-null above.
    unsafe {
        *found = if removed { 1 } else { 0 };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_set_value(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    value: f64,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_control_set_value name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.set_control_value(&name, value)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_get_value(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    out_value: *mut f64,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_value.is_null() || found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_control_get_value: out_value/found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_control_get_value name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    // SAFETY: output pointers are validated non-null above.
    unsafe {
        if let Some(value) = handle.engine.control_value(&name) {
            *found = 1;
            *out_value = value;
        } else {
            *found = 0;
            *out_value = 0.0;
        }
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_get_def(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    out_def: *mut DvcControlDefRaw,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_def.is_null() || found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_control_get_def: out_def/found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_control_get_def name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    // SAFETY: output pointers are validated non-null above.
    unsafe {
        if let Some(def) = handle.engine.control_definition(&name).copied() {
            *found = 1;
            *out_def = control_def_to_raw(def);
        } else {
            *found = 0;
            *out_def = DvcControlDefRaw::default();
        }
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterate(
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
            "dvc_control_iterate: out_iter is null",
        );
    }
    let entries = handle
        .engine
        .all_controls()
        .into_iter()
        .map(|(name, def, value)| ControlIteratorEntry {
            name,
            def: control_def_to_raw(def),
            value,
        })
        .collect();
    let iterator = IteratorHandle::Control(ControlIteratorState { entries, index: 0 });
    // SAFETY: `out_iter` is validated non-null above.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterator_next(
    iterator: DvcIteratorHandle,
    name_buf: *mut u8,
    name_buf_len: u32,
    name_len: *mut u32,
    out_def: *mut DvcControlDefRaw,
    out_value: *mut f64,
    done: *mut i32,
) -> i32 {
    if name_len.is_null() || out_def.is_null() || out_value.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Control(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: output pointers are validated non-null above.
        unsafe {
            *done = 1;
            *name_len = 0;
            *out_def = DvcControlDefRaw::default();
            *out_value = 0.0;
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    // SAFETY: output pointers are validated non-null above.
    unsafe {
        *out_def = entry.def;
        *out_value = entry.value;
        *done = 0;
    }
    let status = write_utf8(&entry.name, name_buf, name_buf_len, name_len);
    if status != DVC_OK {
        return status;
    }
    if should_advance_string_entry(name_buf, name_buf_len, &entry.name) {
        state.index += 1;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_control_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
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
pub unsafe extern "C" fn dvc_chart_define(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    def: *const DvcChartDefRaw,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if def.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_chart_define: def is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_chart_define name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    // SAFETY: `def` is validated non-null above.
    let converted = match chart_def_from_raw(handle, unsafe { *def }) {
        Ok(def) => def,
        Err(status) => return status,
    };
    match engine_call!(handle, handle.engine.define_chart(&name, converted)) {
        Ok(()) => DVC_OK,
        Err(status) => status,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_remove(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_chart_remove: found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_chart_remove name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let removed = handle.engine.remove_chart(&name);
    // SAFETY: `found` is validated non-null above.
    unsafe {
        *found = if removed { 1 } else { 0 };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterate(
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
            "dvc_chart_iterate: out_iter is null",
        );
    }
    let entries = handle
        .engine
        .all_charts()
        .into_iter()
        .map(|(name, def)| ChartIteratorEntry {
            name,
            def: chart_def_to_raw(def),
        })
        .collect();
    let iterator = IteratorHandle::Chart(ChartIteratorState { entries, index: 0 });
    // SAFETY: `out_iter` is validated non-null above.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterator_next(
    iterator: DvcIteratorHandle,
    name_buf: *mut u8,
    name_buf_len: u32,
    name_len: *mut u32,
    out_def: *mut DvcChartDefRaw,
    done: *mut i32,
) -> i32 {
    if name_len.is_null() || out_def.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Chart(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: output pointers are validated non-null above.
        unsafe {
            *done = 1;
            *name_len = 0;
            *out_def = DvcChartDefRaw::default();
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    // SAFETY: output pointers are validated non-null above.
    unsafe {
        *out_def = entry.def;
        *done = 0;
    }
    let status = write_utf8(&entry.name, name_buf, name_buf_len, name_len);
    if status != DVC_OK {
        return status;
    }
    if should_advance_string_entry(name_buf, name_buf_len, &entry.name) {
        state.index += 1;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
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
pub unsafe extern "C" fn dvc_chart_get_output(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    out_output: *mut DvcChartOutputHandle,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_output.is_null() || found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_chart_get_output: out_output/found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_chart_get_output name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    // SAFETY: pointers are validated non-null above.
    unsafe {
        if let Some(output) = handle.engine.chart_output(&name).cloned() {
            *found = 1;
            *out_output = handle.register_chart_output_handle(output);
        } else {
            *found = 0;
            *out_output = std::ptr::null_mut();
        }
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_output_series_count(
    output: DvcChartOutputHandle,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { chart_output_handle(output) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let count = match u32::try_from(handle.output.series.len()) {
        Ok(v) => v,
        Err(_) => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: `out_count` is validated non-null above.
    unsafe {
        *out_count = count;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_output_label_count(
    output: DvcChartOutputHandle,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { chart_output_handle(output) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let count = match u32::try_from(handle.output.labels.len()) {
        Ok(v) => v,
        Err(_) => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: `out_count` is validated non-null above.
    unsafe {
        *out_count = count;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_output_label(
    output: DvcChartOutputHandle,
    index: u32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { chart_output_handle(output) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let Some(label) = handle.output.labels.get(index as usize) else {
        return DVC_ERR_OUT_OF_BOUNDS;
    };
    write_utf8(label, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_output_series_name(
    output: DvcChartOutputHandle,
    series_index: u32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { chart_output_handle(output) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let Some(series) = handle.output.series.get(series_index as usize) else {
        return DVC_ERR_OUT_OF_BOUNDS;
    };
    write_utf8(&series.name, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_chart_output_series_values(
    output: DvcChartOutputHandle,
    series_index: u32,
    buf: *mut f64,
    buf_len: u32,
    out_count: *mut u32,
) -> i32 {
    if out_count.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { chart_output_handle(output) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let Some(series) = handle.output.series.get(series_index as usize) else {
        return DVC_ERR_OUT_OF_BOUNDS;
    };
    let total = match u32::try_from(series.values.len()) {
        Ok(v) => v,
        Err(_) => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: `out_count` is validated non-null above.
    unsafe {
        *out_count = total;
    }
    if !buf.is_null() && buf_len > 0 {
        let count = std::cmp::min(buf_len as usize, series.values.len());
        // SAFETY: caller provides writable f64 buffer of at least `count` elements.
        unsafe {
            ptr::copy_nonoverlapping(series.values.as_ptr(), buf, count);
        }
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_udf_register(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    callback: Option<DvcUdfCallback>,
    user_data: *mut c_void,
    volatility: i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let Some(callback) = callback else {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_udf_register: callback is null",
        );
    };
    let name = match read_text_arg(handle, name, name_len, "dvc_udf_register name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let volatility = match volatility_from_raw(volatility) {
        Ok(volatility) => volatility,
        Err(status) => {
            return fail(
                handle,
                status,
                "dvc_udf_register: volatility must be standard/volatile/externally-invalidated",
            );
        }
    };
    let udf = CApiUdf {
        callback,
        user_data,
        volatility,
    };
    handle.engine.register_udf(&name, Box::new(udf));
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_udf_unregister(
    engine: DvcEngineHandle,
    name: *const u8,
    name_len: u32,
    found: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if found.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_udf_unregister: found is null",
        );
    }
    let name = match read_text_arg(handle, name, name_len, "dvc_udf_unregister name") {
        Ok(name) => name,
        Err(status) => return status,
    };
    let removed = handle.engine.unregister_udf(&name);
    // SAFETY: `found` is validated non-null above.
    unsafe {
        *found = if removed { 1 } else { 0 };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_tracking_enable(engine: DvcEngineHandle) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    handle.engine.enable_change_tracking();
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_tracking_disable(engine: DvcEngineHandle) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    handle.engine.disable_change_tracking();
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_tracking_is_enabled(
    engine: DvcEngineHandle,
    out_enabled: *mut i32,
) -> i32 {
    let handle = match unsafe { engine_handle_mut(engine) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    if out_enabled.is_null() {
        return fail(
            handle,
            DVC_ERR_NULL_POINTER,
            "dvc_change_tracking_is_enabled: out_enabled is null",
        );
    }
    // SAFETY: `out_enabled` is validated non-null above.
    unsafe {
        *out_enabled = if handle.engine.is_change_tracking_enabled() {
            1
        } else {
            0
        };
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_iterate(
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
            "dvc_change_iterate: out_iter is null",
        );
    }
    let iterator = IteratorHandle::Change(ChangeIteratorState {
        entries: handle.engine.drain_changes(),
        index: 0,
        current: None,
    });
    // SAFETY: `out_iter` is validated non-null.
    unsafe {
        *out_iter = Box::into_raw(Box::new(iterator)).cast::<c_void>();
    }
    handle.clear_error();
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_iterator_next(
    iterator: DvcIteratorHandle,
    out_change_type: *mut i32,
    out_epoch: *mut u64,
    done: *mut i32,
) -> i32 {
    if out_change_type.is_null() || out_epoch.is_null() || done.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    if state.index >= state.entries.len() {
        // SAFETY: pointers are validated non-null above.
        unsafe {
            *done = 1;
            *out_change_type = DVC_CHANGE_CELL_VALUE;
            *out_epoch = 0;
        }
        return DVC_OK;
    }
    let entry = state.entries[state.index].clone();
    state.index += 1;
    let (change_type, epoch) = change_entry_type_and_epoch(&entry);
    state.current = Some(entry);
    // SAFETY: pointers are validated non-null above.
    unsafe {
        *done = 0;
        *out_change_type = change_type;
        *out_epoch = epoch;
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_cell(
    iterator: DvcIteratorHandle,
    out_addr: *mut DvcCellAddr,
) -> i32 {
    if out_addr.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let cell = match current {
        ChangeEntry::CellValue { cell, .. } => *cell,
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: out pointer is validated non-null above.
    unsafe {
        *out_addr = cell_to_addr(cell);
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_name(
    iterator: DvcIteratorHandle,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let name = match current {
        ChangeEntry::NameValue { name, .. } => name,
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    write_utf8(name, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_chart_name(
    iterator: DvcIteratorHandle,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let name = match current {
        ChangeEntry::ChartOutput { name, .. } => name,
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    write_utf8(name, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_spill(
    iterator: DvcIteratorHandle,
    out_anchor: *mut DvcCellAddr,
    out_old_range: *mut DvcCellRange,
    out_had_old: *mut i32,
    out_new_range: *mut DvcCellRange,
    out_has_new: *mut i32,
) -> i32 {
    if out_anchor.is_null()
        || out_old_range.is_null()
        || out_had_old.is_null()
        || out_new_range.is_null()
        || out_has_new.is_null()
    {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let (anchor, old_range, new_range) = match current {
        ChangeEntry::SpillRegion {
            anchor,
            old_range,
            new_range,
            ..
        } => (*anchor, *old_range, *new_range),
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: pointers are validated non-null above.
    unsafe {
        *out_anchor = cell_to_addr(anchor);
        if let Some(range) = old_range {
            *out_had_old = 1;
            *out_old_range = range_to_raw(range);
        } else {
            *out_had_old = 0;
            *out_old_range = DvcCellRange::default();
        }
        if let Some(range) = new_range {
            *out_has_new = 1;
            *out_new_range = range_to_raw(range);
        } else {
            *out_has_new = 0;
            *out_new_range = DvcCellRange::default();
        }
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_format(
    iterator: DvcIteratorHandle,
    out_addr: *mut DvcCellAddr,
    out_old_format: *mut DvcCellFormatRaw,
    out_new_format: *mut DvcCellFormatRaw,
) -> i32 {
    if out_addr.is_null() || out_old_format.is_null() || out_new_format.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let (cell, old, new) = match current {
        ChangeEntry::CellFormat { cell, old, new, .. } => (*cell, old.clone(), new.clone()),
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    // SAFETY: pointers are validated non-null above.
    unsafe {
        *out_addr = cell_to_addr(cell);
        *out_old_format = format_to_raw(old);
        *out_new_format = format_to_raw(new);
    }
    DVC_OK
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_get_diagnostic(
    iterator: DvcIteratorHandle,
    out_code: *mut i32,
    buf: *mut u8,
    buf_len: u32,
    out_len: *mut u32,
) -> i32 {
    if out_code.is_null() {
        return DVC_ERR_NULL_POINTER;
    }
    let handle = match unsafe { iterator_handle_mut(iterator) } {
        Ok(handle) => handle,
        Err(status) => return status,
    };
    let IteratorHandle::Change(state) = handle else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let Some(current) = state.current.as_ref() else {
        return DVC_ERR_INVALID_ARGUMENT;
    };
    let (code, message) = match current {
        ChangeEntry::Diagnostic { code, message, .. } => (*code, message),
        _ => return DVC_ERR_INVALID_ARGUMENT,
    };
    let raw_code = match code {
        DiagnosticCode::CircularReferenceDetected => DVC_DIAG_CIRCULAR_REFERENCE_DETECTED,
    };
    // SAFETY: `out_code` is validated non-null above.
    unsafe {
        *out_code = raw_code;
    }
    write_utf8(message, buf, buf_len, out_len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dvc_change_iterator_destroy(iterator: DvcIteratorHandle) -> i32 {
    if iterator.is_null() {
        return DVC_OK;
    }
    // SAFETY: `iterator` is allocated by `Box::into_raw` in iterate APIs.
    unsafe {
        drop(Box::from_raw(iterator.cast::<IteratorHandle>()));
    }
    DVC_OK
}
