use std::collections::HashMap;
use std::fmt;

use crate::address::{
    AddressError, CellRange, CellRef, DEFAULT_SHEET_BOUNDS, SheetBounds, parse_cell_ref,
};
use crate::ast::Expr;
use crate::deps::{CalcTree, DependencyError, build_calc_tree};
use crate::eval::{CellError, EvalContext, RuntimeValue, Value};
use crate::experiments::spill_overlay::{SpillOverlayError, SpillOverlayPlanner};
use crate::experiments::spill_rewrite::{RewriteError, materialize_array_values};
use crate::parser::{ParseError, parse_formula};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalcMode {
    Automatic,
    Manual,
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
pub struct Engine {
    bounds: SheetBounds,
    mode: RecalcMode,
    committed_epoch: u64,
    stabilized_epoch: u64,
    cells: HashMap<CellRef, CellEntry>,
    values: HashMap<CellRef, StoredValue>,
    spill_owners: HashMap<CellRef, CellRef>,
    spill_ranges: HashMap<CellRef, CellRange>,
    calc_tree: Option<CalcTree>,
    recalc_serial: u64,
    dynamic_array_strategy: DynamicArrayStrategy,
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
            values: HashMap::new(),
            spill_owners: HashMap::new(),
            spill_ranges: HashMap::new(),
            calc_tree: None,
            recalc_serial: 0,
            dynamic_array_strategy: DynamicArrayStrategy::OverlayInline,
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

    pub fn clear(&mut self) {
        self.committed_epoch += 1;
        self.cells.clear();
        self.values.clear();
        self.spill_owners.clear();
        self.spill_ranges.clear();
        self.calc_tree = None;
        if self.mode == RecalcMode::Automatic {
            self.stabilized_epoch = self.committed_epoch;
        }
    }

    pub fn committed_epoch(&self) -> u64 {
        self.committed_epoch
    }

    pub fn stabilized_epoch(&self) -> u64 {
        self.stabilized_epoch
    }

    pub fn set_number(&mut self, cell: CellRef, number: f64) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        self.cells.insert(cell, CellEntry::Number(number));
        self.committed_epoch += 1;
        self.values.insert(
            cell,
            StoredValue {
                value: Value::Number(number),
                value_epoch: self.committed_epoch,
            },
        );
        self.maybe_recalculate()
    }

    pub fn set_text(&mut self, cell: CellRef, text: impl Into<String>) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let text = text.into();
        self.cells.insert(cell, CellEntry::Text(text.clone()));
        self.committed_epoch += 1;
        self.values.insert(
            cell,
            StoredValue {
                value: Value::Text(text),
                value_epoch: self.committed_epoch,
            },
        );
        self.maybe_recalculate()
    }

    pub fn set_formula(&mut self, cell: CellRef, formula: &str) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        let expr = parse_formula(formula, self.bounds)?;
        self.cells.insert(
            cell,
            CellEntry::Formula(FormulaEntry {
                source: formula.to_string(),
                expr,
            }),
        );
        self.committed_epoch += 1;
        self.maybe_recalculate()
    }

    pub fn clear_cell(&mut self, cell: CellRef) -> Result<(), EngineError> {
        self.ensure_in_bounds(cell)?;
        self.cells.remove(&cell);
        self.values.remove(&cell);
        self.committed_epoch += 1;
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

    pub fn recalculate(&mut self) -> Result<(), EngineError> {
        let mut formulas: HashMap<CellRef, Expr> = HashMap::new();
        let mut literals: HashMap<CellRef, f64> = HashMap::new();
        let mut text_literals: HashMap<CellRef, String> = HashMap::new();

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

        let tree = build_calc_tree(&formulas)?;
        self.recalc_serial = self.recalc_serial.wrapping_add(1);
        let mut evaluator = EvalContext::new(
            &formulas,
            &literals,
            &text_literals,
            self.bounds,
            self.recalc_serial,
        );
        let mut new_values: HashMap<CellRef, StoredValue> = HashMap::new();
        let mut runtime_values: HashMap<CellRef, RuntimeValue> = HashMap::new();

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

        for cell in &tree.order {
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
        }

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

        self.values = new_values;
        self.spill_owners = spill_owners;
        self.spill_ranges = spill_ranges;
        self.stabilized_epoch = self.committed_epoch;
        self.calc_tree = Some(tree);
        Ok(())
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

    fn maybe_recalculate(&mut self) -> Result<(), EngineError> {
        match self.mode {
            RecalcMode::Automatic => self.recalculate(),
            RecalcMode::Manual => Ok(()),
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

#[derive(Debug, Clone)]
pub enum EngineError {
    Address(AddressError),
    Parse(ParseError),
    Dependency(DependencyError),
    OutOfBounds(CellRef),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(err) => write!(f, "{err}"),
            Self::Parse(err) => write!(f, "{err}"),
            Self::Dependency(err) => write!(f, "{err}"),
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
