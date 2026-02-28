pub mod config {
    use std::fmt;
    use std::path::PathBuf;

    pub const COREENGINE_ENV: &str = "DNAVISICALC_COREENGINE";
    pub const COREENGINE_DLL_ENV: &str = "DNAVISICALC_COREENGINE_DLL";

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CoreEngineId {
        DotnetCore,
        RustCore,
    }

    impl CoreEngineId {
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::DotnetCore => "dotnet-core",
                Self::RustCore => "rust-core",
            }
        }

        pub fn parse(input: &str) -> Option<Self> {
            let normalized = input.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "dotnet" | "dotnet-core" | "net" | "coreengine-net" => Some(Self::DotnetCore),
                "rust" | "rust-core" | "core" | "coreengine-rust" => Some(Self::RustCore),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EngineConfig {
        pub coreengine: CoreEngineId,
        pub coreengine_dll: Option<PathBuf>,
    }

    impl Default for EngineConfig {
        fn default() -> Self {
            Self {
                coreengine: CoreEngineId::DotnetCore,
                coreengine_dll: None,
            }
        }
    }

    impl EngineConfig {
        pub fn from_env_lossy() -> Self {
            let coreengine = std::env::var_os(COREENGINE_ENV)
                .and_then(|raw| CoreEngineId::parse(&raw.to_string_lossy()))
                .unwrap_or(CoreEngineId::DotnetCore);
            let coreengine_dll = std::env::var_os(COREENGINE_DLL_ENV).map(PathBuf::from);
            Self {
                coreengine,
                coreengine_dll,
            }
        }

        pub fn from_env_strict() -> Result<Self, EngineConfigError> {
            let coreengine = match std::env::var_os(COREENGINE_ENV) {
                Some(raw) => {
                    let raw_text = raw.to_string_lossy().to_string();
                    CoreEngineId::parse(&raw_text)
                        .ok_or(EngineConfigError::UnknownCoreEngine(raw_text))?
                }
                None => CoreEngineId::DotnetCore,
            };
            let coreengine_dll = std::env::var_os(COREENGINE_DLL_ENV).map(PathBuf::from);
            Ok(Self {
                coreengine,
                coreengine_dll,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EngineConfigError {
        UnknownCoreEngine(String),
    }

    impl fmt::Display for EngineConfigError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::UnknownCoreEngine(value) => write!(
                    f,
                    "unknown coreengine '{value}' (supported: dotnet-core, rust-core)"
                ),
            }
        }
    }

    impl std::error::Error for EngineConfigError {}
}

use std::collections::HashSet;
use std::ffi::c_void;
use std::fmt;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

pub use config::{
    COREENGINE_DLL_ENV, COREENGINE_ENV, CoreEngineId, EngineConfig, EngineConfigError,
};
pub use dnavisicalc_core::{
    AddressError, BinaryOp, CalcNode, CalcTree, CellError, CellFormat, CellInput, CellRange,
    CellRef, CellState, ChangeEntry, ChartDefinition, ChartOutput, ChartSeriesOutput,
    ControlDefinition, ControlKind, DEFAULT_SHEET_BOUNDS, DependencyError, DynamicArrayStrategy,
    Expr, FnUdf, FnUdfWithVolatility, IterationConfig, MAX_COLUMNS, MAX_ROWS, NameInput,
    PaletteColor, ParseError, RecalcMode, RefFlags, SUPPORTED_FUNCTIONS, Scc, SheetBounds,
    StructuralOp, UdfHandler, UnaryOp, Value, Volatility, build_calc_tree,
    build_calc_tree_allow_cycles, col_index_to_label, col_label_to_index, expr_to_formula,
    parse_formula, rewrite_expr,
};

const DVC_OK: i32 = 0;
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
const DVC_CONTROL_CHECKBOX: i32 = 1;
const DVC_CONTROL_BUTTON: i32 = 2;

#[derive(Debug, Clone)]
pub enum EngineError {
    Address(AddressError),
    Api {
        status: i32,
        op: String,
        message: String,
    },
    InvalidUtf8Length,
    InvalidCellRef(String),
    InvalidNumber(String),
    OutOfBounds(CellRef),
    Config(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(err) => write!(f, "{err}"),
            Self::Api {
                status,
                op,
                message,
            } => {
                if message.is_empty() {
                    write!(f, "{op} failed with status {status}")
                } else {
                    write!(f, "{op} failed with status {status}: {message}")
                }
            }
            Self::InvalidUtf8Length => write!(f, "string exceeds C API length limits"),
            Self::InvalidCellRef(a1) => write!(f, "invalid cell reference '{a1}'"),
            Self::InvalidNumber(text) => write!(f, "invalid numeric value '{text}'"),
            Self::OutOfBounds(cell) => write!(f, "cell {cell} is out of engine bounds"),
            Self::Config(msg) => write!(f, "engine config error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<AddressError> for EngineError {
    fn from(value: AddressError) -> Self {
        Self::Address(value)
    }
}

type DvcEngineHandle = *mut c_void;
type DvcIteratorHandle = *mut c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcCellAddr {
    col: u16,
    row: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcCellRange {
    start: DvcCellAddr,
    end: DvcCellAddr,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcSheetBounds {
    max_columns: u16,
    max_rows: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcCellValue {
    value_type: i32,
    number: f64,
    bool_val: i32,
    error_kind: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcCellState {
    value: DvcCellValue,
    value_epoch: u64,
    stale: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcCellFormatRaw {
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
struct DvcIterationConfigRaw {
    enabled: i32,
    max_iterations: u32,
    convergence_tolerance: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcControlDefRaw {
    kind: i32,
    min: f64,
    max: f64,
    step: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct DvcChartDefRaw {
    source_range: DvcCellRange,
}

type FnApiVersion = unsafe extern "C" fn() -> u32;
type FnEngineCreate = unsafe extern "C" fn(*mut DvcEngineHandle) -> i32;
type FnEngineCreateWithBounds = unsafe extern "C" fn(DvcSheetBounds, *mut DvcEngineHandle) -> i32;
type FnEngineDestroy = unsafe extern "C" fn(DvcEngineHandle) -> i32;
type FnEngineClear = unsafe extern "C" fn(DvcEngineHandle) -> i32;
type FnEngineBounds = unsafe extern "C" fn(DvcEngineHandle, *mut DvcSheetBounds) -> i32;
type FnEngineGetRecalcMode = unsafe extern "C" fn(DvcEngineHandle, *mut i32) -> i32;
type FnEngineSetRecalcMode = unsafe extern "C" fn(DvcEngineHandle, i32) -> i32;
type FnEngineCommittedEpoch = unsafe extern "C" fn(DvcEngineHandle, *mut u64) -> i32;
type FnEngineStabilizedEpoch = unsafe extern "C" fn(DvcEngineHandle, *mut u64) -> i32;
type FnRecalculate = unsafe extern "C" fn(DvcEngineHandle) -> i32;
type FnHasStreamCells = unsafe extern "C" fn(DvcEngineHandle, *mut i32) -> i32;
type FnTickStreams = unsafe extern "C" fn(DvcEngineHandle, f64, *mut i32) -> i32;
type FnEngineGetIterationConfig =
    unsafe extern "C" fn(DvcEngineHandle, *mut DvcIterationConfigRaw) -> i32;
type FnEngineSetIterationConfig =
    unsafe extern "C" fn(DvcEngineHandle, *const DvcIterationConfigRaw) -> i32;
type FnLastErrorMessage = unsafe extern "C" fn(DvcEngineHandle, *mut u8, u32, *mut u32) -> i32;

type FnCellSetNumber = unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, f64) -> i32;
type FnCellSetText = unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *const u8, u32) -> i32;
type FnCellSetFormula =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *const u8, u32) -> i32;
type FnCellClear = unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr) -> i32;
type FnCellGetState = unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut DvcCellState) -> i32;
type FnCellGetText =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut u8, u32, *mut u32) -> i32;
type FnCellGetInputType = unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut i32) -> i32;
type FnCellGetInputText =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut u8, u32, *mut u32) -> i32;
type FnCellErrorMessage =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut u8, u32, *mut u32) -> i32;

type FnNameSetNumber = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, f64) -> i32;
type FnNameSetText =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *const u8, u32) -> i32;
type FnNameSetFormula =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *const u8, u32) -> i32;
type FnNameClear = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32) -> i32;
type FnNameGetInputType = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut i32) -> i32;
type FnNameGetInputText =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut u8, u32, *mut u32) -> i32;

type FnCellGetFormat =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut DvcCellFormatRaw) -> i32;
type FnCellSetFormat =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *const DvcCellFormatRaw) -> i32;

type FnCellSpillAnchor =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut DvcCellAddr, *mut i32) -> i32;
type FnCellSpillRange =
    unsafe extern "C" fn(DvcEngineHandle, DvcCellAddr, *mut DvcCellRange, *mut i32) -> i32;

type FnInsertRow = unsafe extern "C" fn(DvcEngineHandle, u16) -> i32;
type FnDeleteRow = unsafe extern "C" fn(DvcEngineHandle, u16) -> i32;
type FnInsertCol = unsafe extern "C" fn(DvcEngineHandle, u16) -> i32;
type FnDeleteCol = unsafe extern "C" fn(DvcEngineHandle, u16) -> i32;

type FnParseCellRef = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut DvcCellAddr) -> i32;

type FnCellIterate = unsafe extern "C" fn(DvcEngineHandle, *mut DvcIteratorHandle) -> i32;
type FnCellIteratorNext =
    unsafe extern "C" fn(DvcIteratorHandle, *mut DvcCellAddr, *mut i32, *mut i32) -> i32;
type FnCellIteratorGetText = unsafe extern "C" fn(DvcIteratorHandle, *mut u8, u32, *mut u32) -> i32;
type FnCellIteratorDestroy = unsafe extern "C" fn(DvcIteratorHandle) -> i32;

type FnNameIterate = unsafe extern "C" fn(DvcEngineHandle, *mut DvcIteratorHandle) -> i32;
type FnNameIteratorNext = unsafe extern "C" fn(
    DvcIteratorHandle,
    *mut u8,
    u32,
    *mut u32,
    *mut i32,
    *mut i32,
) -> i32;
type FnNameIteratorGetText = unsafe extern "C" fn(DvcIteratorHandle, *mut u8, u32, *mut u32) -> i32;
type FnNameIteratorDestroy = unsafe extern "C" fn(DvcIteratorHandle) -> i32;

type FnFormatIterate = unsafe extern "C" fn(DvcEngineHandle, *mut DvcIteratorHandle) -> i32;
type FnFormatIteratorNext = unsafe extern "C" fn(
    DvcIteratorHandle,
    *mut DvcCellAddr,
    *mut DvcCellFormatRaw,
    *mut i32,
) -> i32;
type FnFormatIteratorDestroy = unsafe extern "C" fn(DvcIteratorHandle) -> i32;

type FnControlDefine =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *const DvcControlDefRaw) -> i32;
type FnControlRemove = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut i32) -> i32;
type FnControlSetValue = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, f64) -> i32;
type FnControlGetValue =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut f64, *mut i32) -> i32;
type FnControlGetDef = unsafe extern "C" fn(
    DvcEngineHandle,
    *const u8,
    u32,
    *mut DvcControlDefRaw,
    *mut i32,
) -> i32;
type FnControlIterate = unsafe extern "C" fn(DvcEngineHandle, *mut DvcIteratorHandle) -> i32;
type FnControlIteratorNext = unsafe extern "C" fn(
    DvcIteratorHandle,
    *mut u8,
    u32,
    *mut u32,
    *mut DvcControlDefRaw,
    *mut f64,
    *mut i32,
) -> i32;
type FnControlIteratorDestroy = unsafe extern "C" fn(DvcIteratorHandle) -> i32;

type FnChartDefine =
    unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *const DvcChartDefRaw) -> i32;
type FnChartRemove = unsafe extern "C" fn(DvcEngineHandle, *const u8, u32, *mut i32) -> i32;
type FnChartIterate = unsafe extern "C" fn(DvcEngineHandle, *mut DvcIteratorHandle) -> i32;
type FnChartIteratorNext = unsafe extern "C" fn(
    DvcIteratorHandle,
    *mut u8,
    u32,
    *mut u32,
    *mut DvcChartDefRaw,
    *mut i32,
) -> i32;
type FnChartIteratorDestroy = unsafe extern "C" fn(DvcIteratorHandle) -> i32;

#[derive(Clone, Copy)]
struct ApiFns {
    api_version: FnApiVersion,
    engine_create: FnEngineCreate,
    engine_create_with_bounds: FnEngineCreateWithBounds,
    engine_destroy: FnEngineDestroy,
    engine_clear: FnEngineClear,
    engine_bounds: FnEngineBounds,
    engine_get_recalc_mode: FnEngineGetRecalcMode,
    engine_set_recalc_mode: FnEngineSetRecalcMode,
    engine_committed_epoch: FnEngineCommittedEpoch,
    engine_stabilized_epoch: FnEngineStabilizedEpoch,
    recalculate: FnRecalculate,
    has_stream_cells: FnHasStreamCells,
    tick_streams: FnTickStreams,
    engine_get_iteration_config: FnEngineGetIterationConfig,
    engine_set_iteration_config: FnEngineSetIterationConfig,
    last_error_message: FnLastErrorMessage,

    cell_set_number: FnCellSetNumber,
    cell_set_text: FnCellSetText,
    cell_set_formula: FnCellSetFormula,
    cell_clear: FnCellClear,
    cell_get_state: FnCellGetState,
    cell_get_text: FnCellGetText,
    cell_get_input_type: FnCellGetInputType,
    cell_get_input_text: FnCellGetInputText,
    cell_error_message: FnCellErrorMessage,

    name_set_number: FnNameSetNumber,
    name_set_text: FnNameSetText,
    name_set_formula: FnNameSetFormula,
    name_clear: FnNameClear,
    name_get_input_type: FnNameGetInputType,
    name_get_input_text: FnNameGetInputText,

    cell_get_format: FnCellGetFormat,
    cell_set_format: FnCellSetFormat,

    cell_spill_anchor: FnCellSpillAnchor,
    cell_spill_range: FnCellSpillRange,

    insert_row: FnInsertRow,
    delete_row: FnDeleteRow,
    insert_col: FnInsertCol,
    delete_col: FnDeleteCol,

    parse_cell_ref: FnParseCellRef,

    cell_iterate: FnCellIterate,
    cell_iterator_next: FnCellIteratorNext,
    cell_iterator_get_text: FnCellIteratorGetText,
    cell_iterator_destroy: FnCellIteratorDestroy,

    name_iterate: FnNameIterate,
    name_iterator_next: FnNameIteratorNext,
    name_iterator_get_text: FnNameIteratorGetText,
    name_iterator_destroy: FnNameIteratorDestroy,

    format_iterate: FnFormatIterate,
    format_iterator_next: FnFormatIteratorNext,
    format_iterator_destroy: FnFormatIteratorDestroy,

    control_define: FnControlDefine,
    control_remove: FnControlRemove,
    control_set_value: FnControlSetValue,
    control_get_value: FnControlGetValue,
    control_get_def: FnControlGetDef,
    control_iterate: FnControlIterate,
    control_iterator_next: FnControlIteratorNext,
    control_iterator_destroy: FnControlIteratorDestroy,

    chart_define: FnChartDefine,
    chart_remove: FnChartRemove,
    chart_iterate: FnChartIterate,
    chart_iterator_next: FnChartIteratorNext,
    chart_iterator_destroy: FnChartIteratorDestroy,
}

struct LoadedApi {
    _library: Library,
    fns: ApiFns,
    loaded_from: PathBuf,
}

fn load_symbol<T: Copy>(lib: &Library, name: &[u8]) -> Result<T, EngineError> {
    let sym: Symbol<'_, T> =
        unsafe { lib.get(name) }.map_err(|err| EngineError::Config(err.to_string()))?;
    Ok(*sym)
}

fn load_fns(lib: &Library) -> Result<ApiFns, EngineError> {
    Ok(ApiFns {
        api_version: load_symbol(lib, b"dvc_api_version\0")?,
        engine_create: load_symbol(lib, b"dvc_engine_create\0")?,
        engine_create_with_bounds: load_symbol(lib, b"dvc_engine_create_with_bounds\0")?,
        engine_destroy: load_symbol(lib, b"dvc_engine_destroy\0")?,
        engine_clear: load_symbol(lib, b"dvc_engine_clear\0")?,
        engine_bounds: load_symbol(lib, b"dvc_engine_bounds\0")?,
        engine_get_recalc_mode: load_symbol(lib, b"dvc_engine_get_recalc_mode\0")?,
        engine_set_recalc_mode: load_symbol(lib, b"dvc_engine_set_recalc_mode\0")?,
        engine_committed_epoch: load_symbol(lib, b"dvc_engine_committed_epoch\0")?,
        engine_stabilized_epoch: load_symbol(lib, b"dvc_engine_stabilized_epoch\0")?,
        recalculate: load_symbol(lib, b"dvc_recalculate\0")?,
        has_stream_cells: load_symbol(lib, b"dvc_has_stream_cells\0")?,
        tick_streams: load_symbol(lib, b"dvc_tick_streams\0")?,
        engine_get_iteration_config: load_symbol(lib, b"dvc_engine_get_iteration_config\0")?,
        engine_set_iteration_config: load_symbol(lib, b"dvc_engine_set_iteration_config\0")?,
        last_error_message: load_symbol(lib, b"dvc_last_error_message\0")?,

        cell_set_number: load_symbol(lib, b"dvc_cell_set_number\0")?,
        cell_set_text: load_symbol(lib, b"dvc_cell_set_text\0")?,
        cell_set_formula: load_symbol(lib, b"dvc_cell_set_formula\0")?,
        cell_clear: load_symbol(lib, b"dvc_cell_clear\0")?,
        cell_get_state: load_symbol(lib, b"dvc_cell_get_state\0")?,
        cell_get_text: load_symbol(lib, b"dvc_cell_get_text\0")?,
        cell_get_input_type: load_symbol(lib, b"dvc_cell_get_input_type\0")?,
        cell_get_input_text: load_symbol(lib, b"dvc_cell_get_input_text\0")?,
        cell_error_message: load_symbol(lib, b"dvc_cell_error_message\0")?,

        name_set_number: load_symbol(lib, b"dvc_name_set_number\0")?,
        name_set_text: load_symbol(lib, b"dvc_name_set_text\0")?,
        name_set_formula: load_symbol(lib, b"dvc_name_set_formula\0")?,
        name_clear: load_symbol(lib, b"dvc_name_clear\0")?,
        name_get_input_type: load_symbol(lib, b"dvc_name_get_input_type\0")?,
        name_get_input_text: load_symbol(lib, b"dvc_name_get_input_text\0")?,

        cell_get_format: load_symbol(lib, b"dvc_cell_get_format\0")?,
        cell_set_format: load_symbol(lib, b"dvc_cell_set_format\0")?,

        cell_spill_anchor: load_symbol(lib, b"dvc_cell_spill_anchor\0")?,
        cell_spill_range: load_symbol(lib, b"dvc_cell_spill_range\0")?,

        insert_row: load_symbol(lib, b"dvc_insert_row\0")?,
        delete_row: load_symbol(lib, b"dvc_delete_row\0")?,
        insert_col: load_symbol(lib, b"dvc_insert_col\0")?,
        delete_col: load_symbol(lib, b"dvc_delete_col\0")?,

        parse_cell_ref: load_symbol(lib, b"dvc_parse_cell_ref\0")?,

        cell_iterate: load_symbol(lib, b"dvc_cell_iterate\0")?,
        cell_iterator_next: load_symbol(lib, b"dvc_cell_iterator_next\0")?,
        cell_iterator_get_text: load_symbol(lib, b"dvc_cell_iterator_get_text\0")?,
        cell_iterator_destroy: load_symbol(lib, b"dvc_cell_iterator_destroy\0")?,

        name_iterate: load_symbol(lib, b"dvc_name_iterate\0")?,
        name_iterator_next: load_symbol(lib, b"dvc_name_iterator_next\0")?,
        name_iterator_get_text: load_symbol(lib, b"dvc_name_iterator_get_text\0")?,
        name_iterator_destroy: load_symbol(lib, b"dvc_name_iterator_destroy\0")?,

        format_iterate: load_symbol(lib, b"dvc_format_iterate\0")?,
        format_iterator_next: load_symbol(lib, b"dvc_format_iterator_next\0")?,
        format_iterator_destroy: load_symbol(lib, b"dvc_format_iterator_destroy\0")?,

        control_define: load_symbol(lib, b"dvc_control_define\0")?,
        control_remove: load_symbol(lib, b"dvc_control_remove\0")?,
        control_set_value: load_symbol(lib, b"dvc_control_set_value\0")?,
        control_get_value: load_symbol(lib, b"dvc_control_get_value\0")?,
        control_get_def: load_symbol(lib, b"dvc_control_get_def\0")?,
        control_iterate: load_symbol(lib, b"dvc_control_iterate\0")?,
        control_iterator_next: load_symbol(lib, b"dvc_control_iterator_next\0")?,
        control_iterator_destroy: load_symbol(lib, b"dvc_control_iterator_destroy\0")?,

        chart_define: load_symbol(lib, b"dvc_chart_define\0")?,
        chart_remove: load_symbol(lib, b"dvc_chart_remove\0")?,
        chart_iterate: load_symbol(lib, b"dvc_chart_iterate\0")?,
        chart_iterator_next: load_symbol(lib, b"dvc_chart_iterator_next\0")?,
        chart_iterator_destroy: load_symbol(lib, b"dvc_chart_iterator_destroy\0")?,
    })
}

fn candidate_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        roots.push(dir.to_path_buf());
    }

    // `CARGO_MANIFEST_DIR` is `.../crates/dnavisicalc-engine`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    roots.push(manifest_dir.clone());
    if let Some(parent) = manifest_dir.parent() {
        roots.push(parent.to_path_buf()); // `<repo>/crates`
        if let Some(repo_root) = parent.parent() {
            roots.push(repo_root.to_path_buf()); // `<repo>`
        }
    }

    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn default_candidates(coreengine: CoreEngineId) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let roots = candidate_roots();
    let add_name = |candidates: &mut Vec<PathBuf>, name: &str| {
        candidates.push(PathBuf::from(name));
        for root in &roots {
            candidates.push(root.join(name));
        }
    };
    let add_relative = |candidates: &mut Vec<PathBuf>, relative: &str| {
        candidates.push(PathBuf::from(relative));
        for root in &roots {
            candidates.push(root.join(relative));
        }
    };

    add_name(&mut candidates, "dnavisicalc_coreengine.dll");

    match coreengine {
        CoreEngineId::DotnetCore => {
            add_name(&mut candidates, "Dvc.Native.dll");
            add_relative(
                &mut candidates,
                "engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll",
            );
            add_relative(
                &mut candidates,
                "engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/Debug/net10.0/win-x64/publish/Dvc.Native.dll",
            );
            add_relative(
                &mut candidates,
                "engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/x64/Release/net10.0/win-x64/publish/Dvc.Native.dll",
            );
        }
        CoreEngineId::RustCore => {
            add_name(&mut candidates, "dnavisicalc_coreengine_rust.dll");
            add_name(&mut candidates, "dnavisicalc_coreengine.dll");
            add_relative(
                &mut candidates,
                "target/debug/dnavisicalc_coreengine_rust.dll",
            );
            add_relative(
                &mut candidates,
                "target/release/dnavisicalc_coreengine_rust.dll",
            );
        }
    }

    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn load_api(config: &EngineConfig) -> Result<LoadedApi, EngineError> {
    let candidates = if let Some(path) = &config.coreengine_dll {
        vec![path.clone()]
    } else {
        default_candidates(config.coreengine)
    };

    let mut errors: Vec<String> = Vec::new();
    for path in candidates {
        if !path.exists() {
            continue;
        }
        let library = unsafe { Library::new(&path) };
        let library = match library {
            Ok(lib) => lib,
            Err(err) => {
                errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        let fns = match load_fns(&library) {
            Ok(fns) => fns,
            Err(err) => {
                errors.push(format!("{}: {err}", path.display()));
                continue;
            }
        };
        return Ok(LoadedApi {
            _library: library,
            fns,
            loaded_from: path,
        });
    }

    Err(EngineError::Config(format!(
        "failed to load core engine DLL for {:?}: {}",
        config.coreengine,
        if errors.is_empty() {
            "no candidate found".to_string()
        } else {
            errors.join(" | ")
        }
    )))
}

pub struct Engine {
    config: EngineConfig,
    api: LoadedApi,
    handle: DvcEngineHandle,
    dynamic_array_strategy: DynamicArrayStrategy,
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("config", &self.config)
            .field("loaded_from", &self.api.loaded_from)
            .field("dynamic_array_strategy", &self.dynamic_array_strategy)
            .finish_non_exhaustive()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = unsafe { (self.api.fns.engine_destroy)(self.handle) };
            self.handle = std::ptr::null_mut();
        }
    }
}

impl Engine {
    pub fn try_new() -> Result<Self, EngineError> {
        let config = EngineConfig::from_env_lossy();
        Self::try_new_with_config(config)
    }

    pub fn new() -> Self {
        Self::try_new()
            .unwrap_or_else(|err| panic!("failed to initialize engine via C API DLL: {err}"))
    }

    pub fn try_with_bounds(bounds: SheetBounds) -> Result<Self, EngineError> {
        let config = EngineConfig::from_env_lossy();
        Self::try_with_bounds_and_config(bounds, config)
    }

    pub fn with_bounds(bounds: SheetBounds) -> Self {
        Self::try_with_bounds(bounds)
            .unwrap_or_else(|err| panic!("failed to initialize engine via C API DLL: {err}"))
    }

    pub fn try_new_with_config(config: EngineConfig) -> Result<Self, EngineError> {
        let api = load_api(&config)?;
        let mut handle: DvcEngineHandle = std::ptr::null_mut();
        let status = unsafe { (api.fns.engine_create)(&mut handle) };
        if status != DVC_OK {
            return Err(EngineError::Api {
                status,
                op: "dvc_engine_create".to_string(),
                message: String::new(),
            });
        }
        Ok(Self {
            config,
            api,
            handle,
            dynamic_array_strategy: DynamicArrayStrategy::OverlayInline,
        })
    }

    pub fn new_with_config(config: EngineConfig) -> Self {
        Self::try_new_with_config(config)
            .unwrap_or_else(|err| panic!("failed to initialize engine via C API DLL: {err}"))
    }

    pub fn try_with_bounds_and_config(
        bounds: SheetBounds,
        config: EngineConfig,
    ) -> Result<Self, EngineError> {
        let api = load_api(&config)?;
        let mut handle: DvcEngineHandle = std::ptr::null_mut();
        let raw = sheet_bounds_to_raw(bounds);
        let status = unsafe { (api.fns.engine_create_with_bounds)(raw, &mut handle) };
        if status != DVC_OK {
            return Err(EngineError::Api {
                status,
                op: "dvc_engine_create_with_bounds".to_string(),
                message: String::new(),
            });
        }
        Ok(Self {
            config,
            api,
            handle,
            dynamic_array_strategy: DynamicArrayStrategy::OverlayInline,
        })
    }

    pub fn with_bounds_and_config(bounds: SheetBounds, config: EngineConfig) -> Self {
        Self::try_with_bounds_and_config(bounds, config)
            .unwrap_or_else(|err| panic!("failed to initialize engine via C API DLL: {err}"))
    }

    pub fn coreengine(&self) -> CoreEngineId {
        self.config.coreengine
    }

    pub fn engine_config(&self) -> EngineConfig {
        self.config.clone()
    }

    pub fn loaded_library_path(&self) -> &Path {
        &self.api.loaded_from
    }

    pub fn api_version(&self) -> u32 {
        unsafe { (self.api.fns.api_version)() }
    }

    pub fn clear(&mut self) {
        let status = unsafe { (self.api.fns.engine_clear)(self.handle) };
        if status != DVC_OK {
            panic!(
                "dvc_engine_clear failed: {}",
                self.error_message(status, "dvc_engine_clear")
            );
        }
    }

    pub fn bounds(&self) -> SheetBounds {
        let mut out = DvcSheetBounds::default();
        let status = unsafe { (self.api.fns.engine_bounds)(self.handle, &mut out) };
        if status != DVC_OK {
            panic!(
                "dvc_engine_bounds failed: {}",
                self.error_message(status, "dvc_engine_bounds")
            );
        }
        sheet_bounds_from_raw(out)
    }

    pub fn recalc_mode(&self) -> RecalcMode {
        let mut mode = DVC_RECALC_AUTOMATIC;
        let status = unsafe { (self.api.fns.engine_get_recalc_mode)(self.handle, &mut mode) };
        if status != DVC_OK {
            return RecalcMode::Automatic;
        }
        recalc_mode_from_raw(mode)
    }

    pub fn set_recalc_mode(&mut self, mode: RecalcMode) {
        let status =
            unsafe { (self.api.fns.engine_set_recalc_mode)(self.handle, recalc_mode_to_raw(mode)) };
        if status != DVC_OK {
            panic!(
                "dvc_engine_set_recalc_mode failed: {}",
                self.error_message(status, "dvc_engine_set_recalc_mode")
            );
        }
    }

    pub fn dynamic_array_strategy(&self) -> DynamicArrayStrategy {
        self.dynamic_array_strategy
    }

    pub fn set_dynamic_array_strategy(&mut self, strategy: DynamicArrayStrategy) {
        self.dynamic_array_strategy = strategy;
    }

    pub fn iteration_config(&self) -> IterationConfig {
        let mut raw = DvcIterationConfigRaw::default();
        let status = unsafe { (self.api.fns.engine_get_iteration_config)(self.handle, &mut raw) };
        if status != DVC_OK {
            return IterationConfig::default();
        }
        iteration_config_from_raw(raw)
    }

    pub fn set_iteration_config(&mut self, config: IterationConfig) {
        let raw = iteration_config_to_raw(config);
        let status = unsafe { (self.api.fns.engine_set_iteration_config)(self.handle, &raw) };
        if status != DVC_OK {
            panic!(
                "dvc_engine_set_iteration_config failed: {}",
                self.error_message(status, "dvc_engine_set_iteration_config")
            );
        }
    }

    pub fn committed_epoch(&self) -> u64 {
        let mut epoch = 0u64;
        let status = unsafe { (self.api.fns.engine_committed_epoch)(self.handle, &mut epoch) };
        if status != DVC_OK {
            return 0;
        }
        epoch
    }

    pub fn stabilized_epoch(&self) -> u64 {
        let mut epoch = 0u64;
        let status = unsafe { (self.api.fns.engine_stabilized_epoch)(self.handle, &mut epoch) };
        if status != DVC_OK {
            return 0;
        }
        epoch
    }

    pub fn recalculate(&mut self) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.recalculate)(self.handle) };
        self.status_result(status, "dvc_recalculate")
    }

    pub fn has_stream_cells(&self) -> bool {
        let mut out = 0i32;
        let status = unsafe { (self.api.fns.has_stream_cells)(self.handle, &mut out) };
        status == DVC_OK && out != 0
    }

    pub fn tick_streams(&mut self, elapsed_secs: f64) -> bool {
        let mut advanced = 0i32;
        let status = unsafe { (self.api.fns.tick_streams)(self.handle, elapsed_secs, &mut advanced) };
        status == DVC_OK && advanced != 0
    }
}

impl Engine {
    pub fn set_number(&mut self, cell: CellRef, number: f64) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.cell_set_number)(self.handle, cell_to_addr(cell), number) };
        self.status_result(status, "dvc_cell_set_number")
    }

    pub fn set_text(&mut self, cell: CellRef, text: impl Into<String>) -> Result<(), EngineError> {
        let text = text.into();
        let bytes = text.as_bytes();
        let len = u32_len(bytes.len())?;
        let status =
            unsafe { (self.api.fns.cell_set_text)(self.handle, cell_to_addr(cell), bytes.as_ptr(), len) };
        self.status_result(status, "dvc_cell_set_text")
    }

    pub fn set_formula(&mut self, cell: CellRef, formula: &str) -> Result<(), EngineError> {
        let bytes = formula.as_bytes();
        let len = u32_len(bytes.len())?;
        let status = unsafe {
            (self.api.fns.cell_set_formula)(self.handle, cell_to_addr(cell), bytes.as_ptr(), len)
        };
        self.status_result(status, "dvc_cell_set_formula")
    }

    pub fn clear_cell(&mut self, cell: CellRef) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.cell_clear)(self.handle, cell_to_addr(cell)) };
        self.status_result(status, "dvc_cell_clear")
    }

    pub fn set_number_a1(&mut self, cell_ref: &str, number: f64) -> Result<(), EngineError> {
        self.set_number(self.parse_cell_ref(cell_ref)?, number)
    }

    pub fn set_text_a1(&mut self, cell_ref: &str, text: impl Into<String>) -> Result<(), EngineError> {
        self.set_text(self.parse_cell_ref(cell_ref)?, text)
    }

    pub fn set_formula_a1(&mut self, cell_ref: &str, formula: &str) -> Result<(), EngineError> {
        self.set_formula(self.parse_cell_ref(cell_ref)?, formula)
    }

    pub fn clear_cell_a1(&mut self, cell_ref: &str) -> Result<(), EngineError> {
        self.clear_cell(self.parse_cell_ref(cell_ref)?)
    }

    pub fn set_cell_input_a1(
        &mut self,
        cell_ref: &str,
        input: CellInput,
    ) -> Result<(), EngineError> {
        self.set_cell_input(self.parse_cell_ref(cell_ref)?, input)
    }

    pub fn cell_state(&self, cell: CellRef) -> Result<CellState, EngineError> {
        let addr = cell_to_addr(cell);
        let mut raw = DvcCellState::default();
        let status = unsafe { (self.api.fns.cell_get_state)(self.handle, addr, &mut raw) };
        self.status_result(status, "dvc_cell_get_state")?;
        let text = if raw.value.value_type == DVC_VALUE_TEXT {
            Some(self.read_cell_text(addr)?)
        } else {
            None
        };
        let err_text = if raw.value.value_type == DVC_VALUE_ERROR {
            Some(self.read_cell_error(addr)?)
        } else {
            None
        };
        Ok(CellState {
            value: value_from_raw(raw.value, text, err_text),
            value_epoch: raw.value_epoch,
            stale: raw.stale != 0,
        })
    }

    pub fn cell_state_a1(&self, cell_ref: &str) -> Result<CellState, EngineError> {
        self.cell_state(self.parse_cell_ref(cell_ref)?)
    }

    pub fn cell_input(&self, cell: CellRef) -> Result<Option<CellInput>, EngineError> {
        let addr = cell_to_addr(cell);
        let mut input_type = DVC_INPUT_EMPTY;
        let status = unsafe { (self.api.fns.cell_get_input_type)(self.handle, addr, &mut input_type) };
        self.status_result(status, "dvc_cell_get_input_type")?;
        match input_type {
            DVC_INPUT_EMPTY => Ok(None),
            DVC_INPUT_NUMBER => {
                let text = self.read_cell_input_text(addr)?;
                let number = text
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| EngineError::InvalidNumber(text.clone()))?;
                Ok(Some(CellInput::Number(number)))
            }
            DVC_INPUT_TEXT => Ok(Some(CellInput::Text(self.read_cell_input_text(addr)?))),
            DVC_INPUT_FORMULA => Ok(Some(CellInput::Formula(self.read_cell_input_text(addr)?))),
            _ => Err(EngineError::Api {
                status: -8,
                op: "dvc_cell_get_input_type".to_string(),
                message: format!("unknown input type {input_type}"),
            }),
        }
    }

    pub fn cell_input_a1(&self, cell_ref: &str) -> Result<Option<CellInput>, EngineError> {
        self.cell_input(self.parse_cell_ref(cell_ref)?)
    }

    pub fn set_cell_input(&mut self, cell: CellRef, input: CellInput) -> Result<(), EngineError> {
        match input {
            CellInput::Number(n) => self.set_number(cell, n),
            CellInput::Text(text) => self.set_text(cell, text),
            CellInput::Formula(formula) => self.set_formula(cell, &formula),
        }
    }

    pub fn set_name_number(&mut self, name: &str, value: f64) -> Result<(), EngineError> {
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let status = unsafe { (self.api.fns.name_set_number)(self.handle, name_b.as_ptr(), name_len, value) };
        self.status_result(status, "dvc_name_set_number")
    }

    pub fn set_name_text(&mut self, name: &str, text: impl Into<String>) -> Result<(), EngineError> {
        let text = text.into();
        let name_b = name.as_bytes();
        let text_b = text.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let text_len = u32_len(text_b.len())?;
        let status = unsafe {
            (self.api.fns.name_set_text)(
                self.handle,
                name_b.as_ptr(),
                name_len,
                text_b.as_ptr(),
                text_len,
            )
        };
        self.status_result(status, "dvc_name_set_text")
    }

    pub fn set_name_formula(&mut self, name: &str, formula: &str) -> Result<(), EngineError> {
        let name_b = name.as_bytes();
        let formula_b = formula.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let formula_len = u32_len(formula_b.len())?;
        let status = unsafe {
            (self.api.fns.name_set_formula)(
                self.handle,
                name_b.as_ptr(),
                name_len,
                formula_b.as_ptr(),
                formula_len,
            )
        };
        self.status_result(status, "dvc_name_set_formula")
    }

    pub fn clear_name(&mut self, name: &str) -> Result<(), EngineError> {
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let status = unsafe { (self.api.fns.name_clear)(self.handle, name_b.as_ptr(), name_len) };
        self.status_result(status, "dvc_name_clear")
    }

    pub fn name_input(&self, name: &str) -> Result<Option<NameInput>, EngineError> {
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let mut input_type = DVC_INPUT_EMPTY;
        let status =
            unsafe { (self.api.fns.name_get_input_type)(self.handle, name_b.as_ptr(), name_len, &mut input_type) };
        self.status_result(status, "dvc_name_get_input_type")?;
        match input_type {
            DVC_INPUT_EMPTY => Ok(None),
            DVC_INPUT_NUMBER => {
                let text = self.read_name_input_text(name_b, name_len)?;
                let number = text
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| EngineError::InvalidNumber(text.clone()))?;
                Ok(Some(NameInput::Number(number)))
            }
            DVC_INPUT_TEXT => Ok(Some(NameInput::Text(self.read_name_input_text(name_b, name_len)?))),
            DVC_INPUT_FORMULA => Ok(Some(NameInput::Formula(self.read_name_input_text(name_b, name_len)?))),
            _ => Err(EngineError::Api {
                status: -8,
                op: "dvc_name_get_input_type".to_string(),
                message: format!("unknown input type {input_type}"),
            }),
        }
    }

    pub fn set_name_input(&mut self, name: &str, input: NameInput) -> Result<(), EngineError> {
        match input {
            NameInput::Number(n) => self.set_name_number(name, n),
            NameInput::Text(text) => self.set_name_text(name, text),
            NameInput::Formula(formula) => self.set_name_formula(name, &formula),
        }
    }
}

impl Engine {
    pub fn cell_format(&self, cell: CellRef) -> Result<CellFormat, EngineError> {
        let mut raw = DvcCellFormatRaw::default();
        let status = unsafe { (self.api.fns.cell_get_format)(self.handle, cell_to_addr(cell), &mut raw) };
        self.status_result(status, "dvc_cell_get_format")?;
        Ok(format_from_raw(raw))
    }

    pub fn cell_format_a1(&self, cell_ref: &str) -> Result<CellFormat, EngineError> {
        self.cell_format(self.parse_cell_ref(cell_ref)?)
    }

    pub fn set_cell_format(&mut self, cell: CellRef, format: CellFormat) -> Result<(), EngineError> {
        let raw = format_to_raw(&format);
        let status = unsafe { (self.api.fns.cell_set_format)(self.handle, cell_to_addr(cell), &raw) };
        self.status_result(status, "dvc_cell_set_format")
    }

    pub fn set_cell_format_a1(&mut self, cell_ref: &str, format: CellFormat) -> Result<(), EngineError> {
        self.set_cell_format(self.parse_cell_ref(cell_ref)?, format)
    }

    pub fn spill_anchor_for_cell(&self, cell: CellRef) -> Result<Option<CellRef>, EngineError> {
        let mut anchor = DvcCellAddr::default();
        let mut found = 0i32;
        let status = unsafe {
            (self.api.fns.cell_spill_anchor)(self.handle, cell_to_addr(cell), &mut anchor, &mut found)
        };
        self.status_result(status, "dvc_cell_spill_anchor")?;
        Ok((found != 0).then(|| addr_to_cell(anchor)))
    }

    pub fn spill_range_for_cell(&self, cell: CellRef) -> Result<Option<CellRange>, EngineError> {
        let mut range = DvcCellRange::default();
        let mut found = 0i32;
        let status = unsafe {
            (self.api.fns.cell_spill_range)(self.handle, cell_to_addr(cell), &mut range, &mut found)
        };
        self.status_result(status, "dvc_cell_spill_range")?;
        Ok((found != 0).then(|| range_from_raw(range)))
    }

    pub fn all_cell_inputs(&self) -> Vec<(CellRef, CellInput)> {
        let mut out = Vec::new();
        let mut iter: DvcIteratorHandle = std::ptr::null_mut();
        let status = unsafe { (self.api.fns.cell_iterate)(self.handle, &mut iter) };
        if status != DVC_OK || iter.is_null() {
            return out;
        }
        let guard = IterGuard::new(iter, self.api.fns.cell_iterator_destroy);

        loop {
            let mut addr = DvcCellAddr::default();
            let mut input_type = DVC_INPUT_EMPTY;
            let mut done = 0i32;
            let status = unsafe {
                (self.api.fns.cell_iterator_next)(guard.ptr, &mut addr, &mut input_type, &mut done)
            };
            if status != DVC_OK || done != 0 {
                break;
            }
            let text = self.read_iter_text("dvc_cell_iterator_get_text", |buf, buf_len, out_len| unsafe {
                (self.api.fns.cell_iterator_get_text)(guard.ptr, buf, buf_len, out_len)
            });
            let Ok(text) = text else { continue };
            let input = match input_type {
                DVC_INPUT_NUMBER => match text.trim().parse::<f64>() {
                    Ok(v) => CellInput::Number(v),
                    Err(_) => continue,
                },
                DVC_INPUT_TEXT => CellInput::Text(text),
                DVC_INPUT_FORMULA => CellInput::Formula(text),
                _ => continue,
            };
            out.push((addr_to_cell(addr), input));
        }
        out
    }

    pub fn all_name_inputs(&self) -> Vec<(String, NameInput)> {
        let mut out = Vec::new();
        let mut iter: DvcIteratorHandle = std::ptr::null_mut();
        let status = unsafe { (self.api.fns.name_iterate)(self.handle, &mut iter) };
        if status != DVC_OK || iter.is_null() {
            return out;
        }
        let guard = IterGuard::new(iter, self.api.fns.name_iterator_destroy);

        loop {
            let mut input_type = DVC_INPUT_EMPTY;
            let mut done = 0i32;
            let name = self.read_iter_text("dvc_name_iterator_next", |buf, buf_len, out_len| unsafe {
                (self.api.fns.name_iterator_next)(
                    guard.ptr,
                    buf,
                    buf_len,
                    out_len,
                    &mut input_type,
                    &mut done,
                )
            });
            let Ok(name) = name else { break };
            if done != 0 {
                break;
            }
            let text = self.read_iter_text("dvc_name_iterator_get_text", |buf, buf_len, out_len| unsafe {
                (self.api.fns.name_iterator_get_text)(guard.ptr, buf, buf_len, out_len)
            });
            let Ok(text) = text else { continue };
            let input = match input_type {
                DVC_INPUT_NUMBER => match text.trim().parse::<f64>() {
                    Ok(v) => NameInput::Number(v),
                    Err(_) => continue,
                },
                DVC_INPUT_TEXT => NameInput::Text(text),
                DVC_INPUT_FORMULA => NameInput::Formula(text),
                _ => continue,
            };
            out.push((name, input));
        }
        out
    }

    pub fn all_cell_formats(&self) -> Vec<(CellRef, CellFormat)> {
        let mut out = Vec::new();
        let mut iter: DvcIteratorHandle = std::ptr::null_mut();
        let status = unsafe { (self.api.fns.format_iterate)(self.handle, &mut iter) };
        if status != DVC_OK || iter.is_null() {
            return out;
        }
        let guard = IterGuard::new(iter, self.api.fns.format_iterator_destroy);
        loop {
            let mut addr = DvcCellAddr::default();
            let mut format = DvcCellFormatRaw::default();
            let mut done = 0i32;
            let status = unsafe {
                (self.api.fns.format_iterator_next)(guard.ptr, &mut addr, &mut format, &mut done)
            };
            if status != DVC_OK || done != 0 {
                break;
            }
            out.push((addr_to_cell(addr), format_from_raw(format)));
        }
        out
    }

    pub fn insert_row(&mut self, at: u16) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.insert_row)(self.handle, at) };
        self.status_result(status, "dvc_insert_row")
    }

    pub fn delete_row(&mut self, at: u16) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.delete_row)(self.handle, at) };
        self.status_result(status, "dvc_delete_row")
    }

    pub fn insert_col(&mut self, at: u16) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.insert_col)(self.handle, at) };
        self.status_result(status, "dvc_insert_col")
    }

    pub fn delete_col(&mut self, at: u16) -> Result<(), EngineError> {
        let status = unsafe { (self.api.fns.delete_col)(self.handle, at) };
        self.status_result(status, "dvc_delete_col")
    }
}

impl Engine {
    pub fn define_control(&mut self, name: &str, def: ControlDefinition) -> Result<(), EngineError> {
        let raw = control_def_to_raw(def);
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let status = unsafe { (self.api.fns.control_define)(self.handle, name_b.as_ptr(), name_len, &raw) };
        self.status_result(status, "dvc_control_define")
    }

    pub fn remove_control(&mut self, name: &str) -> bool {
        let name_b = name.as_bytes();
        let Ok(name_len) = u32_len(name_b.len()) else {
            return false;
        };
        let mut found = 0i32;
        let status =
            unsafe { (self.api.fns.control_remove)(self.handle, name_b.as_ptr(), name_len, &mut found) };
        status == DVC_OK && found != 0
    }

    pub fn set_control_value(&mut self, name: &str, value: f64) -> Result<(), EngineError> {
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let status =
            unsafe { (self.api.fns.control_set_value)(self.handle, name_b.as_ptr(), name_len, value) };
        self.status_result(status, "dvc_control_set_value")
    }

    pub fn control_value(&self, name: &str) -> Option<f64> {
        let name_b = name.as_bytes();
        let Ok(name_len) = u32_len(name_b.len()) else {
            return None;
        };
        let mut value = 0.0;
        let mut found = 0i32;
        let status = unsafe {
            (self.api.fns.control_get_value)(self.handle, name_b.as_ptr(), name_len, &mut value, &mut found)
        };
        (status == DVC_OK && found != 0).then_some(value)
    }

    pub fn control_definition(&self, name: &str) -> Option<ControlDefinition> {
        let name_b = name.as_bytes();
        let Ok(name_len) = u32_len(name_b.len()) else {
            return None;
        };
        let mut raw = DvcControlDefRaw::default();
        let mut found = 0i32;
        let status = unsafe {
            (self.api.fns.control_get_def)(self.handle, name_b.as_ptr(), name_len, &mut raw, &mut found)
        };
        (status == DVC_OK && found != 0).then_some(control_def_from_raw(raw))
    }

    pub fn all_controls(&self) -> Vec<(String, ControlDefinition, f64)> {
        let mut out = Vec::new();
        let mut iter: DvcIteratorHandle = std::ptr::null_mut();
        let status = unsafe { (self.api.fns.control_iterate)(self.handle, &mut iter) };
        if status != DVC_OK || iter.is_null() {
            return out;
        }
        let guard = IterGuard::new(iter, self.api.fns.control_iterator_destroy);
        loop {
            let mut def = DvcControlDefRaw::default();
            let mut value = 0.0;
            let mut done = 0i32;
            let name =
                self.read_iter_text("dvc_control_iterator_next", |buf, buf_len, out_len| unsafe {
                    (self.api.fns.control_iterator_next)(
                        guard.ptr,
                        buf,
                        buf_len,
                        out_len,
                        &mut def,
                        &mut value,
                        &mut done,
                    )
                });
            let Ok(name) = name else { break };
            if done != 0 {
                break;
            }
            out.push((name, control_def_from_raw(def), value));
        }
        out
    }

    pub fn define_chart(&mut self, name: &str, def: ChartDefinition) -> Result<(), EngineError> {
        let raw = DvcChartDefRaw {
            source_range: range_to_raw(def.source_range),
        };
        let name_b = name.as_bytes();
        let name_len = u32_len(name_b.len())?;
        let status = unsafe { (self.api.fns.chart_define)(self.handle, name_b.as_ptr(), name_len, &raw) };
        self.status_result(status, "dvc_chart_define")
    }

    pub fn remove_chart(&mut self, name: &str) -> bool {
        let name_b = name.as_bytes();
        let Ok(name_len) = u32_len(name_b.len()) else {
            return false;
        };
        let mut found = 0i32;
        let status =
            unsafe { (self.api.fns.chart_remove)(self.handle, name_b.as_ptr(), name_len, &mut found) };
        status == DVC_OK && found != 0
    }

    pub fn all_charts(&self) -> Vec<(String, ChartDefinition)> {
        let mut out = Vec::new();
        let mut iter: DvcIteratorHandle = std::ptr::null_mut();
        let status = unsafe { (self.api.fns.chart_iterate)(self.handle, &mut iter) };
        if status != DVC_OK || iter.is_null() {
            return out;
        }
        let guard = IterGuard::new(iter, self.api.fns.chart_iterator_destroy);
        loop {
            let mut raw = DvcChartDefRaw::default();
            let mut done = 0i32;
            let name = self.read_iter_text("dvc_chart_iterator_next", |buf, buf_len, out_len| unsafe {
                (self.api.fns.chart_iterator_next)(
                    guard.ptr,
                    buf,
                    buf_len,
                    out_len,
                    &mut raw,
                    &mut done,
                )
            });
            let Ok(name) = name else { break };
            if done != 0 {
                break;
            }
            out.push((
                name,
                ChartDefinition {
                    source_range: range_from_raw(raw.source_range),
                },
            ));
        }
        out
    }

    fn parse_cell_ref(&self, a1: &str) -> Result<CellRef, EngineError> {
        let bytes = a1.as_bytes();
        let len = u32_len(bytes.len())?;
        let mut addr = DvcCellAddr::default();
        let status = unsafe { (self.api.fns.parse_cell_ref)(self.handle, bytes.as_ptr(), len, &mut addr) };
        if status != DVC_OK {
            return Err(EngineError::InvalidCellRef(a1.to_string()));
        }
        Ok(addr_to_cell(addr))
    }

    fn read_cell_text(&self, addr: DvcCellAddr) -> Result<String, EngineError> {
        self.read_utf8("dvc_cell_get_text", |buf, buf_len, out_len| unsafe {
            (self.api.fns.cell_get_text)(self.handle, addr, buf, buf_len, out_len)
        })
    }

    fn read_cell_input_text(&self, addr: DvcCellAddr) -> Result<String, EngineError> {
        self.read_utf8("dvc_cell_get_input_text", |buf, buf_len, out_len| unsafe {
            (self.api.fns.cell_get_input_text)(self.handle, addr, buf, buf_len, out_len)
        })
    }

    fn read_name_input_text(&self, name: &[u8], name_len: u32) -> Result<String, EngineError> {
        self.read_utf8("dvc_name_get_input_text", |buf, buf_len, out_len| unsafe {
            (self.api.fns.name_get_input_text)(
                self.handle,
                name.as_ptr(),
                name_len,
                buf,
                buf_len,
                out_len,
            )
        })
    }

    fn read_cell_error(&self, addr: DvcCellAddr) -> Result<String, EngineError> {
        self.read_utf8("dvc_cell_error_message", |buf, buf_len, out_len| unsafe {
            (self.api.fns.cell_error_message)(self.handle, addr, buf, buf_len, out_len)
        })
    }

    fn read_last_error_message(&self) -> String {
        self.read_utf8("dvc_last_error_message", |buf, buf_len, out_len| unsafe {
            (self.api.fns.last_error_message)(self.handle, buf, buf_len, out_len)
        })
        .unwrap_or_default()
    }

    fn read_utf8<F>(&self, op: &str, mut f: F) -> Result<String, EngineError>
    where
        F: FnMut(*mut u8, u32, *mut u32) -> i32,
    {
        let mut len = 0u32;
        let status = f(std::ptr::null_mut(), 0, &mut len);
        self.status_result(status, op)?;
        if len == 0 {
            return Ok(String::new());
        }
        let mut buffer = vec![0u8; len as usize];
        let mut written = 0u32;
        let status = f(buffer.as_mut_ptr(), len, &mut written);
        self.status_result(status, op)?;
        buffer.truncate(written as usize);
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }

    fn read_iter_text<F>(&self, op: &str, f: F) -> Result<String, EngineError>
    where
        F: FnMut(*mut u8, u32, *mut u32) -> i32,
    {
        self.read_utf8(op, f)
    }

    fn status_result(&self, status: i32, op: &str) -> Result<(), EngineError> {
        if status == DVC_OK {
            return Ok(());
        }
        Err(EngineError::Api {
            status,
            op: op.to_string(),
            message: self.read_last_error_message(),
        })
    }

    fn error_message(&self, status: i32, op: &str) -> String {
        match self.status_result(status, op) {
            Ok(()) => String::new(),
            Err(err) => err.to_string(),
        }
    }
}

struct IterGuard {
    ptr: DvcIteratorHandle,
    destroy: unsafe extern "C" fn(DvcIteratorHandle) -> i32,
}

impl IterGuard {
    fn new(ptr: DvcIteratorHandle, destroy: unsafe extern "C" fn(DvcIteratorHandle) -> i32) -> Self {
        Self { ptr, destroy }
    }
}

impl Drop for IterGuard {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let _ = unsafe { (self.destroy)(self.ptr) };
        }
    }
}

fn u32_len(len: usize) -> Result<u32, EngineError> {
    u32::try_from(len).map_err(|_| EngineError::InvalidUtf8Length)
}

fn cell_to_addr(cell: CellRef) -> DvcCellAddr {
    DvcCellAddr {
        col: cell.col,
        row: cell.row,
    }
}

fn addr_to_cell(addr: DvcCellAddr) -> CellRef {
    CellRef {
        col: addr.col,
        row: addr.row,
    }
}

fn range_to_raw(range: CellRange) -> DvcCellRange {
    DvcCellRange {
        start: cell_to_addr(range.start),
        end: cell_to_addr(range.end),
    }
}

fn range_from_raw(range: DvcCellRange) -> CellRange {
    CellRange::new(addr_to_cell(range.start), addr_to_cell(range.end))
}

fn sheet_bounds_to_raw(bounds: SheetBounds) -> DvcSheetBounds {
    DvcSheetBounds {
        max_columns: bounds.max_columns,
        max_rows: bounds.max_rows,
    }
}

fn sheet_bounds_from_raw(bounds: DvcSheetBounds) -> SheetBounds {
    SheetBounds {
        max_columns: bounds.max_columns,
        max_rows: bounds.max_rows,
    }
}

fn recalc_mode_to_raw(mode: RecalcMode) -> i32 {
    match mode {
        RecalcMode::Automatic => DVC_RECALC_AUTOMATIC,
        RecalcMode::Manual => DVC_RECALC_MANUAL,
    }
}

fn recalc_mode_from_raw(mode: i32) -> RecalcMode {
    match mode {
        DVC_RECALC_MANUAL => RecalcMode::Manual,
        _ => RecalcMode::Automatic,
    }
}

fn iteration_config_to_raw(config: IterationConfig) -> DvcIterationConfigRaw {
    DvcIterationConfigRaw {
        enabled: if config.enabled { 1 } else { 0 },
        max_iterations: config.max_iterations,
        convergence_tolerance: config.convergence_tolerance,
    }
}

fn iteration_config_from_raw(raw: DvcIterationConfigRaw) -> IterationConfig {
    IterationConfig {
        enabled: raw.enabled != 0,
        max_iterations: raw.max_iterations,
        convergence_tolerance: raw.convergence_tolerance,
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

fn format_to_raw(format: &CellFormat) -> DvcCellFormatRaw {
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

fn value_from_raw(raw: DvcCellValue, text: Option<String>, err_text: Option<String>) -> Value {
    match raw.value_type {
        DVC_VALUE_NUMBER => Value::Number(raw.number),
        DVC_VALUE_TEXT => Value::Text(text.unwrap_or_default()),
        DVC_VALUE_BOOL => Value::Bool(raw.bool_val != 0),
        DVC_VALUE_BLANK => Value::Blank,
        DVC_VALUE_ERROR => {
            let msg = err_text.unwrap_or_default();
            Value::Error(match raw.error_kind {
                DVC_ERROR_DIV_ZERO => CellError::DivisionByZero,
                DVC_ERROR_VALUE => CellError::Value(msg),
                DVC_ERROR_NAME => CellError::Name(msg),
                DVC_ERROR_UNKNOWN_NAME => CellError::UnknownName(msg),
                DVC_ERROR_REF => CellError::Ref(msg),
                DVC_ERROR_SPILL => CellError::Spill(msg),
                DVC_ERROR_CYCLE => {
                    if msg.is_empty() {
                        CellError::Cycle(Vec::new())
                    } else {
                        CellError::Cycle(vec![msg])
                    }
                }
                DVC_ERROR_NA => CellError::Na,
                DVC_ERROR_NULL => CellError::Null,
                DVC_ERROR_NUM => CellError::Num(msg),
                _ => CellError::Value(msg),
            })
        }
        _ => Value::Blank,
    }
}
