use std::collections::HashMap;
use std::fmt;

use crate::address::{AddressError, CellRef, DEFAULT_SHEET_BOUNDS, SheetBounds, parse_cell_ref};
use crate::ast::Expr;
use crate::deps::{CalcTree, DependencyError, build_calc_tree};
use crate::eval::{EvalContext, Value};
use crate::parser::{ParseError, parse_formula};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecalcMode {
    Automatic,
    Manual,
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
    calc_tree: Option<CalcTree>,
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
            calc_tree: None,
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

    pub fn clear_cell_a1(&mut self, cell_ref: &str) -> Result<(), EngineError> {
        let cell = parse_cell_ref(cell_ref, self.bounds)?;
        self.clear_cell(cell)
    }

    pub fn recalculate(&mut self) -> Result<(), EngineError> {
        let mut formulas: HashMap<CellRef, Expr> = HashMap::new();
        let mut literals: HashMap<CellRef, f64> = HashMap::new();

        for (cell, entry) in &self.cells {
            match entry {
                CellEntry::Number(n) => {
                    literals.insert(*cell, *n);
                }
                CellEntry::Formula(formula) => {
                    formulas.insert(*cell, formula.expr.clone());
                }
            }
        }

        let tree = build_calc_tree(&formulas)?;
        let mut evaluator = EvalContext::new(&formulas, &literals);
        let mut new_values: HashMap<CellRef, StoredValue> = HashMap::new();

        for (cell, number) in &literals {
            new_values.insert(
                *cell,
                StoredValue {
                    value: Value::Number(*number),
                    value_epoch: self.committed_epoch,
                },
            );
        }

        for cell in &tree.order {
            let value = evaluator.evaluate_cell(*cell);
            new_values.insert(
                *cell,
                StoredValue {
                    value,
                    value_epoch: self.committed_epoch,
                },
            );
        }

        self.values = new_values;
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
            CellEntry::Number(_) => None,
        });
        Ok(source)
    }

    pub fn calc_tree(&self) -> Option<&CalcTree> {
        self.calc_tree.as_ref()
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
