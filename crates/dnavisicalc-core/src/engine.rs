use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::address::{
    AddressError, CellRange, CellRef, DEFAULT_SHEET_BOUNDS, SheetBounds, col_index_to_label,
    is_cell_reference_token, parse_cell_ref,
};
use crate::ast::{Expr, StructuralOp, expr_to_formula, rewrite_expr};
use crate::deps::{CalcTree, DependencyError, build_calc_tree_allow_cycles};
use crate::eval::{
    CellError, EvalContext, RuntimeValue, SUPPORTED_FUNCTIONS, UdfHandler, Value, Volatility,
};
use crate::experiments::spill_overlay::{SpillOverlayError, SpillOverlayPlanner};
use crate::experiments::spill_rewrite::{RewriteError, materialize_array_values};
use crate::parser::{ParseError, parse_formula};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalcMode {
    Automatic,
    Manual,
}

/// Configuration for iterative calculation of circular references.
///
/// When enabled, the engine resolves circular dependencies by iterating
/// each strongly connected component up to `max_iterations` times, stopping
/// early if all values converge within `convergence_tolerance`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IterationConfig {
    pub enabled: bool,
    pub max_iterations: u32,
    pub convergence_tolerance: f64,
}

impl Default for IterationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_iterations: 100,
            convergence_tolerance: 0.001,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKind {
    Slider,
    Checkbox,
    Button,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlDefinition {
    pub kind: ControlKind,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ControlDefinition {
    pub const fn slider(min: f64, max: f64, step: f64) -> Self {
        Self {
            kind: ControlKind::Slider,
            min,
            max,
            step,
        }
    }

    pub const fn checkbox() -> Self {
        Self {
            kind: ControlKind::Checkbox,
            min: 0.0,
            max: 1.0,
            step: 1.0,
        }
    }

    pub const fn button() -> Self {
        Self {
            kind: ControlKind::Button,
            min: 0.0,
            max: 0.0,
            step: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartDefinition {
    pub source_range: CellRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeriesOutput {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartOutput {
    pub labels: Vec<String>,
    pub series: Vec<ChartSeriesOutput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    CircularReferenceDetected,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeEntry {
    CellValue {
        cell: CellRef,
        old: Value,
        new: Value,
        epoch: u64,
    },
    NameValue {
        name: String,
        old: Value,
        new: Value,
        epoch: u64,
    },
    ChartOutput {
        name: String,
        epoch: u64,
    },
    SpillRegion {
        anchor: CellRef,
        old_range: Option<CellRange>,
        new_range: Option<CellRange>,
        epoch: u64,
    },
    CellFormat {
        cell: CellRef,
        old: CellFormat,
        new: CellFormat,
        epoch: u64,
    },
    Diagnostic {
        code: DiagnosticCode,
        message: String,
        epoch: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicArrayStrategy {
    OverlayInline,
    OverlayPlanner,
    RewriteMaterialize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellInput {
    Number(f64),
    Text(String),
    Formula(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NameInput {
    Number(f64),
    Text(String),
    Formula(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaletteColor {
    Mist,
    Sage,
    Fern,
    Moss,
    Olive,
    Seafoam,
    Lagoon,
    Teal,
    Sky,
    Cloud,
    Sand,
    Clay,
    Peach,
    Rose,
    Lavender,
    Slate,
}

impl PaletteColor {
    pub const ALL: [PaletteColor; 16] = [
        PaletteColor::Mist,
        PaletteColor::Sage,
        PaletteColor::Fern,
        PaletteColor::Moss,
        PaletteColor::Olive,
        PaletteColor::Seafoam,
        PaletteColor::Lagoon,
        PaletteColor::Teal,
        PaletteColor::Sky,
        PaletteColor::Cloud,
        PaletteColor::Sand,
        PaletteColor::Clay,
        PaletteColor::Peach,
        PaletteColor::Rose,
        PaletteColor::Lavender,
        PaletteColor::Slate,
    ];

    pub fn as_name(self) -> &'static str {
        match self {
            Self::Mist => "MIST",
            Self::Sage => "SAGE",
            Self::Fern => "FERN",
            Self::Moss => "MOSS",
            Self::Olive => "OLIVE",
            Self::Seafoam => "SEAFOAM",
            Self::Lagoon => "LAGOON",
            Self::Teal => "TEAL",
            Self::Sky => "SKY",
            Self::Cloud => "CLOUD",
            Self::Sand => "SAND",
            Self::Clay => "CLAY",
            Self::Peach => "PEACH",
            Self::Rose => "ROSE",
            Self::Lavender => "LAVENDER",
            Self::Slate => "SLATE",
        }
    }

    pub fn from_name(input: &str) -> Option<Self> {
        match input.trim().to_ascii_uppercase().as_str() {
            "MIST" => Some(Self::Mist),
            "SAGE" => Some(Self::Sage),
            "FERN" => Some(Self::Fern),
            "MOSS" => Some(Self::Moss),
            "OLIVE" => Some(Self::Olive),
            "SEAFOAM" => Some(Self::Seafoam),
            "LAGOON" => Some(Self::Lagoon),
            "TEAL" => Some(Self::Teal),
            "SKY" => Some(Self::Sky),
            "CLOUD" => Some(Self::Cloud),
            "SAND" => Some(Self::Sand),
            "CLAY" => Some(Self::Clay),
            "PEACH" => Some(Self::Peach),
            "ROSE" => Some(Self::Rose),
            "LAVENDER" => Some(Self::Lavender),
            "SLATE" => Some(Self::Slate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellFormat {
    pub decimals: Option<u8>,
    pub bold: bool,
    pub italic: bool,
    pub fg: Option<PaletteColor>,
    pub bg: Option<PaletteColor>,
}

impl CellFormat {
    pub fn is_default(&self) -> bool {
        self.decimals.is_none()
            && !self.bold
            && !self.italic
            && self.fg.is_none()
            && self.bg.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellState {
    pub value: Value,
    pub value_epoch: u64,
    pub stale: bool,
}

#[derive(Debug, Clone)]
enum CellEntry {
    Number(f64),
    Text(String),
    Formula(FormulaEntry),
}

#[derive(Debug, Clone)]
enum NameEntry {
    Number(f64),
    Text(String),
    Formula(FormulaEntry),
}

#[derive(Debug, Clone)]
struct FormulaEntry {
    source: String,
    expr: Expr,
}

#[derive(Debug, Clone)]
struct StoredValue {
    value: Value,
    value_epoch: u64,
}

#[derive(Debug, Clone)]
struct StreamState {
    period_secs: f64,
    counter: u64,
    elapsed_accumulator: f64,
}

#[derive(Debug, Clone)]
struct ChangeBaseline {
    values: HashMap<CellRef, Value>,
    name_values: HashMap<String, Value>,
    spill_ranges: HashMap<CellRef, CellRange>,
    chart_outputs: HashMap<String, ChartOutput>,
}

#[derive(Debug)]
pub struct Engine {
    bounds: SheetBounds,
    mode: RecalcMode,
    committed_epoch: u64,
    stabilized_epoch: u64,
    cells: HashMap<CellRef, CellEntry>,
    formats: HashMap<CellRef, CellFormat>,
    names: HashMap<String, NameEntry>,
    values: HashMap<CellRef, StoredValue>,
    name_values: HashMap<String, StoredValue>,
    spill_owners: HashMap<CellRef, CellRef>,
    spill_ranges: HashMap<CellRef, CellRange>,
    calc_tree: Option<CalcTree>,
    recalc_serial: u64,
    dynamic_array_strategy: DynamicArrayStrategy,
    stream_cells: HashMap<CellRef, StreamState>,
    iteration_config: IterationConfig,
    /// Cells directly modified since the last recalculation.
    dirty_cells: HashSet<CellRef>,
    /// Names directly modified since the last recalculation.
    dirty_names: HashSet<String>,
    /// When true, forces a full recalculation (e.g. after formula structure changes).
    full_recalc_needed: bool,
    /// Reverse dependency map: maps a cell to the set of formula cells that reference it.
    /// Maintained incrementally as formulas are added/removed.
    reverse_deps: HashMap<CellRef, HashSet<CellRef>>,
    /// Number of formula cells evaluated in the last recalculation.
    last_eval_count: usize,
    /// Registered user-defined functions.
    udfs: HashMap<String, Box<dyn UdfHandler>>,
    /// Controls are named values with metadata.
    controls: HashMap<String, ControlDefinition>,
    /// Charts are engine-owned sink entities.
    charts: HashMap<String, ChartDefinition>,
    /// Latest computed chart outputs.
    chart_outputs: HashMap<String, ChartOutput>,
    /// Optional calc-delta capture.
    change_tracking_enabled: bool,
    /// Accumulated change entries since last drain.
    change_journal: Vec<ChangeEntry>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::with_bounds(DEFAULT_SHEET_BOUNDS)
    }

    pub fn with_bounds(bounds: SheetBounds) -> Self {
        Self {
            bounds,
            mode: RecalcMode::Automatic,
            committed_epoch: 0,
            stabilized_epoch: 0,
            cells: HashMap::new(),
            formats: HashMap::new(),
            names: HashMap::new(),
            values: HashMap::new(),
            name_values: HashMap::new(),
            spill_owners: HashMap::new(),
            spill_ranges: HashMap::new(),
            calc_tree: None,
            recalc_serial: 0,
            dynamic_array_strategy: DynamicArrayStrategy::OverlayInline,
            stream_cells: HashMap::new(),
            iteration_config: IterationConfig::default(),
            dirty_cells: HashSet::new(),
            dirty_names: HashSet::new(),
            full_recalc_needed: true,
            reverse_deps: HashMap::new(),
            last_eval_count: 0,
            udfs: HashMap::new(),
            controls: HashMap::new(),
            charts: HashMap::new(),
            chart_outputs: HashMap::new(),
            change_tracking_enabled: false,
            change_journal: Vec::new(),
        }
    }

    pub fn bounds(&self) -> SheetBounds {
        self.bounds
    }

    pub fn recalc_mode(&self) -> RecalcMode {
        self.mode
    }

    pub fn set_recalc_mode(&mut self, mode: RecalcMode) {
        self.mode = mode;
    }

    pub fn dynamic_array_strategy(&self) -> DynamicArrayStrategy {
        self.dynamic_array_strategy
    }

    pub fn set_dynamic_array_strategy(&mut self, strategy: DynamicArrayStrategy) {
        self.dynamic_array_strategy = strategy;
    }

    pub fn iteration_config(&self) -> IterationConfig {
        self.iteration_config
    }

    pub fn set_iteration_config(&mut self, config: IterationConfig) {
        self.iteration_config = config;
    }

    pub fn clear(&mut self) {
        let baseline = self.capture_change_baseline();
        let old_formats = if self.change_tracking_enabled {
            Some(self.formats.clone())
        } else {
            None
        };
        self.committed_epoch += 1;
        self.cells.clear();
        self.formats.clear();
        self.names.clear();
        self.values.clear();
        self.name_values.clear();
        self.spill_owners.clear();
        self.spill_ranges.clear();
        self.calc_tree = None;
        self.stream_cells.clear();
        self.dirty_cells.clear();
        self.dirty_names.clear();
        self.full_recalc_needed = true;
        self.reverse_deps.clear();
        self.controls.clear();
        self.charts.clear();
        self.chart_outputs.clear();
        if self.mode == RecalcMode::Automatic {
            self.stabilized_epoch = self.committed_epoch;
        }
        if let Some(baseline) = baseline {
            self.record_changes_from_baseline(baseline);
        }
        if let Some(old_formats) = old_formats {
            for (cell, old) in old_formats {
                self.push_change(ChangeEntry::CellFormat {
                    cell,
                    old,
                    new: CellFormat::default(),
                    epoch: self.committed_epoch,
                });
            }
        }
    }

    pub fn committed_epoch(&self) -> u64 {
        self.committed_epoch
    }

    pub fn stabilized_epoch(&self) -> u64 {
        self.stabilized_epoch
    }

    pub fn define_control(
        &mut self,
        name: &str,
        def: ControlDefinition,
    ) -> Result<(), EngineError> {
        self.validate_control_definition(def)?;
        let key = self.normalize_name(name)?;
        let existing_number = match self.names.get(&key) {
            Some(NameEntry::Number(n)) => Some(*n),
            Some(NameEntry::Text(_)) | Some(NameEntry::Formula(_)) | None => None,
        };
        let initial = self.normalize_control_value(
            def,
            existing_number.unwrap_or_else(|| Self::default_control_value(def)),
            false,
        )?;

        self.controls.insert(key.clone(), def);
        self.names.insert(key.clone(), NameEntry::Number(initial));
        self.dirty_names.insert(key);
        self.full_recalc_needed = true;
        self.committed_epoch += 1;
        self.maybe_recalculate()
    }

    pub fn remove_control(&mut self, name: &str) -> bool {
        let Some(key) = Self::normalize_lookup_name(name) else {
            return false;
        };
        let removed = self.controls.remove(&key).is_some();
        if removed {
            self.mark_presentation_change();
        }
        removed
    }

    pub fn set_control_value(&mut self, name: &str, value: f64) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        let Some(def) = self.controls.get(&key).copied() else {
            return Err(EngineError::Name(format!("control '{key}' is not defined")));
        };
        let normalized = self.normalize_control_value(def, value, true)?;
        self.set_name_number(&key, normalized)
    }

    pub fn control_value(&self, name: &str) -> Option<f64> {
        let key = Self::normalize_lookup_name(name)?;
        self.control_value_by_key(&key)
    }

    pub fn control_definition(&self, name: &str) -> Option<&ControlDefinition> {
        let key = Self::normalize_lookup_name(name)?;
        self.controls.get(&key)
    }

    pub fn all_controls(&self) -> Vec<(String, ControlDefinition, f64)> {
        let mut keys: Vec<String> = self.controls.keys().cloned().collect();
        keys.sort();
        keys.into_iter()
            .filter_map(|key| {
                let def = self.controls.get(&key).copied()?;
                let value = self
                    .control_value_by_key(&key)
                    .unwrap_or_else(|| Self::default_control_value(def));
                Some((key, def, value))
            })
            .collect()
    }

    pub fn define_chart(&mut self, name: &str, def: ChartDefinition) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        self.ensure_in_bounds(def.source_range.start)?;
        self.ensure_in_bounds(def.source_range.end)?;

        let baseline = self.capture_change_baseline();
        self.charts.insert(key, def);
        self.chart_outputs = self.compute_chart_outputs_from_values(&self.values);
        self.mark_presentation_change();
        if let Some(baseline) = baseline {
            self.record_changes_from_baseline(baseline);
        }
        Ok(())
    }

    pub fn remove_chart(&mut self, name: &str) -> bool {
        let Some(key) = Self::normalize_lookup_name(name) else {
            return false;
        };
        if !self.charts.contains_key(&key) {
            return false;
        }
        let baseline = self.capture_change_baseline();
        self.charts.remove(&key);
        self.chart_outputs.remove(&key);
        self.mark_presentation_change();
        if let Some(baseline) = baseline {
            self.record_changes_from_baseline(baseline);
        }
        true
    }

    pub fn chart_output(&self, name: &str) -> Option<&ChartOutput> {
        let key = Self::normalize_lookup_name(name)?;
        self.chart_outputs.get(&key)
    }

    pub fn all_charts(&self) -> Vec<(String, ChartDefinition)> {
        let mut entries: Vec<(String, ChartDefinition)> = self
            .charts
            .iter()
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));
        entries
    }

    pub fn enable_change_tracking(&mut self) {
        self.change_tracking_enabled = true;
    }

    pub fn disable_change_tracking(&mut self) {
        self.change_tracking_enabled = false;
        self.change_journal.clear();
    }

    pub fn is_change_tracking_enabled(&self) -> bool {
        self.change_tracking_enabled
    }

    pub fn drain_changes(&mut self) -> Vec<ChangeEntry> {
        std::mem::take(&mut self.change_journal)
    }

    pub fn set_number(&mut self, cell: CellRef, number: f64) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let old_value = self
            .values
            .get(&cell)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        let was_formula = matches!(self.cells.get(&cell), Some(CellEntry::Formula(_)));
        if was_formula {
            self.remove_reverse_deps_for(cell);
            self.full_recalc_needed = true;
        }
        self.cells.insert(cell, CellEntry::Number(number));
        self.committed_epoch += 1;
        self.dirty_cells.insert(cell);
        self.values.insert(
            cell,
            StoredValue {
                value: Value::Number(number),
                value_epoch: self.committed_epoch,
            },
        );
        if old_value != Value::Number(number) {
            self.push_change(ChangeEntry::CellValue {
                cell,
                old: old_value,
                new: Value::Number(number),
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_text(&mut self, cell: CellRef, text: impl Into<String>) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let old_value = self
            .values
            .get(&cell)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        let was_formula = matches!(self.cells.get(&cell), Some(CellEntry::Formula(_)));
        if was_formula {
            self.remove_reverse_deps_for(cell);
            self.full_recalc_needed = true;
        }
        let text = text.into();
        self.cells.insert(cell, CellEntry::Text(text.clone()));
        self.committed_epoch += 1;
        self.dirty_cells.insert(cell);
        self.values.insert(
            cell,
            StoredValue {
                value: Value::Text(text.clone()),
                value_epoch: self.committed_epoch,
            },
        );
        if old_value != Value::Text(text.clone()) {
            self.push_change(ChangeEntry::CellValue {
                cell,
                old: old_value,
                new: Value::Text(text.clone()),
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_formula(&mut self, cell: CellRef, formula: &str) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let expr = parse_formula(formula, self.bounds)?;
        // Remove old reverse deps if this cell had a formula.
        if matches!(self.cells.get(&cell), Some(CellEntry::Formula(_))) {
            self.remove_reverse_deps_for(cell);
        }
        self.cells.insert(
            cell,
            CellEntry::Formula(FormulaEntry {
                source: formula.to_string(),
                expr,
            }),
        );
        self.dirty_cells.insert(cell);
        self.full_recalc_needed = true; // graph structure changed
        self.committed_epoch += 1;
        self.maybe_recalculate()
    }

    pub fn clear_cell(&mut self, cell: CellRef) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let old_value = self
            .values
            .get(&cell)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        let was_formula = matches!(self.cells.get(&cell), Some(CellEntry::Formula(_)));
        if was_formula {
            self.remove_reverse_deps_for(cell);
            self.full_recalc_needed = true;
        }
        self.cells.remove(&cell);
        self.values.remove(&cell);
        self.dirty_cells.insert(cell);
        self.committed_epoch += 1;
        if old_value != Value::Blank {
            self.push_change(ChangeEntry::CellValue {
                cell,
                old: old_value,
                new: Value::Blank,
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_name_number(&mut self, name: &str, number: f64) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        let old_value = self
            .name_values
            .get(&key)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        self.dirty_names.insert(key.clone());
        self.full_recalc_needed = true; // names affect all formulas referencing them
        self.names.insert(key.clone(), NameEntry::Number(number));
        self.committed_epoch += 1;
        self.name_values.insert(
            key.clone(),
            StoredValue {
                value: Value::Number(number),
                value_epoch: self.committed_epoch,
            },
        );
        if old_value != Value::Number(number) {
            self.push_change(ChangeEntry::NameValue {
                name: key,
                old: old_value,
                new: Value::Number(number),
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_name_text(
        &mut self,
        name: &str,
        text: impl Into<String>,
    ) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        let old_value = self
            .name_values
            .get(&key)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        self.dirty_names.insert(key.clone());
        self.full_recalc_needed = true;
        let text = text.into();
        self.names
            .insert(key.clone(), NameEntry::Text(text.clone()));
        self.committed_epoch += 1;
        self.name_values.insert(
            key.clone(),
            StoredValue {
                value: Value::Text(text.clone()),
                value_epoch: self.committed_epoch,
            },
        );
        if old_value != Value::Text(text.clone()) {
            self.push_change(ChangeEntry::NameValue {
                name: key,
                old: old_value,
                new: Value::Text(text),
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_name_formula(&mut self, name: &str, formula: &str) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        self.dirty_names.insert(key.clone());
        self.full_recalc_needed = true;
        let expr = parse_formula(formula, self.bounds)?;
        self.names.insert(
            key,
            NameEntry::Formula(FormulaEntry {
                source: formula.to_string(),
                expr,
            }),
        );
        self.committed_epoch += 1;
        self.maybe_recalculate()
    }

    pub fn set_name_input(&mut self, name: &str, input: NameInput) -> Result<(), EngineError> {
        match input {
            NameInput::Number(n) => self.set_name_number(name, n),
            NameInput::Text(t) => self.set_name_text(name, t),
            NameInput::Formula(f) => self.set_name_formula(name, &f),
        }
    }

    pub fn clear_name(&mut self, name: &str) -> Result<(), EngineError> {
        let key = self.normalize_name(name)?;
        let old_value = self
            .name_values
            .get(&key)
            .map(|stored| stored.value.clone())
            .unwrap_or(Value::Blank);
        self.dirty_names.insert(key.clone());
        self.full_recalc_needed = true;
        self.names.remove(&key);
        self.name_values.remove(&key);
        self.committed_epoch += 1;
        if old_value != Value::Blank {
            self.push_change(ChangeEntry::NameValue {
                name: key,
                old: old_value,
                new: Value::Blank,
                epoch: self.committed_epoch,
            });
        }
        self.maybe_recalculate()
    }

    pub fn set_number_a1(&mut self, cell_ref: &str, number: f64) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.set_number(cell, number)
    }

    pub fn set_formula_a1(&mut self, cell_ref: &str, formula: &str) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.set_formula(cell, formula)
    }

    pub fn set_text_a1(
        &mut self,
        cell_ref: &str,
        text: impl Into<String>,
    ) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.set_text(cell, text)
    }

    pub fn clear_cell_a1(&mut self, cell_ref: &str) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.clear_cell(cell)
    }

    // -----------------------------------------------------------------------
    // Structural mutations: insert/delete rows and columns
    // -----------------------------------------------------------------------

    /// Insert a row at position `at` (1-based). Existing cells at row `at` and
    /// below are shifted down by one. All formula references are rewritten
    /// accordingly. Cells in the last row are discarded if shifted out of bounds.
    pub fn insert_row(&mut self, at: u16) -> Result<(), EngineError> {
        if at == 0 || at > self.bounds.max_rows {
            return Err(EngineError::OutOfBounds(CellRef { col: 1, row: at }));
        }
        self.apply_structural_op(StructuralOp::InsertRow { at })
    }

    /// Delete the row at position `at` (1-based). Cells below shift up. Any
    /// formula referencing a cell in the deleted row becomes `#REF!`.
    pub fn delete_row(&mut self, at: u16) -> Result<(), EngineError> {
        if at == 0 || at > self.bounds.max_rows {
            return Err(EngineError::OutOfBounds(CellRef { col: 1, row: at }));
        }
        self.apply_structural_op(StructuralOp::DeleteRow { at })
    }

    /// Insert a column at position `at` (1-based). Existing cells at column
    /// `at` and to the right are shifted right by one.
    pub fn insert_col(&mut self, at: u16) -> Result<(), EngineError> {
        if at == 0 || at > self.bounds.max_columns {
            return Err(EngineError::OutOfBounds(CellRef { col: at, row: 1 }));
        }
        self.apply_structural_op(StructuralOp::InsertCol { at })
    }

    /// Delete the column at position `at` (1-based). Cells to the right shift
    /// left. Any formula referencing a cell in the deleted column becomes `#REF!`.
    pub fn delete_col(&mut self, at: u16) -> Result<(), EngineError> {
        if at == 0 || at > self.bounds.max_columns {
            return Err(EngineError::OutOfBounds(CellRef { col: at, row: 1 }));
        }
        self.apply_structural_op(StructuralOp::DeleteCol { at })
    }

    /// Core implementation shared by all structural mutations.
    fn apply_structural_op(&mut self, op: StructuralOp) -> Result<(), EngineError> {
        let is_row_op = matches!(
            op,
            StructuralOp::InsertRow { .. } | StructuralOp::DeleteRow { .. }
        );
        let is_insert = matches!(
            op,
            StructuralOp::InsertRow { .. } | StructuralOp::InsertCol { .. }
        );

        // Phase 1: Relocate cell entries (shift coordinates).
        let old_cells: Vec<(CellRef, CellEntry)> = self.cells.drain().collect();
        let old_formats: Vec<(CellRef, CellFormat)> = self.formats.drain().collect();

        for (cell, entry) in old_cells {
            if let Some(new_cell) = shift_cell_ref(cell, op, is_row_op, is_insert, self.bounds) {
                self.cells.insert(new_cell, entry);
            }
            // If shift_cell_ref returns None, the cell was in the deleted row/col — discard it.
        }

        for (cell, fmt) in old_formats {
            if let Some(new_cell) = shift_cell_ref(cell, op, is_row_op, is_insert, self.bounds) {
                self.formats.insert(new_cell, fmt);
            }
        }

        // Phase 2: Rewrite formula ASTs and source strings.
        let cells_to_rewrite: Vec<CellRef> = self
            .cells
            .iter()
            .filter_map(|(cell, entry)| {
                if matches!(entry, CellEntry::Formula(_)) {
                    Some(*cell)
                } else {
                    None
                }
            })
            .collect();

        for cell in cells_to_rewrite {
            if let Some(CellEntry::Formula(formula_entry)) = self.cells.get(&cell) {
                if let Some(new_expr) = rewrite_expr(&formula_entry.expr, op, self.bounds) {
                    let new_source = expr_to_formula(&new_expr);
                    self.cells.insert(
                        cell,
                        CellEntry::Formula(FormulaEntry {
                            source: new_source,
                            expr: new_expr,
                        }),
                    );
                } else {
                    // Formula has an invalidated reference — replace with #REF!
                    // error text. The cell becomes a text cell that displays #REF!
                    // and evaluates to an error when referenced by other formulas.
                    self.cells
                        .insert(cell, CellEntry::Text("#REF!".to_string()));
                }
            }
        }

        // Phase 3: Rewrite name formulas.
        let name_keys: Vec<String> = self
            .names
            .iter()
            .filter_map(|(name, entry)| {
                if matches!(entry, NameEntry::Formula(_)) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        for name in name_keys {
            if let Some(NameEntry::Formula(formula_entry)) = self.names.get(&name) {
                if let Some(new_expr) = rewrite_expr(&formula_entry.expr, op, self.bounds) {
                    let new_source = expr_to_formula(&new_expr);
                    self.names.insert(
                        name,
                        NameEntry::Formula(FormulaEntry {
                            source: new_source,
                            expr: new_expr,
                        }),
                    );
                } else {
                    self.names.remove(&name);
                }
            }
        }

        // Phase 4: Relocate stream cells.
        let old_streams: Vec<(CellRef, StreamState)> = self.stream_cells.drain().collect();
        for (cell, state) in old_streams {
            if let Some(new_cell) = shift_cell_ref(cell, op, is_row_op, is_insert, self.bounds) {
                self.stream_cells.insert(new_cell, state);
            }
        }

        // Phase 5: Clear cached values and force full recalculate.
        self.values.clear();
        self.spill_owners.clear();
        self.spill_ranges.clear();
        self.calc_tree = None;
        self.reverse_deps.clear();
        self.dirty_cells.clear();
        self.dirty_names.clear();
        self.full_recalc_needed = true;
        self.committed_epoch += 1;
        self.maybe_recalculate()
    }

    pub fn recalculate(&mut self) -> Result<(), EngineError> {
        // Determine whether we can use incremental recalc.
        // Incremental is possible when:
        // 1. The dependency graph structure hasn't changed (no new/removed formulas)
        // 2. We have a cached CalcTree and reverse_deps
        // 3. There are dirty cells to propagate
        let use_incremental =
            !self.full_recalc_needed && self.calc_tree.is_some() && !self.dirty_cells.is_empty();

        if use_incremental {
            self.recalculate_incremental()
        } else {
            self.recalculate_full()
        }
    }

    /// Full recalculation: rebuild dependency graph, evaluate all formulas.
    fn recalculate_full(&mut self) -> Result<(), EngineError> {
        let baseline = self.capture_change_baseline();
        let mut formulas: HashMap<CellRef, Expr> = HashMap::new();
        let mut literals: HashMap<CellRef, f64> = HashMap::new();
        let mut text_literals: HashMap<CellRef, String> = HashMap::new();
        let mut name_formulas: HashMap<String, Expr> = HashMap::new();
        let mut name_literals: HashMap<String, f64> = HashMap::new();
        let mut name_text_literals: HashMap<String, String> = HashMap::new();

        for (cell, entry) in &self.cells {
            match entry {
                CellEntry::Number(n) => {
                    literals.insert(*cell, *n);
                }
                CellEntry::Text(t) => {
                    text_literals.insert(*cell, t.clone());
                }
                CellEntry::Formula(formula) => {
                    formulas.insert(*cell, formula.expr.clone());
                }
            }
        }
        for (name, entry) in &self.names {
            match entry {
                NameEntry::Number(n) => {
                    name_literals.insert(name.clone(), *n);
                }
                NameEntry::Text(t) => {
                    name_text_literals.insert(name.clone(), t.clone());
                }
                NameEntry::Formula(formula) => {
                    name_formulas.insert(name.clone(), formula.expr.clone());
                }
            }
        }

        let tree = build_calc_tree_allow_cycles(&formulas);

        self.recalc_serial = self.recalc_serial.wrapping_add(1);
        let now_timestamp = excel_now_timestamp();
        let stream_counters: HashMap<CellRef, u64> = self
            .stream_cells
            .iter()
            .map(|(cell, state)| (*cell, state.counter))
            .collect();
        let mut evaluator = EvalContext::new(
            &formulas,
            &literals,
            &text_literals,
            &name_formulas,
            &name_literals,
            &name_text_literals,
            self.bounds,
            self.recalc_serial,
            now_timestamp,
            &stream_counters,
            &self.udfs,
        );
        let mut new_values: HashMap<CellRef, StoredValue> = HashMap::new();
        let mut new_name_values: HashMap<String, StoredValue> = HashMap::new();
        let mut runtime_values: HashMap<CellRef, RuntimeValue> = HashMap::new();
        let mut eval_count: usize = 0;

        for (cell, number) in &literals {
            new_values.insert(
                *cell,
                StoredValue {
                    value: Value::Number(*number),
                    value_epoch: self.committed_epoch,
                },
            );
        }
        for (cell, text) in &text_literals {
            new_values.insert(
                *cell,
                StoredValue {
                    value: Value::Text(text.clone()),
                    value_epoch: self.committed_epoch,
                },
            );
        }

        for scc in &tree.sccs {
            if !scc.is_cyclic {
                for cell in &scc.cells {
                    let runtime = evaluator.evaluate_cell_runtime(*cell);
                    runtime_values.insert(*cell, runtime.clone());
                    let value = runtime.to_scalar();
                    new_values.insert(
                        *cell,
                        StoredValue {
                            value,
                            value_epoch: self.committed_epoch,
                        },
                    );
                    eval_count += 1;
                }
            } else {
                let max_iter = if self.iteration_config.enabled {
                    self.iteration_config.max_iterations
                } else {
                    1
                };
                let tolerance = self.iteration_config.convergence_tolerance;
                let seeded_prev = if self.iteration_config.enabled {
                    evaluator.begin_iteration(&scc.cells);
                    None
                } else {
                    let mut seeds: HashMap<CellRef, RuntimeValue> = HashMap::new();
                    for cell in &scc.cells {
                        let seed = self
                            .values
                            .get(cell)
                            .map(|stored| RuntimeValue::scalar(stored.value.clone()))
                            .unwrap_or_else(|| RuntimeValue::scalar(Value::Number(0.0)));
                        seeds.insert(*cell, seed);
                    }
                    evaluator.begin_iteration_seeded(&scc.cells, &seeds);
                    Some(seeds)
                };

                for cell in &scc.cells {
                    let seed_value = seeded_prev
                        .as_ref()
                        .and_then(|seeds| seeds.get(cell))
                        .map(RuntimeValue::to_scalar)
                        .unwrap_or(Value::Number(0.0));
                    new_values.insert(
                        *cell,
                        StoredValue {
                            value: seed_value,
                            value_epoch: self.committed_epoch,
                        },
                    );
                }

                for _iteration in 0..max_iter {
                    evaluator.advance_iteration(&scc.cells);

                    let mut converged = true;
                    for cell in &scc.cells {
                        let old_value = new_values
                            .get(cell)
                            .map(|sv| &sv.value)
                            .cloned()
                            .unwrap_or(Value::Number(0.0));
                        let runtime = evaluator.evaluate_cell_runtime(*cell);
                        runtime_values.insert(*cell, runtime.clone());
                        let new_value = runtime.to_scalar();

                        if let (Value::Number(old_n), Value::Number(new_n)) =
                            (&old_value, &new_value)
                        {
                            if (new_n - old_n).abs() > tolerance {
                                converged = false;
                            }
                        } else if old_value != new_value {
                            converged = false;
                        }

                        new_values.insert(
                            *cell,
                            StoredValue {
                                value: new_value,
                                value_epoch: self.committed_epoch,
                            },
                        );
                        eval_count += 1;
                    }

                    if self.iteration_config.enabled && converged {
                        break;
                    }
                }

                evaluator.end_iteration();
            }
        }

        let mut sorted_names: Vec<String> = self.names.keys().cloned().collect();
        sorted_names.sort();
        for name in sorted_names {
            let value = evaluator.evaluate_name_runtime(&name).to_scalar();
            new_name_values.insert(
                name,
                StoredValue {
                    value,
                    value_epoch: self.committed_epoch,
                },
            );
        }
        let emit_cycle_diagnostic =
            !self.iteration_config.enabled && evaluator.take_cycle_detected();

        let mut spill_owners: HashMap<CellRef, CellRef> = HashMap::new();
        let mut spill_ranges: HashMap<CellRef, CellRange> = HashMap::new();
        match self.dynamic_array_strategy {
            DynamicArrayStrategy::OverlayInline => {
                self.apply_spills_overlay_inline(
                    &tree.order,
                    &runtime_values,
                    &mut new_values,
                    &mut spill_owners,
                    &mut spill_ranges,
                );
            }
            DynamicArrayStrategy::OverlayPlanner => {
                self.apply_spills_overlay_planner(
                    &tree.order,
                    &runtime_values,
                    &mut new_values,
                    &mut spill_owners,
                    &mut spill_ranges,
                );
            }
            DynamicArrayStrategy::RewriteMaterialize => {
                self.apply_spills_rewrite_materialize(
                    &tree.order,
                    &runtime_values,
                    &mut new_values,
                    &mut spill_owners,
                    &mut spill_ranges,
                );
            }
        }

        let registrations = evaluator.take_stream_registrations();
        if emit_cycle_diagnostic {
            self.push_change(ChangeEntry::Diagnostic {
                code: DiagnosticCode::CircularReferenceDetected,
                message: "Circular reference detected; non-iterative fallback applied.".to_string(),
                epoch: self.committed_epoch,
            });
        }
        let mut new_stream_cells: HashMap<CellRef, StreamState> = HashMap::new();
        for (cell, reg) in registrations {
            if let Some(existing) = self.stream_cells.get(&cell) {
                new_stream_cells.insert(
                    cell,
                    StreamState {
                        period_secs: reg.period_secs,
                        counter: existing.counter,
                        elapsed_accumulator: existing.elapsed_accumulator,
                    },
                );
            } else {
                new_stream_cells.insert(
                    cell,
                    StreamState {
                        period_secs: reg.period_secs,
                        counter: 0,
                        elapsed_accumulator: 0.0,
                    },
                );
            }
        }
        self.stream_cells = new_stream_cells;
        let new_chart_outputs = self.compute_chart_outputs_from_values(&new_values);

        self.values = new_values;
        self.name_values = new_name_values;
        self.spill_owners = spill_owners;
        self.spill_ranges = spill_ranges;
        self.chart_outputs = new_chart_outputs;
        self.stabilized_epoch = self.committed_epoch;
        self.calc_tree = Some(tree);
        self.last_eval_count = eval_count;

        // Rebuild reverse dependency map after full recalc.
        self.rebuild_reverse_deps();
        self.dirty_cells.clear();
        self.dirty_names.clear();
        self.full_recalc_needed = false;
        if let Some(baseline) = baseline {
            self.record_changes_from_baseline(baseline);
        }

        Ok(())
    }

    /// Incremental recalculation: only re-evaluate cells in the dirty closure.
    fn recalculate_incremental(&mut self) -> Result<(), EngineError> {
        let baseline = self.capture_change_baseline();
        // Compute dirty closure: all formula cells transitively dependent on dirty cells.
        let dirty_closure = self.compute_dirty_closure();

        // Build evaluation context (same as full recalc — we need all formulas
        // available since dirty cells might reference any cell).
        let mut formulas: HashMap<CellRef, Expr> = HashMap::new();
        let mut literals: HashMap<CellRef, f64> = HashMap::new();
        let mut text_literals: HashMap<CellRef, String> = HashMap::new();
        let mut name_formulas: HashMap<String, Expr> = HashMap::new();
        let mut name_literals: HashMap<String, f64> = HashMap::new();
        let mut name_text_literals: HashMap<String, String> = HashMap::new();

        for (cell, entry) in &self.cells {
            match entry {
                CellEntry::Number(n) => {
                    literals.insert(*cell, *n);
                }
                CellEntry::Text(t) => {
                    text_literals.insert(*cell, t.clone());
                }
                CellEntry::Formula(formula) => {
                    formulas.insert(*cell, formula.expr.clone());
                }
            }
        }
        for (name, entry) in &self.names {
            match entry {
                NameEntry::Number(n) => {
                    name_literals.insert(name.clone(), *n);
                }
                NameEntry::Text(t) => {
                    name_text_literals.insert(name.clone(), t.clone());
                }
                NameEntry::Formula(formula) => {
                    name_formulas.insert(name.clone(), formula.expr.clone());
                }
            }
        }

        // We still need the CalcTree for evaluation order.
        // Use the cached tree (we know it exists since we checked in recalculate()).
        let tree = self.calc_tree.take().unwrap();

        self.recalc_serial = self.recalc_serial.wrapping_add(1);
        let now_timestamp = excel_now_timestamp();
        let stream_counters: HashMap<CellRef, u64> = self
            .stream_cells
            .iter()
            .map(|(cell, state)| (*cell, state.counter))
            .collect();
        let mut evaluator = EvalContext::new(
            &formulas,
            &literals,
            &text_literals,
            &name_formulas,
            &name_literals,
            &name_text_literals,
            self.bounds,
            self.recalc_serial,
            now_timestamp,
            &stream_counters,
            &self.udfs,
        );

        // Seed evaluator caches with current committed values, then evict dirty
        // entries before recomputing them. This keeps incremental recalc
        // dependency reads iterative and prevents deep recursive walks.
        let mut cell_cache_seed: HashMap<CellRef, RuntimeValue> = HashMap::new();
        for (cell, stored) in &self.values {
            if formulas.contains_key(cell) {
                cell_cache_seed.insert(*cell, RuntimeValue::scalar(stored.value.clone()));
            }
        }
        evaluator.seed_cell_cache(&cell_cache_seed);

        let mut name_cache_seed: HashMap<String, RuntimeValue> = HashMap::new();
        for (name, stored) in &self.name_values {
            if name_formulas.contains_key(name) {
                name_cache_seed.insert(name.clone(), RuntimeValue::scalar(stored.value.clone()));
            }
        }
        evaluator.seed_name_cache(&name_cache_seed);

        let mut runtime_values: HashMap<CellRef, RuntimeValue> = HashMap::new();
        let mut eval_count: usize = 0;

        // Re-evaluate only cells in the dirty closure, in dependency order.
        for scc in &tree.sccs {
            if !scc.is_cyclic {
                for cell in &scc.cells {
                    if dirty_closure.contains(cell) {
                        evaluator.evict_cell_cache(*cell);
                        let runtime = evaluator.evaluate_cell_runtime(*cell);
                        runtime_values.insert(*cell, runtime.clone());
                        let value = runtime.to_scalar();
                        self.values.insert(
                            *cell,
                            StoredValue {
                                value,
                                value_epoch: self.committed_epoch,
                            },
                        );
                        eval_count += 1;
                    }
                    // Clean cells keep their existing value in self.values.
                }
            } else {
                // For cyclic SCCs, if any member is dirty, re-evaluate the entire SCC.
                let scc_has_dirty = scc.cells.iter().any(|c| dirty_closure.contains(c));
                if scc_has_dirty {
                    let max_iter = if self.iteration_config.enabled {
                        self.iteration_config.max_iterations
                    } else {
                        1
                    };
                    let tolerance = self.iteration_config.convergence_tolerance;
                    let seeded_prev = if self.iteration_config.enabled {
                        evaluator.begin_iteration(&scc.cells);
                        None
                    } else {
                        let mut seeds: HashMap<CellRef, RuntimeValue> = HashMap::new();
                        for cell in &scc.cells {
                            let seed = self
                                .values
                                .get(cell)
                                .map(|stored| RuntimeValue::scalar(stored.value.clone()))
                                .unwrap_or_else(|| RuntimeValue::scalar(Value::Number(0.0)));
                            seeds.insert(*cell, seed);
                        }
                        evaluator.begin_iteration_seeded(&scc.cells, &seeds);
                        Some(seeds)
                    };

                    for cell in &scc.cells {
                        let seed_value = seeded_prev
                            .as_ref()
                            .and_then(|seeds| seeds.get(cell))
                            .map(RuntimeValue::to_scalar)
                            .unwrap_or(Value::Number(0.0));
                        self.values.insert(
                            *cell,
                            StoredValue {
                                value: seed_value,
                                value_epoch: self.committed_epoch,
                            },
                        );
                    }

                    for _iteration in 0..max_iter {
                        evaluator.advance_iteration(&scc.cells);

                        let mut converged = true;
                        for cell in &scc.cells {
                            let old_value = self
                                .values
                                .get(cell)
                                .map(|sv| &sv.value)
                                .cloned()
                                .unwrap_or(Value::Number(0.0));
                            let runtime = evaluator.evaluate_cell_runtime(*cell);
                            runtime_values.insert(*cell, runtime.clone());
                            let new_value = runtime.to_scalar();

                            if let (Value::Number(old_n), Value::Number(new_n)) =
                                (&old_value, &new_value)
                            {
                                if (new_n - old_n).abs() > tolerance {
                                    converged = false;
                                }
                            } else if old_value != new_value {
                                converged = false;
                            }

                            self.values.insert(
                                *cell,
                                StoredValue {
                                    value: new_value,
                                    value_epoch: self.committed_epoch,
                                },
                            );
                            eval_count += 1;
                        }

                        if self.iteration_config.enabled && converged {
                            break;
                        }
                    }

                    evaluator.end_iteration();
                }
            }
        }

        // Re-evaluate names if any are dirty.
        if !self.dirty_names.is_empty() {
            let mut sorted_names: Vec<String> = self.names.keys().cloned().collect();
            sorted_names.sort();
            for name in sorted_names {
                evaluator.evict_name_cache(&name);
                let value = evaluator.evaluate_name_runtime(&name).to_scalar();
                self.name_values.insert(
                    name,
                    StoredValue {
                        value,
                        value_epoch: self.committed_epoch,
                    },
                );
            }
        }
        let emit_cycle_diagnostic =
            !self.iteration_config.enabled && evaluator.take_cycle_detected();

        // Spill handling can be skipped for pure scalar updates when no dirty
        // cell affects existing spill state and no dirty formula produced a
        // spilled array.
        let dirty_touches_existing_spill_state = dirty_closure.iter().any(|cell| {
            self.spill_ranges.contains_key(cell) || self.spill_owners.contains_key(cell)
        });
        let dirty_produced_spill_array = runtime_values
            .values()
            .any(|runtime| runtime.as_array().is_some_and(|array| array.is_spill()));

        if !dirty_closure.is_empty()
            && (dirty_touches_existing_spill_state || dirty_produced_spill_array)
        {
            let mut spill_owners: HashMap<CellRef, CellRef> = HashMap::new();
            let mut spill_ranges: HashMap<CellRef, CellRange> = HashMap::new();
            // Build a combined values map for spill processing.
            let mut combined_values = self.values.clone();
            match self.dynamic_array_strategy {
                DynamicArrayStrategy::OverlayInline => {
                    self.apply_spills_overlay_inline(
                        &tree.order,
                        &runtime_values,
                        &mut combined_values,
                        &mut spill_owners,
                        &mut spill_ranges,
                    );
                }
                DynamicArrayStrategy::OverlayPlanner => {
                    self.apply_spills_overlay_planner(
                        &tree.order,
                        &runtime_values,
                        &mut combined_values,
                        &mut spill_owners,
                        &mut spill_ranges,
                    );
                }
                DynamicArrayStrategy::RewriteMaterialize => {
                    self.apply_spills_rewrite_materialize(
                        &tree.order,
                        &runtime_values,
                        &mut combined_values,
                        &mut spill_owners,
                        &mut spill_ranges,
                    );
                }
            }
            self.values = combined_values;
            self.spill_owners = spill_owners;
            self.spill_ranges = spill_ranges;
        }

        // Handle stream registrations.
        let registrations = evaluator.take_stream_registrations();
        if emit_cycle_diagnostic {
            self.push_change(ChangeEntry::Diagnostic {
                code: DiagnosticCode::CircularReferenceDetected,
                message: "Circular reference detected; non-iterative fallback applied.".to_string(),
                epoch: self.committed_epoch,
            });
        }
        if !dirty_closure.is_empty() {
            let old_stream_cells = self.stream_cells.clone();
            let mut merged = old_stream_cells.clone();
            for cell in &dirty_closure {
                merged.remove(cell);
            }
            for (cell, reg) in registrations {
                if let Some(existing) = old_stream_cells.get(&cell) {
                    merged.insert(
                        cell,
                        StreamState {
                            period_secs: reg.period_secs,
                            counter: existing.counter,
                            elapsed_accumulator: existing.elapsed_accumulator,
                        },
                    );
                } else {
                    merged.insert(
                        cell,
                        StreamState {
                            period_secs: reg.period_secs,
                            counter: 0,
                            elapsed_accumulator: 0.0,
                        },
                    );
                }
            }
            self.stream_cells = merged;
        }

        self.chart_outputs = self.compute_chart_outputs_from_values(&self.values);
        self.stabilized_epoch = self.committed_epoch;
        self.calc_tree = Some(tree);
        self.last_eval_count = eval_count;
        self.dirty_cells.clear();
        self.dirty_names.clear();
        if let Some(baseline) = baseline {
            self.record_changes_from_baseline(baseline);
        }

        Ok(())
    }

    pub fn has_volatile_cells(&self) -> bool {
        for entry in self.cells.values() {
            if let CellEntry::Formula(formula) = entry {
                if expr_contains_volatility(&formula.expr, &self.udfs, Volatility::Volatile) {
                    return true;
                }
            }
        }
        false
    }

    pub fn has_externally_invalidated_cells(&self) -> bool {
        for entry in self.cells.values() {
            if let CellEntry::Formula(formula) = entry {
                if expr_contains_volatility(
                    &formula.expr,
                    &self.udfs,
                    Volatility::ExternallyInvalidated,
                ) {
                    return true;
                }
            }
        }
        false
    }

    pub fn invalidate_volatile(&mut self) -> Result<(), EngineError> {
        let volatile_cells: Vec<CellRef> = self
            .cells
            .iter()
            .filter_map(|(cell, entry)| match entry {
                CellEntry::Formula(formula)
                    if expr_contains_volatility(
                        &formula.expr,
                        &self.udfs,
                        Volatility::Volatile,
                    ) =>
                {
                    Some(*cell)
                }
                CellEntry::Number(_) | CellEntry::Text(_) | CellEntry::Formula(_) => None,
            })
            .collect();

        if volatile_cells.is_empty() {
            return Ok(());
        }
        self.committed_epoch += 1;
        for cell in volatile_cells {
            self.dirty_cells.insert(cell);
        }
        self.maybe_recalculate()
    }

    pub fn has_stream_cells(&self) -> bool {
        !self.stream_cells.is_empty()
    }

    pub fn tick_streams(&mut self, elapsed_secs: f64) -> bool {
        let mut any_advanced = false;
        let mut dirty: Vec<CellRef> = Vec::new();
        for (cell, state) in &mut self.stream_cells {
            let mut advanced_this_cell = false;
            state.elapsed_accumulator += elapsed_secs;
            while state.elapsed_accumulator >= state.period_secs {
                state.elapsed_accumulator -= state.period_secs;
                state.counter += 1;
                any_advanced = true;
                advanced_this_cell = true;
            }
            if advanced_this_cell {
                dirty.push(*cell);
            }
        }
        if any_advanced {
            self.committed_epoch += 1;
            for cell in dirty {
                self.dirty_cells.insert(cell);
            }
            let _ = self.maybe_recalculate();
        }
        any_advanced
    }

    pub fn invalidate_udf(&mut self, name: &str) -> Result<(), EngineError> {
        let key = name.trim().to_ascii_uppercase();
        if key.is_empty() {
            return Err(EngineError::Name("UDF name cannot be empty".to_string()));
        }
        let Some(handler) = self.udfs.get(&key) else {
            return Err(EngineError::Name(format!("UDF '{key}' is not registered")));
        };
        if handler.volatility() != Volatility::ExternallyInvalidated {
            return Err(EngineError::Name(format!(
                "UDF '{key}' is not externally invalidated"
            )));
        }

        let mut dirty_cells: Vec<CellRef> = Vec::new();
        for (cell, entry) in &self.cells {
            if let CellEntry::Formula(formula) = entry
                && expr_calls_udf(&formula.expr, &key)
            {
                dirty_cells.push(*cell);
            }
        }

        let mut dirty_names: Vec<String> = Vec::new();
        for (name_key, entry) in &self.names {
            if let NameEntry::Formula(formula) = entry
                && expr_calls_udf(&formula.expr, &key)
            {
                dirty_names.push(name_key.clone());
            }
        }

        if dirty_cells.is_empty() && dirty_names.is_empty() {
            return Ok(());
        }

        self.committed_epoch += 1;
        for cell in dirty_cells {
            self.dirty_cells.insert(cell);
        }
        if !dirty_names.is_empty() {
            self.full_recalc_needed = true;
            for name_key in dirty_names {
                self.dirty_names.insert(name_key);
            }
        }
        self.maybe_recalculate()
    }

    pub fn cell_state(&self, cell: CellRef) -> Result<CellState, EngineError> {
        self.ensure_in_bounds(cell)?;
        let state = if let Some(stored) = self.values.get(&cell) {
            CellState {
                value: stored.value.clone(),
                value_epoch: stored.value_epoch,
                stale: stored.value_epoch < self.committed_epoch,
            }
        } else {
            CellState {
                value: Value::Blank,
                value_epoch: self.stabilized_epoch,
                stale: self.stabilized_epoch < self.committed_epoch,
            }
        };
        Ok(state)
    }

    pub fn cell_state_a1(&self, cell_ref: &str) -> Result<CellState, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.cell_state(cell)
    }

    pub fn formula_source_a1(&self, cell_ref: &str) -> Result<Option<&str>, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        let source = self.cells.get(&cell).and_then(|entry| match entry {
            CellEntry::Formula(formula) => Some(formula.source.as_str()),
            CellEntry::Number(_) | CellEntry::Text(_) => None,
        });
        Ok(source)
    }

    pub fn calc_tree(&self) -> Option<&CalcTree> {
        self.calc_tree.as_ref()
    }

    pub fn spill_anchor_for_cell(&self, cell: CellRef) -> Result<Option<CellRef>, EngineError> {
        self.ensure_in_bounds(cell)?;
        Ok(self.spill_owners.get(&cell).copied())
    }

    pub fn spill_anchor_for_cell_a1(&self, cell_ref: &str) -> Result<Option<CellRef>, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.spill_anchor_for_cell(cell)
    }

    pub fn spill_range_for_anchor(&self, cell: CellRef) -> Result<Option<CellRange>, EngineError> {
        self.ensure_in_bounds(cell)?;
        Ok(self.spill_ranges.get(&cell).cloned())
    }

    pub fn spill_range_for_cell(&self, cell: CellRef) -> Result<Option<CellRange>, EngineError> {
        self.ensure_in_bounds(cell)?;
        if let Some(range) = self.spill_ranges.get(&cell) {
            return Ok(Some(range.clone()));
        }
        if let Some(anchor) = self.spill_owners.get(&cell).copied() {
            return Ok(self.spill_ranges.get(&anchor).cloned());
        }
        Ok(None)
    }

    pub fn spill_range_for_cell_a1(
        &self,
        cell_ref: &str,
    ) -> Result<Option<CellRange>, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.spill_range_for_cell(cell)
    }

    pub fn set_cell_input(&mut self, cell: CellRef, input: CellInput) -> Result<(), EngineError> {
        match input {
            CellInput::Number(n) => self.set_number(cell, n),
            CellInput::Text(t) => self.set_text(cell, t),
            CellInput::Formula(f) => self.set_formula(cell, &f),
        }
    }

    pub fn set_cell_input_a1(
        &mut self,
        cell_ref: &str,
        input: CellInput,
    ) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.set_cell_input(cell, input)
    }

    pub fn cell_input(&self, cell: CellRef) -> Result<Option<CellInput>, EngineError> {
        self.ensure_in_bounds(cell)?;
        let entry = self.cells.get(&cell).map(|entry| match entry {
            CellEntry::Number(n) => CellInput::Number(*n),
            CellEntry::Text(t) => CellInput::Text(t.clone()),
            CellEntry::Formula(f) => CellInput::Formula(f.source.clone()),
        });
        Ok(entry)
    }

    pub fn cell_input_a1(&self, cell_ref: &str) -> Result<Option<CellInput>, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.cell_input(cell)
    }

    pub fn all_cell_inputs(&self) -> Vec<(CellRef, CellInput)> {
        let mut entries: Vec<(CellRef, CellInput)> = self
            .cells
            .iter()
            .map(|(cell, entry)| {
                let input = match entry {
                    CellEntry::Number(n) => CellInput::Number(*n),
                    CellEntry::Text(t) => CellInput::Text(t.clone()),
                    CellEntry::Formula(f) => CellInput::Formula(f.source.clone()),
                };
                (*cell, input)
            })
            .collect();
        entries.sort_by_key(|(cell, _)| *cell);
        entries
    }

    pub fn cell_format(&self, cell: CellRef) -> Result<CellFormat, EngineError> {
        self.ensure_in_bounds(cell)?;
        Ok(self.formats.get(&cell).cloned().unwrap_or_default())
    }

    pub fn cell_format_a1(&self, cell_ref: &str) -> Result<CellFormat, EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.cell_format(cell)
    }

    pub fn set_cell_format(
        &mut self,
        cell: CellRef,
        format: CellFormat,
    ) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let old_format = self.formats.get(&cell).cloned().unwrap_or_default();
        if format.is_default() {
            self.formats.remove(&cell);
        } else {
            self.formats.insert(cell, format);
        }
        let new_format = self.formats.get(&cell).cloned().unwrap_or_default();
        if old_format == new_format {
            return Ok(());
        }
        self.mark_presentation_change();
        self.push_change(ChangeEntry::CellFormat {
            cell,
            old: old_format,
            new: new_format,
            epoch: self.committed_epoch,
        });
        Ok(())
    }

    pub fn set_cell_format_a1(
        &mut self,
        cell_ref: &str,
        format: CellFormat,
    ) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.set_cell_format(cell, format)
    }

    pub fn all_cell_formats(&self) -> Vec<(CellRef, CellFormat)> {
        let mut entries: Vec<(CellRef, CellFormat)> = self
            .formats
            .iter()
            .map(|(cell, format)| (*cell, format.clone()))
            .collect();
        entries.sort_by_key(|(cell, _)| *cell);
        entries
    }

    pub fn name_input(&self, name: &str) -> Result<Option<NameInput>, EngineError> {
        let key = self.normalize_name(name)?;
        let entry = self.names.get(&key).map(|entry| match entry {
            NameEntry::Number(n) => NameInput::Number(*n),
            NameEntry::Text(t) => NameInput::Text(t.clone()),
            NameEntry::Formula(f) => NameInput::Formula(f.source.clone()),
        });
        Ok(entry)
    }

    pub fn all_name_inputs(&self) -> Vec<(String, NameInput)> {
        let mut entries: Vec<(String, NameInput)> = self
            .names
            .iter()
            .map(|(name, entry)| {
                let input = match entry {
                    NameEntry::Number(n) => NameInput::Number(*n),
                    NameEntry::Text(t) => NameInput::Text(t.clone()),
                    NameEntry::Formula(f) => NameInput::Formula(f.source.clone()),
                };
                (name.clone(), input)
            })
            .collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));
        entries
    }

    fn validate_control_definition(&self, def: ControlDefinition) -> Result<(), EngineError> {
        if def.kind == ControlKind::Slider {
            if def.min > def.max {
                return Err(EngineError::Name(
                    "slider control requires min <= max".to_string(),
                ));
            }
            if !(def.step.is_finite() && def.step > 0.0) {
                return Err(EngineError::Name(
                    "slider control requires step > 0".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn default_control_value(def: ControlDefinition) -> f64 {
        match def.kind {
            ControlKind::Slider => def.min,
            ControlKind::Checkbox | ControlKind::Button => 0.0,
        }
    }

    fn normalize_control_value(
        &self,
        def: ControlDefinition,
        value: f64,
        strict_checkbox: bool,
    ) -> Result<f64, EngineError> {
        if !value.is_finite() {
            return Err(EngineError::Name(
                "control value must be finite".to_string(),
            ));
        }
        match def.kind {
            ControlKind::Slider => Ok(value.clamp(def.min, def.max)),
            ControlKind::Checkbox => {
                if value == 0.0 || value == 1.0 {
                    Ok(value)
                } else if strict_checkbox {
                    Err(EngineError::Name(
                        "checkbox control value must be 0.0 or 1.0".to_string(),
                    ))
                } else {
                    Ok(0.0)
                }
            }
            ControlKind::Button => Ok(0.0),
        }
    }

    fn normalize_lookup_name(name: &str) -> Option<String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_ascii_uppercase())
        }
    }

    fn control_value_by_key(&self, key: &str) -> Option<f64> {
        if !self.controls.contains_key(key) {
            return None;
        }
        self.name_values
            .get(key)
            .and_then(|stored| stored.value.as_f64())
            .or_else(|| match self.names.get(key) {
                Some(NameEntry::Number(n)) => Some(*n),
                Some(NameEntry::Text(_)) | Some(NameEntry::Formula(_)) | None => None,
            })
    }

    fn capture_change_baseline(&self) -> Option<ChangeBaseline> {
        if !self.change_tracking_enabled {
            return None;
        }
        Some(ChangeBaseline {
            values: self
                .values
                .iter()
                .map(|(cell, stored)| (*cell, stored.value.clone()))
                .collect(),
            name_values: self
                .name_values
                .iter()
                .map(|(name, stored)| (name.clone(), stored.value.clone()))
                .collect(),
            spill_ranges: self.spill_ranges.clone(),
            chart_outputs: self.chart_outputs.clone(),
        })
    }

    fn record_changes_from_baseline(&mut self, baseline: ChangeBaseline) {
        let epoch = self.committed_epoch;

        let mut cell_keys: HashSet<CellRef> = baseline.values.keys().copied().collect();
        cell_keys.extend(self.values.keys().copied());
        let mut cell_keys: Vec<CellRef> = cell_keys.into_iter().collect();
        cell_keys.sort();
        for cell in cell_keys {
            let old = baseline.values.get(&cell).cloned().unwrap_or(Value::Blank);
            let new = self
                .values
                .get(&cell)
                .map(|stored| stored.value.clone())
                .unwrap_or(Value::Blank);
            if old != new {
                self.push_change(ChangeEntry::CellValue {
                    cell,
                    old,
                    new,
                    epoch,
                });
            }
        }

        let mut name_keys: HashSet<String> = baseline.name_values.keys().cloned().collect();
        name_keys.extend(self.name_values.keys().cloned());
        let mut name_keys: Vec<String> = name_keys.into_iter().collect();
        name_keys.sort();
        for name in name_keys {
            let old = baseline
                .name_values
                .get(&name)
                .cloned()
                .unwrap_or(Value::Blank);
            let new = self
                .name_values
                .get(&name)
                .map(|stored| stored.value.clone())
                .unwrap_or(Value::Blank);
            if old != new {
                self.push_change(ChangeEntry::NameValue {
                    name,
                    old,
                    new,
                    epoch,
                });
            }
        }

        let mut spill_anchors: HashSet<CellRef> = baseline.spill_ranges.keys().copied().collect();
        spill_anchors.extend(self.spill_ranges.keys().copied());
        let mut spill_anchors: Vec<CellRef> = spill_anchors.into_iter().collect();
        spill_anchors.sort();
        for anchor in spill_anchors {
            let old_range = baseline.spill_ranges.get(&anchor).cloned();
            let new_range = self.spill_ranges.get(&anchor).cloned();
            if old_range != new_range {
                self.push_change(ChangeEntry::SpillRegion {
                    anchor,
                    old_range,
                    new_range,
                    epoch,
                });
            }
        }

        let mut chart_names: HashSet<String> = baseline.chart_outputs.keys().cloned().collect();
        chart_names.extend(self.chart_outputs.keys().cloned());
        let mut chart_names: Vec<String> = chart_names.into_iter().collect();
        chart_names.sort();
        for name in chart_names {
            let old = baseline.chart_outputs.get(&name);
            let new = self.chart_outputs.get(&name);
            if old != new {
                self.push_change(ChangeEntry::ChartOutput { name, epoch });
            }
        }
    }

    fn push_change(&mut self, entry: ChangeEntry) {
        if self.change_tracking_enabled {
            self.change_journal.push(entry);
        }
    }

    fn compute_chart_outputs_from_values(
        &self,
        values: &HashMap<CellRef, StoredValue>,
    ) -> HashMap<String, ChartOutput> {
        let mut chart_names: Vec<String> = self.charts.keys().cloned().collect();
        chart_names.sort();
        let mut outputs = HashMap::with_capacity(chart_names.len());
        for name in chart_names {
            if let Some(def) = self.charts.get(&name) {
                outputs.insert(name, self.compute_chart_output_from_values(def, values));
            }
        }
        outputs
    }

    fn compute_chart_output_from_values(
        &self,
        def: &ChartDefinition,
        values: &HashMap<CellRef, StoredValue>,
    ) -> ChartOutput {
        let range = def.source_range;
        let num_cols = range.end.col - range.start.col + 1;
        let num_rows = range.end.row - range.start.row + 1;

        let number_at = |cell: CellRef| -> f64 {
            values
                .get(&cell)
                .and_then(|stored| stored.value.as_f64())
                .unwrap_or(0.0)
        };

        let text_at = |cell: CellRef| -> String {
            match values.get(&cell).map(|stored| &stored.value) {
                Some(Value::Text(text)) => text.clone(),
                Some(Value::Number(number)) => {
                    if number.fract() == 0.0 {
                        format!("{number:.0}")
                    } else {
                        format!("{number}")
                    }
                }
                Some(Value::Bool(true)) => "TRUE".to_string(),
                Some(Value::Bool(false)) => "FALSE".to_string(),
                Some(Value::Error(err)) => err.excel_tag().to_string(),
                Some(Value::Blank) | None => String::new(),
            }
        };

        if num_rows == 1 {
            let labels: Vec<String> = if range.start.row > 1 {
                (range.start.col..=range.end.col)
                    .map(|col| {
                        text_at(CellRef {
                            col,
                            row: range.start.row - 1,
                        })
                    })
                    .collect()
            } else {
                (range.start.col..=range.end.col)
                    .map(col_index_to_label)
                    .collect()
            };
            let values_row: Vec<f64> = (range.start.col..=range.end.col)
                .map(|col| {
                    number_at(CellRef {
                        col,
                        row: range.start.row,
                    })
                })
                .collect();
            return ChartOutput {
                labels,
                series: vec![ChartSeriesOutput {
                    name: range.start.row.to_string(),
                    values: values_row,
                }],
            };
        }

        if num_cols == 1 {
            let labels: Vec<String> = if range.start.col > 1 {
                (range.start.row..=range.end.row)
                    .map(|row| {
                        text_at(CellRef {
                            col: range.start.col - 1,
                            row,
                        })
                    })
                    .collect()
            } else {
                (range.start.row..=range.end.row)
                    .map(|row| row.to_string())
                    .collect()
            };
            let values_col: Vec<f64> = (range.start.row..=range.end.row)
                .map(|row| {
                    number_at(CellRef {
                        col: range.start.col,
                        row,
                    })
                })
                .collect();
            return ChartOutput {
                labels,
                series: vec![ChartSeriesOutput {
                    name: col_index_to_label(range.start.col),
                    values: values_col,
                }],
            };
        }

        let label_start_col = range.start.col + 1;
        let labels: Vec<String> = (label_start_col..=range.end.col)
            .map(|col| {
                text_at(CellRef {
                    col,
                    row: range.start.row,
                })
            })
            .collect();
        let series: Vec<ChartSeriesOutput> = (range.start.row + 1..=range.end.row)
            .map(|row| {
                let raw_name = text_at(CellRef {
                    col: range.start.col,
                    row,
                });
                let name = if raw_name.is_empty() {
                    row.to_string()
                } else {
                    raw_name
                };
                let values_row: Vec<f64> = (label_start_col..=range.end.col)
                    .map(|col| number_at(CellRef { col, row }))
                    .collect();
                ChartSeriesOutput {
                    name,
                    values: values_row,
                }
            })
            .collect();
        ChartOutput { labels, series }
    }

    /// Remove this cell's entries from the reverse dependency map.
    /// Called before removing or replacing a formula.
    fn remove_reverse_deps_for(&mut self, cell: CellRef) {
        if let Some(CellEntry::Formula(fe)) = self.cells.get(&cell) {
            let deps = crate::deps::dependencies_for_expr(&fe.expr);
            for dep in deps {
                if let Some(set) = self.reverse_deps.get_mut(&dep) {
                    set.remove(&cell);
                    if set.is_empty() {
                        self.reverse_deps.remove(&dep);
                    }
                }
            }
        }
    }

    /// Rebuild the reverse dependency map from scratch.
    fn rebuild_reverse_deps(&mut self) {
        self.reverse_deps.clear();
        for (cell, entry) in &self.cells {
            if let CellEntry::Formula(fe) = entry {
                let deps = crate::deps::dependencies_for_expr(&fe.expr);
                for dep in deps {
                    self.reverse_deps.entry(dep).or_default().insert(*cell);
                }
            }
        }
    }

    /// Compute the transitive closure of dirty cells through reverse dependencies.
    /// Returns the set of all formula cells that need re-evaluation.
    fn compute_dirty_closure(&self) -> HashSet<CellRef> {
        let mut dirty = HashSet::new();
        let mut stack: Vec<CellRef> = Vec::new();

        for source in self.dirty_cells.iter().copied() {
            if matches!(self.cells.get(&source), Some(CellEntry::Formula(_))) {
                dirty.insert(source);
            }
            stack.push(source);
        }

        while let Some(cell) = stack.pop() {
            if let Some(dependents) = self.reverse_deps.get(&cell) {
                for dep in dependents {
                    if dirty.insert(*dep) {
                        stack.push(*dep);
                    }
                }
            }
        }
        dirty
    }

    /// Returns the number of formula cells that were evaluated in the last
    /// recalculation. Useful for testing incremental recalc effectiveness.
    pub fn last_eval_count(&self) -> usize {
        self.last_eval_count
    }

    // -----------------------------------------------------------------------
    // User-defined functions (UDFs)
    // -----------------------------------------------------------------------

    /// Register a user-defined function. The `name` is matched case-insensitively
    /// (stored in uppercase). If a UDF with the same name already exists, it is replaced.
    pub fn register_udf(&mut self, name: &str, handler: Box<dyn UdfHandler>) {
        self.udfs.insert(name.to_ascii_uppercase(), handler);
    }

    /// Unregister a previously registered UDF. Returns `true` if the UDF existed.
    pub fn unregister_udf(&mut self, name: &str) -> bool {
        self.udfs.remove(&name.to_ascii_uppercase()).is_some()
    }

    fn maybe_recalculate(&mut self) -> Result<(), EngineError> {
        match self.mode {
            RecalcMode::Automatic => self.recalculate(),
            RecalcMode::Manual => Ok(()),
        }
    }

    fn mark_presentation_change(&mut self) {
        self.committed_epoch += 1;
        self.stabilized_epoch = self.committed_epoch;
        for stored in self.values.values_mut() {
            stored.value_epoch = self.committed_epoch;
        }
        for stored in self.name_values.values_mut() {
            stored.value_epoch = self.committed_epoch;
        }
    }

    fn ensure_in_bounds(&self, cell: CellRef) -> Result<(), EngineError> {
        if cell.col == 0 || cell.col > self.bounds.max_columns {
            return Err(EngineError::OutOfBounds(cell));
        }
        if cell.row == 0 || cell.row > self.bounds.max_rows {
            return Err(EngineError::OutOfBounds(cell));
        }
        Ok(())
    }

    fn normalize_name(&self, name: &str) -> Result<String, EngineError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(EngineError::Name("name cannot be empty".to_string()));
        }
        let upper = trimmed.to_ascii_uppercase();
        let mut chars = upper.chars();
        let Some(first) = chars.next() else {
            return Err(EngineError::Name("name cannot be empty".to_string()));
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(EngineError::Name(
                "name must start with a letter or '_'".to_string(),
            ));
        }
        if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
            return Err(EngineError::Name(
                "name may only contain letters, digits, or '_'".to_string(),
            ));
        }
        if upper == "TRUE" || upper == "FALSE" {
            return Err(EngineError::Name(
                "name cannot be TRUE or FALSE".to_string(),
            ));
        }
        if is_cell_reference_token(&upper) {
            return Err(EngineError::Name(format!(
                "name '{upper}' conflicts with a cell reference"
            )));
        }
        if SUPPORTED_FUNCTIONS.contains(&upper.as_str()) {
            return Err(EngineError::Name(format!(
                "name '{upper}' conflicts with a built-in function"
            )));
        }
        Ok(upper)
    }

    fn apply_spills_overlay_inline(
        &self,
        order: &[CellRef],
        runtime_values: &HashMap<CellRef, RuntimeValue>,
        values: &mut HashMap<CellRef, StoredValue>,
        spill_owners: &mut HashMap<CellRef, CellRef>,
        spill_ranges: &mut HashMap<CellRef, CellRange>,
    ) {
        for cell in order {
            let Some(runtime) = runtime_values.get(cell) else {
                continue;
            };
            let Some(array) = runtime.as_array() else {
                continue;
            };
            if !array.is_spill() {
                continue;
            }

            let spill_range = match self.spill_range_for_array(*cell, array.rows(), array.cols()) {
                Ok(range) => range,
                Err(err) => {
                    values.insert(
                        *cell,
                        StoredValue {
                            value: Value::Error(err),
                            value_epoch: self.committed_epoch,
                        },
                    );
                    continue;
                }
            };

            let mut blocked_by_input: Option<CellRef> = None;
            let mut blocked_by_spill: Option<CellRef> = None;
            for target in spill_range.iter() {
                if target != *cell && self.cells.contains_key(&target) {
                    blocked_by_input = Some(target);
                    break;
                }
                if target != *cell && spill_owners.contains_key(&target) {
                    blocked_by_spill = Some(target);
                    break;
                }
            }

            if let Some(blocked) = blocked_by_input {
                values.insert(
                    *cell,
                    StoredValue {
                        value: Value::Error(CellError::Spill(format!(
                            "spill blocked by existing cell {blocked}"
                        ))),
                        value_epoch: self.committed_epoch,
                    },
                );
                continue;
            }
            if let Some(blocked) = blocked_by_spill {
                values.insert(
                    *cell,
                    StoredValue {
                        value: Value::Error(CellError::Spill(format!(
                            "spill blocked by another spilled range at {blocked}"
                        ))),
                        value_epoch: self.committed_epoch,
                    },
                );
                continue;
            }

            self.write_spill_values(
                *cell,
                array,
                spill_range,
                values,
                spill_owners,
                spill_ranges,
            );
        }
    }

    fn apply_spills_overlay_planner(
        &self,
        order: &[CellRef],
        runtime_values: &HashMap<CellRef, RuntimeValue>,
        values: &mut HashMap<CellRef, StoredValue>,
        spill_owners: &mut HashMap<CellRef, CellRef>,
        spill_ranges: &mut HashMap<CellRef, CellRange>,
    ) {
        let mut planner = SpillOverlayPlanner::with_inputs(self.cells.keys().copied());
        for cell in order {
            let Some(runtime) = runtime_values.get(cell) else {
                continue;
            };
            let Some(array) = runtime.as_array() else {
                continue;
            };
            if !array.is_spill() {
                continue;
            }

            let plan = match planner.plan_spill(*cell, array.rows(), array.cols(), self.bounds) {
                Ok(plan) => plan,
                Err(err) => {
                    values.insert(
                        *cell,
                        StoredValue {
                            value: Value::Error(Self::spill_overlay_error_to_cell_error(err)),
                            value_epoch: self.committed_epoch,
                        },
                    );
                    continue;
                }
            };

            self.write_spill_values(*cell, array, plan.range, values, spill_owners, spill_ranges);
        }
    }

    fn apply_spills_rewrite_materialize(
        &self,
        order: &[CellRef],
        runtime_values: &HashMap<CellRef, RuntimeValue>,
        values: &mut HashMap<CellRef, StoredValue>,
        spill_owners: &mut HashMap<CellRef, CellRef>,
        spill_ranges: &mut HashMap<CellRef, CellRange>,
    ) {
        for cell in order {
            let Some(runtime) = runtime_values.get(cell) else {
                continue;
            };
            let Some(array) = runtime.as_array() else {
                continue;
            };
            if !array.is_spill() {
                continue;
            }

            let values_vec: Vec<Value> = array.iter().cloned().collect();
            let materialized = match materialize_array_values(
                *cell,
                array.rows(),
                array.cols(),
                &values_vec,
                self.bounds,
            ) {
                Ok(cells) => cells,
                Err(err) => {
                    values.insert(
                        *cell,
                        StoredValue {
                            value: Value::Error(Self::rewrite_error_to_cell_error(err)),
                            value_epoch: self.committed_epoch,
                        },
                    );
                    continue;
                }
            };

            let mut blocked_by_input: Option<CellRef> = None;
            let mut blocked_by_spill: Option<CellRef> = None;
            for materialized_cell in &materialized {
                let target = materialized_cell.target;
                if target != *cell && self.cells.contains_key(&target) {
                    blocked_by_input = Some(target);
                    break;
                }
                if target != *cell && spill_owners.contains_key(&target) {
                    blocked_by_spill = Some(target);
                    break;
                }
            }

            if let Some(blocked) = blocked_by_input {
                values.insert(
                    *cell,
                    StoredValue {
                        value: Value::Error(CellError::Spill(format!(
                            "spill blocked by existing cell {blocked}"
                        ))),
                        value_epoch: self.committed_epoch,
                    },
                );
                continue;
            }
            if let Some(blocked) = blocked_by_spill {
                values.insert(
                    *cell,
                    StoredValue {
                        value: Value::Error(CellError::Spill(format!(
                            "spill blocked by another spilled range at {blocked}"
                        ))),
                        value_epoch: self.committed_epoch,
                    },
                );
                continue;
            }

            let spill_range = self
                .spill_range_for_array(*cell, array.rows(), array.cols())
                .expect("materialized cells already validated bounds");
            spill_ranges.insert(*cell, spill_range.clone());
            for materialized_cell in materialized {
                let target = materialized_cell.target;
                values.insert(
                    target,
                    StoredValue {
                        value: materialized_cell.value,
                        value_epoch: self.committed_epoch,
                    },
                );
                if target != *cell {
                    spill_owners.insert(target, *cell);
                }
            }
        }
    }

    fn spill_range_for_array(
        &self,
        anchor: CellRef,
        rows: usize,
        cols: usize,
    ) -> Result<CellRange, CellError> {
        let end_col = anchor.col as usize + cols - 1;
        let end_row = anchor.row as usize + rows - 1;
        if end_col > self.bounds.max_columns as usize || end_row > self.bounds.max_rows as usize {
            return Err(CellError::Spill(
                "spill range exceeds sheet bounds".to_string(),
            ));
        }
        let end = CellRef {
            col: end_col as u16,
            row: end_row as u16,
        };
        Ok(CellRange::new(anchor, end))
    }

    fn write_spill_values(
        &self,
        anchor: CellRef,
        array: &crate::eval::ArrayValue,
        spill_range: CellRange,
        values: &mut HashMap<CellRef, StoredValue>,
        spill_owners: &mut HashMap<CellRef, CellRef>,
        spill_ranges: &mut HashMap<CellRef, CellRange>,
    ) {
        spill_ranges.insert(anchor, spill_range.clone());
        for target in spill_range.iter() {
            let row = (target.row - anchor.row) as usize;
            let col = (target.col - anchor.col) as usize;
            let value = array.value_at(row, col);
            values.insert(
                target,
                StoredValue {
                    value,
                    value_epoch: self.committed_epoch,
                },
            );
            if target != anchor {
                spill_owners.insert(target, anchor);
            }
        }
    }

    fn spill_overlay_error_to_cell_error(err: SpillOverlayError) -> CellError {
        match err {
            SpillOverlayError::OutOfBounds(_) => {
                CellError::Spill("spill range exceeds sheet bounds".to_string())
            }
            SpillOverlayError::BlockedByInput(cell) => {
                CellError::Spill(format!("spill blocked by existing cell {cell}"))
            }
            SpillOverlayError::BlockedBySpill(cell) => {
                CellError::Spill(format!("spill blocked by another spilled range at {cell}"))
            }
        }
    }

    fn rewrite_error_to_cell_error(err: RewriteError) -> CellError {
        match err {
            RewriteError::OutOfBounds(_) => {
                CellError::Spill("spill range exceeds sheet bounds".to_string())
            }
            RewriteError::ShapeMismatch => {
                CellError::Spill("internal rewrite shape mismatch".to_string())
            }
        }
    }
}

fn excel_now_timestamp() -> f64 {
    match std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64() / 86400.0 + 25569.0,
        Err(_) => 0.0,
    }
}

fn builtin_function_volatility(name: &str) -> Option<Volatility> {
    match name {
        "NOW" | "RAND" | "RANDARRAY" => Some(Volatility::Volatile),
        "STREAM" => Some(Volatility::ExternallyInvalidated),
        _ => None,
    }
}

fn expr_contains_volatility(
    expr: &Expr,
    udfs: &HashMap<String, Box<dyn UdfHandler>>,
    target: Volatility,
) -> bool {
    match expr {
        Expr::FunctionCall { name, args } => {
            if builtin_function_volatility(name) == Some(target) {
                return true;
            }
            if udfs
                .get(name)
                .map(|handler| handler.volatility() == target)
                .unwrap_or(false)
            {
                return true;
            }
            args.iter()
                .any(|arg| expr_contains_volatility(arg, udfs, target))
        }
        Expr::Unary { expr, .. } => expr_contains_volatility(expr, udfs, target),
        Expr::Binary { left, right, .. } => {
            expr_contains_volatility(left, udfs, target)
                || expr_contains_volatility(right, udfs, target)
        }
        Expr::Invoke { callee, args } => {
            expr_contains_volatility(callee, udfs, target)
                || args
                    .iter()
                    .any(|arg| expr_contains_volatility(arg, udfs, target))
        }
        Expr::Number(_)
        | Expr::Text(_)
        | Expr::Bool(_)
        | Expr::Cell(_, _)
        | Expr::Name(_)
        | Expr::SpillRef(_)
        | Expr::Range(_, _, _) => false,
    }
}

fn expr_calls_udf(expr: &Expr, udf_upper: &str) -> bool {
    match expr {
        Expr::FunctionCall { name, args } => {
            if name == udf_upper {
                return true;
            }
            args.iter().any(|arg| expr_calls_udf(arg, udf_upper))
        }
        Expr::Unary { expr, .. } => expr_calls_udf(expr, udf_upper),
        Expr::Binary { left, right, .. } => {
            expr_calls_udf(left, udf_upper) || expr_calls_udf(right, udf_upper)
        }
        Expr::Invoke { callee, args } => {
            expr_calls_udf(callee, udf_upper)
                || args.iter().any(|arg| expr_calls_udf(arg, udf_upper))
        }
        Expr::Number(_)
        | Expr::Text(_)
        | Expr::Bool(_)
        | Expr::Cell(_, _)
        | Expr::Name(_)
        | Expr::SpillRef(_)
        | Expr::Range(_, _, _) => false,
    }
}

/// Shift a cell's coordinate for a structural mutation. Used to relocate
/// cell entries (not formula references — that's handled by `rewrite_expr`).
/// For insert: cells at or after `at` shift by +1.
/// For delete: cells at `at` are discarded (None), cells after shift by -1.
/// Returns None if the cell should be discarded or falls out of bounds.
fn shift_cell_ref(
    cell: CellRef,
    op: StructuralOp,
    is_row_op: bool,
    is_insert: bool,
    bounds: SheetBounds,
) -> Option<CellRef> {
    let at = match op {
        StructuralOp::InsertRow { at }
        | StructuralOp::DeleteRow { at }
        | StructuralOp::InsertCol { at }
        | StructuralOp::DeleteCol { at } => at,
    };

    let coord = if is_row_op { cell.row } else { cell.col };
    let max = if is_row_op {
        bounds.max_rows
    } else {
        bounds.max_columns
    };

    let new_coord = if is_insert {
        if coord >= at {
            let shifted = coord + 1;
            if shifted > max {
                return None; // pushed out of bounds
            }
            shifted
        } else {
            coord
        }
    } else {
        // delete
        if coord == at {
            return None; // in the deleted row/col
        } else if coord > at {
            coord - 1
        } else {
            coord
        }
    };

    if is_row_op {
        Some(CellRef {
            col: cell.col,
            row: new_coord,
        })
    } else {
        Some(CellRef {
            col: new_coord,
            row: cell.row,
        })
    }
}

#[derive(Debug, Clone)]
pub enum EngineError {
    Address(AddressError),
    Parse(ParseError),
    Dependency(DependencyError),
    Name(String),
    OutOfBounds(CellRef),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(err) => write!(f, "{err}"),
            Self::Parse(err) => write!(f, "{err}"),
            Self::Dependency(err) => write!(f, "{err}"),
            Self::Name(message) => write!(f, "invalid name: {message}"),
            Self::OutOfBounds(cell) => write!(f, "cell {cell} is out of engine bounds"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<AddressError> for EngineError {
    fn from(value: AddressError) -> Self {
        Self::Address(value)
    }
}

impl From<ParseError> for EngineError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<DependencyError> for EngineError {
    fn from(value: DependencyError) -> Self {
        Self::Dependency(value)
    }
}
