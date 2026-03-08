use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::rc::Rc;

use crate::address::CellRef;
use crate::address::{AddressError, parse_cell_ref};
use crate::address::{CellRange, SheetBounds};
use crate::ast::{BinaryOp, Expr, UnaryOp};
use crate::cell_grid::{CellBitset, CellGrid};
use crate::engine::StoredValue;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Blank,
    Error(CellError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Volatility {
    #[default]
    Standard,
    Volatile,
    ExternallyInvalidated,
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellError {
    DivisionByZero,
    Value(String),
    Name(String),
    UnknownName(String),
    Ref(String),
    Spill(String),
    Cycle(Vec<String>),
    Na,
    Null,
    Num(String),
}

impl CellError {
    /// Returns the Excel-style error tag (e.g. `#DIV/0!`, `#VALUE!`).
    pub fn excel_tag(&self) -> &'static str {
        match self {
            Self::DivisionByZero => "#DIV/0!",
            Self::Value(_) => "#VALUE!",
            Self::Name(_) | Self::UnknownName(_) => "#NAME?",
            Self::Ref(_) => "#REF!",
            Self::Spill(_) => "#SPILL!",
            Self::Cycle(_) => "#REF!",
            Self::Na => "#N/A",
            Self::Null => "#NULL!",
            Self::Num(_) => "#NUM!",
        }
    }

    /// Returns the `ERROR.TYPE` number for this error, matching Excel's convention.
    pub fn error_type_number(&self) -> u8 {
        match self {
            Self::Null => 1,
            Self::DivisionByZero => 2,
            Self::Value(_) => 3,
            Self::Ref(_) | Self::Cycle(_) => 4,
            Self::Name(_) | Self::UnknownName(_) => 5,
            Self::Num(_) => 6,
            Self::Na => 7,
            Self::Spill(_) => 9,
        }
    }

    /// Returns true if this is an `#N/A` error.
    pub fn is_na(&self) -> bool {
        matches!(self, Self::Na)
    }
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::Value(msg) => write!(f, "value error: {msg}"),
            Self::Name(name) => write!(f, "unknown function: {name}"),
            Self::UnknownName(name) => write!(f, "unknown name: {name}"),
            Self::Ref(msg) => write!(f, "reference error: {msg}"),
            Self::Spill(msg) => write!(f, "spill error: {msg}"),
            Self::Cycle(path) => {
                let joined = path.join(" -> ");
                write!(f, "circular reference during evaluation: {joined}")
            }
            Self::Na => write!(f, "value not available"),
            Self::Null => write!(f, "null intersection"),
            Self::Num(msg) => write!(f, "numeric error: {msg}"),
        }
    }
}

impl std::error::Error for CellError {}

pub const SUPPORTED_FUNCTIONS: &[&str] = &[
    "SUM",
    "MIN",
    "MAX",
    "AVERAGE",
    "COUNT",
    "IF",
    "IFERROR",
    "IFNA",
    "AND",
    "OR",
    "NOT",
    "ABS",
    "INT",
    "ROUND",
    "SIGN",
    "SQRT",
    "EXP",
    "LN",
    "LOG10",
    "SIN",
    "COS",
    "TAN",
    "ATN",
    "PI",
    "NPV",
    "PV",
    "FV",
    "PMT",
    "LOOKUP",
    "NA",
    "ERROR",
    "ISERROR",
    "ISNA",
    "ISBLANK",
    "ISTEXT",
    "ISNUMBER",
    "ISLOGICAL",
    "ERROR.TYPE",
    "CONCAT",
    "LEN",
    "SEQUENCE",
    "RANDARRAY",
    "LET",
    "LAMBDA",
    "MAP",
    "INDIRECT",
    "OFFSET",
    "ROW",
    "COLUMN",
    "NOW",
    "RAND",
    "STREAM",
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ArrayValue {
    rows: usize,
    cols: usize,
    values: Vec<Value>,
}

impl ArrayValue {
    pub fn new(rows: usize, cols: usize, values: Vec<Value>) -> Self {
        Self { rows, cols, values }
    }

    pub fn from_scalar(value: Value) -> Self {
        Self {
            rows: 1,
            cols: 1,
            values: vec![value],
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn is_spill(&self) -> bool {
        self.rows > 1 || self.cols > 1
    }

    pub fn top_left(&self) -> Value {
        self.values.first().cloned().unwrap_or(Value::Blank)
    }

    pub fn value_at(&self, row: usize, col: usize) -> Value {
        self.values[row * self.cols + col].clone()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.values.iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RuntimeValue {
    Scalar(Value),
    Array(ArrayValue),
    Lambda(LambdaValue),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LambdaValue {
    params: Vec<String>,
    body: Expr,
    captured: FxHashMap<String, RuntimeValue>,
}

impl RuntimeValue {
    pub fn scalar(value: Value) -> Self {
        Self::Scalar(value)
    }

    pub fn to_scalar(&self) -> Value {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::Array(array) => array.top_left(),
            Self::Lambda(_) => Value::Error(CellError::Value(
                "lambda cannot be used as a scalar value".to_string(),
            )),
        }
    }

    pub fn as_array(&self) -> Option<&ArrayValue> {
        match self {
            Self::Array(array) => Some(array),
            Self::Scalar(_) | Self::Lambda(_) => None,
        }
    }

    fn to_array_value(&self) -> ArrayValue {
        match self {
            Self::Scalar(value) => ArrayValue::from_scalar(value.clone()),
            Self::Array(array) => array.clone(),
            Self::Lambda(_) => ArrayValue::from_scalar(Value::Error(CellError::Value(
                "lambda cannot be used as an array value".to_string(),
            ))),
        }
    }

    fn flatten_values(&self) -> Vec<Value> {
        match self {
            Self::Scalar(value) => vec![value.clone()],
            Self::Array(array) => array.iter().cloned().collect(),
            Self::Lambda(_) => vec![Value::Error(CellError::Value(
                "lambda cannot be expanded as argument values".to_string(),
            ))],
        }
    }
}

/// Trait for user-defined functions (UDFs). Implementors provide a `call`
/// method that receives evaluated argument values and returns a result value.
pub trait UdfHandler: std::fmt::Debug {
    /// Evaluate the UDF with the given arguments.
    fn call(&self, args: &[Value]) -> Value;

    /// Volatility class used by host-driven invalidation APIs.
    fn volatility(&self) -> Volatility {
        Volatility::Standard
    }
}

/// Wrapper that turns any `Fn(&[Value]) -> Value` into a [`UdfHandler`].
pub struct FnUdf<F>(pub F);

impl<F> std::fmt::Debug for FnUdf<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FnUdf(<closure>)")
    }
}

impl<F: Fn(&[Value]) -> Value> UdfHandler for FnUdf<F> {
    fn call(&self, args: &[Value]) -> Value {
        (self.0)(args)
    }
}

/// Wrapper that pairs a closure UDF with an explicit volatility class.
pub struct FnUdfWithVolatility<F> {
    pub callback: F,
    pub volatility: Volatility,
}

impl<F> FnUdfWithVolatility<F> {
    pub fn new(callback: F, volatility: Volatility) -> Self {
        Self {
            callback,
            volatility,
        }
    }
}

impl<F> std::fmt::Debug for FnUdfWithVolatility<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FnUdfWithVolatility(<closure>)")
    }
}

impl<F: Fn(&[Value]) -> Value> UdfHandler for FnUdfWithVolatility<F> {
    fn call(&self, args: &[Value]) -> Value {
        (self.callback)(args)
    }

    fn volatility(&self) -> Volatility {
        self.volatility
    }
}

pub struct EvalContext<'a> {
    formulas: &'a FxHashMap<CellRef, Rc<Expr>>,
    literals: &'a FxHashMap<CellRef, f64>,
    text_literals: &'a FxHashMap<CellRef, String>,
    name_formulas: &'a FxHashMap<String, Rc<Expr>>,
    name_literals: &'a FxHashMap<String, f64>,
    name_text_literals: &'a FxHashMap<String, String>,
    bounds: SheetBounds,
    cache: CellGrid<RuntimeValue>,
    name_cache: FxHashMap<String, RuntimeValue>,
    /// Committed cell values for lazy cache lookup during incremental recalc.
    /// When set, cache misses on formula cells fall through to this map
    /// instead of triggering a full recursive evaluation.
    committed_cell_values: Option<&'a CellGrid<StoredValue>>,
    /// Committed name values for lazy cache lookup during incremental recalc.
    committed_name_values: Option<&'a FxHashMap<String, StoredValue>>,
    /// Cells evicted from the committed-value fallback (dirty cells that
    /// must be re-evaluated rather than read from committed state).
    committed_evicted_cells: CellBitset,
    /// Names evicted from the committed-value fallback.
    committed_evicted_names: FxHashSet<String>,
    stack: Vec<EvalStackNode>,
    /// O(1) lookup set for cell cycle detection — mirrors Cell entries in stack.
    cell_stack_set: CellBitset,
    local_scopes: Vec<FxHashMap<String, RuntimeValue>>,
    recalc_serial: u64,
    random_counter: u64,
    now_timestamp: f64,
    stream_counters: &'a FxHashMap<CellRef, u64>,
    stream_registrations: FxHashMap<CellRef, StreamRegistration>,
    /// Previous iteration's cached values for cells in the currently
    /// iterating SCC. When a cycle is detected during evaluation and the
    /// cell is in this set, the previous value is returned instead of a
    /// cycle error.
    iterating_prev: FxHashMap<CellRef, RuntimeValue>,
    /// True once a circular-reference path is observed during this evaluation.
    cycle_detected: bool,
    /// Guardrails to avoid process-level stack overflow on pathological nesting.
    cell_eval_depth: usize,
    expr_eval_depth: usize,
    max_eval_depth: usize,
    /// Registered user-defined functions.
    udfs: &'a FxHashMap<String, Box<dyn UdfHandler>>,
}

#[derive(Debug, Clone)]
pub(crate) struct StreamRegistration {
    pub period_secs: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvalStackNode {
    Cell(CellRef),
    Name(String),
}

impl<'a> EvalContext<'a> {
    pub fn new(
        formulas: &'a FxHashMap<CellRef, Rc<Expr>>,
        literals: &'a FxHashMap<CellRef, f64>,
        text_literals: &'a FxHashMap<CellRef, String>,
        name_formulas: &'a FxHashMap<String, Rc<Expr>>,
        name_literals: &'a FxHashMap<String, f64>,
        name_text_literals: &'a FxHashMap<String, String>,
        bounds: SheetBounds,
        recalc_serial: u64,
        now_timestamp: f64,
        stream_counters: &'a FxHashMap<CellRef, u64>,
        udfs: &'a FxHashMap<String, Box<dyn UdfHandler>>,
    ) -> Self {
        let name_count = name_formulas.len() + name_literals.len() + name_text_literals.len();
        Self {
            formulas,
            literals,
            text_literals,
            name_formulas,
            name_literals,
            name_text_literals,
            bounds,
            cache: CellGrid::new(bounds.max_columns, bounds.max_rows),
            name_cache: FxHashMap::with_capacity_and_hasher(name_count, Default::default()),
            committed_cell_values: None,
            committed_name_values: None,
            committed_evicted_cells: CellBitset::new(bounds.max_columns, bounds.max_rows),
            committed_evicted_names: FxHashSet::default(),
            stack: Vec::new(),
            cell_stack_set: CellBitset::new(bounds.max_columns, bounds.max_rows),
            local_scopes: Vec::new(),
            recalc_serial,
            random_counter: 0,
            now_timestamp,
            stream_counters,
            stream_registrations: FxHashMap::default(),
            iterating_prev: FxHashMap::default(),
            cycle_detected: false,
            cell_eval_depth: 0,
            expr_eval_depth: 0,
            max_eval_depth: 4096,
            udfs,
        }
    }

    pub(crate) fn take_stream_registrations(&mut self) -> FxHashMap<CellRef, StreamRegistration> {
        std::mem::take(&mut self.stream_registrations)
    }

    pub(crate) fn take_cycle_detected(&mut self) -> bool {
        let detected = self.cycle_detected;
        self.cycle_detected = false;
        detected
    }

    /// Sets committed cell values for lazy cache lookup during incremental
    /// recalc. Cache misses on formula cells will fall through to this map,
    /// avoiding the need to pre-seed all ~8,800 formula values.
    pub(crate) fn set_committed_cell_values(&mut self, values: &'a CellGrid<StoredValue>) {
        self.committed_cell_values = Some(values);
    }

    /// Sets committed name values for lazy cache lookup during incremental
    /// recalc.
    pub(crate) fn set_committed_name_values(&mut self, values: &'a FxHashMap<String, StoredValue>) {
        self.committed_name_values = Some(values);
    }

    /// Removes a single cached cell value, forcing re-evaluation on next read.
    /// Also evicts from the committed-value fallback so stale values aren't used.
    pub(crate) fn evict_cell_cache(&mut self, cell: CellRef) {
        self.cache.remove(&cell);
        self.committed_evicted_cells.insert(cell);
    }

    /// Removes a single cached name value, forcing re-evaluation on next read.
    /// Also evicts from the committed-value fallback.
    pub(crate) fn evict_name_cache(&mut self, name: &str) {
        let upper = name.to_ascii_uppercase();
        self.name_cache.remove(&upper);
        self.committed_evicted_names.insert(upper);
    }

    /// Prepares for iterative evaluation of a cyclic SCC. Seeds the cache with
    /// initial values (0.0) for all SCC members and marks them as iterating so
    /// that cycle detection returns previous values instead of errors.
    pub(crate) fn begin_iteration(&mut self, cells: &[CellRef]) {
        let initial = RuntimeValue::scalar(Value::Number(0.0));
        for cell in cells {
            self.cache.insert(*cell, initial.clone());
            self.iterating_prev.insert(*cell, initial.clone());
        }
    }

    /// Like `begin_iteration`, but uses caller-provided seed values for each
    /// cyclic cell. Missing seeds default to `0.0`.
    pub(crate) fn begin_iteration_seeded(
        &mut self,
        cells: &[CellRef],
        seeds: &FxHashMap<CellRef, RuntimeValue>,
    ) {
        let default_value = RuntimeValue::scalar(Value::Number(0.0));
        for cell in cells {
            let seed = seeds
                .get(cell)
                .cloned()
                .unwrap_or_else(|| default_value.clone());
            self.cache.insert(*cell, seed.clone());
            self.iterating_prev.insert(*cell, seed);
        }
    }

    /// Moves current cached values to `iterating_prev` and clears the cache
    /// for the given SCC cells, preparing for the next iteration pass.
    pub(crate) fn advance_iteration(&mut self, cells: &[CellRef]) {
        for cell in cells {
            if let Some(current) = self.cache.remove(cell) {
                self.iterating_prev.insert(*cell, current);
            }
        }
    }

    /// Finishes iterative evaluation. Clears the iterating_prev state.
    pub(crate) fn end_iteration(&mut self) {
        self.iterating_prev.clear();
    }

    pub(crate) fn evaluate_cell_runtime(&mut self, cell: CellRef) -> RuntimeValue {
        self.cell_eval_depth += 1;
        if self.cell_eval_depth > self.max_eval_depth {
            self.cell_eval_depth -= 1;
            return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
                "cell evaluation nesting exceeded {} frames",
                self.max_eval_depth
            ))));
        }

        if let Some(value) = self.cache.get(&cell) {
            self.cell_eval_depth -= 1;
            return value.clone();
        }
        if self.cell_stack_set.contains(&cell) {
            self.cycle_detected = true;
            // During iterative SCC evaluation, return the previous iteration's
            // value instead of a cycle error.
            if let Some(prev) = self.iterating_prev.get(&cell) {
                self.cell_eval_depth -= 1;
                return prev.clone();
            }
            self.cell_eval_depth -= 1;
            return RuntimeValue::scalar(Value::Number(0.0));
        }

        if let Some(expr) = self.formulas.get(&cell) {
            // During incremental recalc, clean formula cells have committed
            // values that don't need re-evaluation. Check before recursing.
            if let Some(committed) = self.committed_cell_values {
                if !self.committed_evicted_cells.contains(&cell) {
                    if let Some(stored) = committed.get(&cell) {
                        let rv = RuntimeValue::scalar(stored.value.clone());
                        self.cache.insert(cell, rv.clone());
                        self.cell_eval_depth -= 1;
                        return rv;
                    }
                }
            }
            self.stack.push(EvalStackNode::Cell(cell));
            self.cell_stack_set.insert(cell);
            let value = self.evaluate_expr(expr);
            self.stack.pop();
            self.cell_stack_set.remove(&cell);
            self.cache.insert(cell, value.clone());
            self.cell_eval_depth -= 1;
            return value;
        }

        if let Some(number) = self.literals.get(&cell) {
            self.cell_eval_depth -= 1;
            return RuntimeValue::scalar(Value::Number(*number));
        }
        if let Some(text) = self.text_literals.get(&cell) {
            self.cell_eval_depth -= 1;
            return RuntimeValue::scalar(Value::Text(text.clone()));
        }

        if let Some(value) = self.resolve_spilled_cell_value(cell) {
            self.cell_eval_depth -= 1;
            return RuntimeValue::scalar(value);
        }

        self.cell_eval_depth -= 1;
        RuntimeValue::scalar(Value::Blank)
    }

    pub(crate) fn evaluate_name_runtime(&mut self, name: &str) -> RuntimeValue {
        if let Some(local) = self.lookup_local(name) {
            return local;
        }
        let upper = name.to_ascii_uppercase();
        if let Some(value) = self.name_cache.get(&upper) {
            return value.clone();
        }
        if self
            .stack
            .iter()
            .any(|node| *node == EvalStackNode::Name(upper.clone()))
        {
            self.cycle_detected = true;
            return RuntimeValue::scalar(Value::Number(0.0));
        }

        if let Some(expr) = self.name_formulas.get(&upper) {
            // During incremental recalc, check committed name values first.
            if let Some(committed) = self.committed_name_values {
                if !self.committed_evicted_names.contains(&upper) {
                    if let Some(stored) = committed.get(&upper) {
                        let rv = RuntimeValue::scalar(stored.value.clone());
                        self.name_cache.insert(upper, rv.clone());
                        return rv;
                    }
                }
            }
            self.stack.push(EvalStackNode::Name(upper.clone()));
            let value = self.evaluate_expr(expr);
            self.stack.pop();
            self.name_cache.insert(upper, value.clone());
            return value;
        }
        if let Some(number) = self.name_literals.get(&upper) {
            return RuntimeValue::scalar(Value::Number(*number));
        }
        if let Some(text) = self.name_text_literals.get(&upper) {
            return RuntimeValue::scalar(Value::Text(text.clone()));
        }

        RuntimeValue::scalar(Value::Error(CellError::UnknownName(name.to_string())))
    }

    fn lookup_local(&self, name: &str) -> Option<RuntimeValue> {
        let key = name.to_ascii_uppercase();
        for scope in self.local_scopes.iter().rev() {
            if let Some(value) = scope.get(&key) {
                return Some(value.clone());
            }
        }
        None
    }

    fn push_scope(&mut self) {
        self.local_scopes.push(FxHashMap::default());
    }

    fn pop_scope(&mut self) {
        self.local_scopes.pop();
    }

    fn bind_local(&mut self, name: &str, value: RuntimeValue) {
        let key = name.to_ascii_uppercase();
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    fn collect_visible_locals(&self) -> FxHashMap<String, RuntimeValue> {
        let mut out = FxHashMap::default();
        for scope in &self.local_scopes {
            for (name, value) in scope {
                out.insert(name.clone(), value.clone());
            }
        }
        out
    }

    fn current_context_cell(&self) -> Option<CellRef> {
        self.stack.iter().rev().find_map(|node| match node {
            EvalStackNode::Cell(cell) => Some(*cell),
            EvalStackNode::Name(_) => None,
        })
    }

    fn evaluate_expr_with_scope(
        &mut self,
        expr: &Expr,
        scope: FxHashMap<String, RuntimeValue>,
    ) -> RuntimeValue {
        self.local_scopes.push(scope);
        let value = self.evaluate_expr(expr);
        self.local_scopes.pop();
        value
    }

    fn invoke_lambda_runtime(
        &mut self,
        lambda: &LambdaValue,
        args: Vec<RuntimeValue>,
    ) -> RuntimeValue {
        if args.len() != lambda.params.len() {
            return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
                "lambda expects {} argument(s), received {}",
                lambda.params.len(),
                args.len()
            ))));
        }
        let mut scope = lambda.captured.clone();
        for (param, arg) in lambda.params.iter().zip(args.into_iter()) {
            scope.insert(param.to_ascii_uppercase(), arg);
        }
        self.evaluate_expr_with_scope(&lambda.body, scope)
    }

    pub(crate) fn evaluate_expr(&mut self, expr: &Expr) -> RuntimeValue {
        self.expr_eval_depth += 1;
        if self.expr_eval_depth > self.max_eval_depth {
            self.expr_eval_depth -= 1;
            return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
                "expression evaluation nesting exceeded {} frames",
                self.max_eval_depth
            ))));
        }

        let value = match expr {
            Expr::Number(n) => RuntimeValue::scalar(Value::Number(*n)),
            Expr::Text(text) => RuntimeValue::scalar(Value::Text(text.clone())),
            Expr::Bool(b) => RuntimeValue::scalar(Value::Bool(*b)),
            Expr::Cell(cell, _) => {
                RuntimeValue::scalar(self.evaluate_cell_runtime(*cell).to_scalar())
            }
            Expr::Name(name) => self.evaluate_name_runtime(name),
            Expr::SpillRef(cell) => self.evaluate_spill_ref(*cell),
            Expr::Range(_, _, _) => RuntimeValue::scalar(Value::Error(CellError::Value(
                "range cannot be used as a scalar value".to_string(),
            ))),
            Expr::Unary { op, expr } => self.eval_unary(*op, expr),
            Expr::Binary { op, left, right } => {
                let lval = self.evaluate_expr(left);
                if let Value::Error(err) = lval.to_scalar() {
                    RuntimeValue::scalar(Value::Error(err))
                } else {
                    let rval = self.evaluate_expr(right);
                    if let Value::Error(err) = rval.to_scalar() {
                        RuntimeValue::scalar(Value::Error(err))
                    } else {
                        evaluate_binary_runtime(*op, &lval, &rval)
                    }
                }
            }
            Expr::FunctionCall { name, args } => evaluate_function(name, args, self),
            Expr::Invoke { callee, args } => eval_invoke(callee, args, self),
        };

        self.expr_eval_depth -= 1;
        value
    }

    fn eval_unary(&mut self, op: UnaryOp, expr: &Expr) -> RuntimeValue {
        let value = self.evaluate_expr(expr);
        match value {
            RuntimeValue::Scalar(v) => match op {
                UnaryOp::Plus => {
                    RuntimeValue::scalar(coerce_number(&v).map_or_else(Value::Error, Value::Number))
                }
                UnaryOp::Minus => RuntimeValue::scalar(
                    coerce_number(&v).map_or_else(Value::Error, |n| Value::Number(-n)),
                ),
            },
            RuntimeValue::Array(array) => {
                let mut out = Vec::with_capacity(array.rows() * array.cols());
                for value in array.iter() {
                    let element = match op {
                        UnaryOp::Plus => {
                            coerce_number(value).map_or_else(Value::Error, Value::Number)
                        }
                        UnaryOp::Minus => {
                            coerce_number(value).map_or_else(Value::Error, |n| Value::Number(-n))
                        }
                    };
                    out.push(element);
                }
                RuntimeValue::Array(ArrayValue::new(array.rows(), array.cols(), out))
            }
            RuntimeValue::Lambda(_) => RuntimeValue::scalar(Value::Error(CellError::Value(
                "lambda cannot be used with unary operators".to_string(),
            ))),
        }
    }

    fn evaluate_spill_ref(&mut self, cell: CellRef) -> RuntimeValue {
        let value = self.evaluate_cell_runtime(cell);
        match value {
            RuntimeValue::Array(array) if array.is_spill() => RuntimeValue::Array(array),
            RuntimeValue::Array(_) | RuntimeValue::Scalar(Value::Blank) => {
                RuntimeValue::scalar(Value::Error(CellError::Ref(format!(
                    "{cell} does not contain a spilled range"
                ))))
            }
            RuntimeValue::Scalar(Value::Error(err)) => RuntimeValue::scalar(Value::Error(err)),
            RuntimeValue::Scalar(_) => RuntimeValue::scalar(Value::Error(CellError::Ref(format!(
                "{cell} does not contain a spilled range"
            )))),
            RuntimeValue::Lambda(_) => RuntimeValue::scalar(Value::Error(CellError::Ref(format!(
                "{cell} does not contain a spilled range"
            )))),
        }
    }

    fn current_eval_anchor(&self) -> CellRef {
        self.stack
            .last()
            .and_then(|node| match node {
                EvalStackNode::Cell(cell) => Some(*cell),
                EvalStackNode::Name(_) => None,
            })
            .unwrap_or(CellRef { col: 1, row: 1 })
    }

    fn mix_u64(mut x: u64) -> u64 {
        // SplitMix64 finalizer for deterministic bit-mixing.
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    fn unit_from_u64(x: u64) -> f64 {
        (x >> 11) as f64 / ((1u64 << 53) as f64)
    }

    fn next_rand_unit(&mut self) -> f64 {
        self.random_counter = self.random_counter.wrapping_add(1);
        let anchor = self.current_eval_anchor();
        let slot_seed = ((anchor.col as u64) << 16)
            ^ ((anchor.row as u64) << 32)
            ^ self.random_counter.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let base_seed = Self::mix_u64(slot_seed);
        let base = Self::unit_from_u64(base_seed);
        let phase_seed = Self::mix_u64(base_seed ^ 0xd1b5_4a32_d192_ed03);
        let phase = Self::unit_from_u64(phase_seed) * std::f64::consts::TAU;
        let drift = self.recalc_serial as f64 * 0.09;
        let perturbation = (phase + drift).sin() * 0.005;
        (base + perturbation).clamp(0.0, 1.0 - f64::EPSILON)
    }

    fn resolve_spilled_cell_value(&mut self, target: CellRef) -> Option<Value> {
        let mut anchors: Vec<CellRef> = self.formulas.keys().copied().collect();
        anchors.sort();

        for anchor in anchors {
            if anchor == target {
                continue;
            }

            let runtime = self.evaluate_cell_runtime(anchor);
            let Some(array) = runtime.as_array() else {
                continue;
            };
            if !array.is_spill() {
                continue;
            }

            let Some(range) = self.spill_range_if_input_unblocked(anchor, array) else {
                continue;
            };
            if !range_contains(range, target) {
                continue;
            }

            let row = (target.row - anchor.row) as usize;
            let col = (target.col - anchor.col) as usize;
            return Some(array.value_at(row, col));
        }

        None
    }

    fn spill_range_if_input_unblocked(
        &self,
        anchor: CellRef,
        array: &ArrayValue,
    ) -> Option<CellRange> {
        let end_col = anchor.col as usize + array.cols() - 1;
        let end_row = anchor.row as usize + array.rows() - 1;
        if end_col > self.bounds.max_columns as usize || end_row > self.bounds.max_rows as usize {
            return None;
        }

        let end = CellRef {
            col: end_col as u16,
            row: end_row as u16,
        };
        let range = CellRange::new(anchor, end);
        for cell in range.iter() {
            if cell != anchor
                && (self.formulas.contains_key(&cell) || self.literals.contains_key(&cell))
            {
                return None;
            }
        }
        Some(range)
    }
}

fn range_contains(range: CellRange, cell: CellRef) -> bool {
    cell.col >= range.start.col
        && cell.col <= range.end.col
        && cell.row >= range.start.row
        && cell.row <= range.end.row
}

fn evaluate_binary_runtime(op: BinaryOp, lhs: &RuntimeValue, rhs: &RuntimeValue) -> RuntimeValue {
    match (lhs, rhs) {
        (RuntimeValue::Scalar(left), RuntimeValue::Scalar(right)) => {
            RuntimeValue::scalar(evaluate_binary_scalar(op, left, right))
        }
        _ => {
            let left_array = lhs.to_array_value();
            let right_array = rhs.to_array_value();
            let (rows, cols) = match broadcast_shape(
                left_array.rows(),
                left_array.cols(),
                right_array.rows(),
                right_array.cols(),
            ) {
                Ok(shape) => shape,
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            };

            let mut values = Vec::with_capacity(rows * cols);
            for row in 0..rows {
                for col in 0..cols {
                    let left_row = if left_array.rows() == 1 { 0 } else { row };
                    let left_col = if left_array.cols() == 1 { 0 } else { col };
                    let right_row = if right_array.rows() == 1 { 0 } else { row };
                    let right_col = if right_array.cols() == 1 { 0 } else { col };
                    let cell = evaluate_binary_scalar(
                        op,
                        &left_array.value_at(left_row, left_col),
                        &right_array.value_at(right_row, right_col),
                    );
                    values.push(cell);
                }
            }
            RuntimeValue::Array(ArrayValue::new(rows, cols, values))
        }
    }
}

fn broadcast_shape(
    left_rows: usize,
    left_cols: usize,
    right_rows: usize,
    right_cols: usize,
) -> Result<(usize, usize), CellError> {
    if left_rows == right_rows && left_cols == right_cols {
        return Ok((left_rows, left_cols));
    }
    if left_rows == 1 && left_cols == 1 {
        return Ok((right_rows, right_cols));
    }
    if right_rows == 1 && right_cols == 1 {
        return Ok((left_rows, left_cols));
    }
    Err(CellError::Value(
        "array dimensions are incompatible for element-wise operation".to_string(),
    ))
}

fn evaluate_binary_scalar(op: BinaryOp, lhs: &Value, rhs: &Value) -> Value {
    match op {
        BinaryOp::Add => eval_numeric_binary(lhs, rhs, |a, b| Ok(a + b)),
        BinaryOp::Sub => eval_numeric_binary(lhs, rhs, |a, b| Ok(a - b)),
        BinaryOp::Mul => eval_numeric_binary(lhs, rhs, |a, b| Ok(a * b)),
        BinaryOp::Div => eval_numeric_binary(lhs, rhs, |a, b| {
            if b == 0.0 {
                Err(CellError::DivisionByZero)
            } else {
                Ok(a / b)
            }
        }),
        BinaryOp::Pow => eval_numeric_binary(lhs, rhs, |a, b| Ok(a.powf(b))),
        BinaryOp::Concat => Value::Text(format!("{}{}", value_to_text(lhs), value_to_text(rhs))),
        BinaryOp::Eq => compare_values(lhs, rhs, |a, b| a == b),
        BinaryOp::Ne => compare_values(lhs, rhs, |a, b| a != b),
        BinaryOp::Lt => compare_values(lhs, rhs, |a, b| a < b),
        BinaryOp::Le => compare_values(lhs, rhs, |a, b| a <= b),
        BinaryOp::Gt => compare_values(lhs, rhs, |a, b| a > b),
        BinaryOp::Ge => compare_values(lhs, rhs, |a, b| a >= b),
    }
}

fn eval_numeric_binary<F>(lhs: &Value, rhs: &Value, f: F) -> Value
where
    F: FnOnce(f64, f64) -> Result<f64, CellError>,
{
    let a = match coerce_number(lhs) {
        Ok(v) => v,
        Err(err) => return Value::Error(err),
    };
    let b = match coerce_number(rhs) {
        Ok(v) => v,
        Err(err) => return Value::Error(err),
    };
    f(a, b).map_or_else(Value::Error, Value::Number)
}

fn compare_values<F>(lhs: &Value, rhs: &Value, cmp: F) -> Value
where
    F: FnOnce(f64, f64) -> bool,
{
    match (lhs, rhs) {
        (Value::Bool(a), Value::Bool(b)) => {
            Value::Bool(cmp(if *a { 1.0 } else { 0.0 }, if *b { 1.0 } else { 0.0 }))
        }
        _ => {
            let a = match coerce_number(lhs) {
                Ok(v) => v,
                Err(err) => return Value::Error(err),
            };
            let b = match coerce_number(rhs) {
                Ok(v) => v,
                Err(err) => return Value::Error(err),
            };
            Value::Bool(cmp(a, b))
        }
    }
}

fn eval_invoke(callee: &Expr, args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    let callee_value = ctx.evaluate_expr(callee);
    let RuntimeValue::Lambda(lambda) = callee_value else {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "only LAMBDA values are callable".to_string(),
        )));
    };
    let runtime_args: Vec<RuntimeValue> = args.iter().map(|arg| ctx.evaluate_expr(arg)).collect();
    ctx.invoke_lambda_runtime(&lambda, runtime_args)
}

fn evaluate_function(name: &str, args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if let Some(local) = ctx.lookup_local(name) {
        if let RuntimeValue::Lambda(lambda) = local {
            let runtime_args: Vec<RuntimeValue> =
                args.iter().map(|arg| ctx.evaluate_expr(arg)).collect();
            return ctx.invoke_lambda_runtime(&lambda, runtime_args);
        }
        return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
            "{name} is not callable"
        ))));
    }

    match name {
        "SUM" => aggregate_numbers(args, ctx, AggregateKind::Sum),
        "MIN" => aggregate_numbers(args, ctx, AggregateKind::Min),
        "MAX" => aggregate_numbers(args, ctx, AggregateKind::Max),
        "AVERAGE" => aggregate_numbers(args, ctx, AggregateKind::Average),
        "COUNT" => aggregate_numbers(args, ctx, AggregateKind::Count),
        "IF" => eval_if(args, ctx),
        "IFERROR" => eval_iferror(args, ctx),
        "IFNA" => eval_ifna(args, ctx),
        "AND" => eval_and(args, ctx),
        "OR" => eval_or(args, ctx),
        "NOT" => eval_not(args, ctx),
        "ABS" => eval_abs(args, ctx),
        "INT" => eval_int(args, ctx),
        "ROUND" => eval_round(args, ctx),
        "SIGN" => eval_sign(args, ctx),
        "SQRT" => eval_sqrt(args, ctx),
        "EXP" => eval_exp(args, ctx),
        "LN" => eval_ln(args, ctx),
        "LOG10" => eval_log10(args, ctx),
        "SIN" => eval_sin(args, ctx),
        "COS" => eval_cos(args, ctx),
        "TAN" => eval_tan(args, ctx),
        "ATN" => eval_atn(args, ctx),
        "PI" => eval_pi(args),
        "NPV" => eval_npv(args, ctx),
        "PV" => eval_pv(args, ctx),
        "FV" => eval_fv(args, ctx),
        "PMT" => eval_pmt(args, ctx),
        "LOOKUP" => eval_lookup(args, ctx),
        "NA" => eval_na(args),
        "ERROR" => eval_error(args, ctx),
        "ISERROR" => eval_iserror(args, ctx),
        "ISNA" => eval_isna(args, ctx),
        "ISBLANK" => eval_isblank(args, ctx),
        "ISTEXT" => eval_istext(args, ctx),
        "ISNUMBER" => eval_isnumber(args, ctx),
        "ISLOGICAL" => eval_islogical(args, ctx),
        "ERROR.TYPE" => eval_error_type(args, ctx),
        "CONCAT" => eval_concat(args, ctx),
        "LEN" => eval_len(args, ctx),
        "SEQUENCE" => eval_sequence(args, ctx),
        "RANDARRAY" => eval_randarray(args, ctx),
        "LET" => eval_let(args, ctx),
        "LAMBDA" => eval_lambda(args, ctx),
        "MAP" => eval_map(args, ctx),
        "INDIRECT" => eval_indirect(args, ctx),
        "OFFSET" => eval_offset(args, ctx),
        "ROW" => eval_row(args, ctx),
        "COLUMN" => eval_column(args, ctx),
        "NOW" => eval_now(args, ctx),
        "RAND" => eval_rand(args, ctx),
        "STREAM" => eval_stream(args, ctx),
        other => {
            // Check for user-defined functions before returning an error.
            if let Some(handler) = ctx.udfs.get(other) {
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|a| ctx.evaluate_expr(a).to_scalar())
                    .collect();
                RuntimeValue::scalar(handler.call(&evaluated_args))
            } else {
                RuntimeValue::scalar(Value::Error(CellError::Name(other.to_string())))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AggregateKind {
    Sum,
    Min,
    Max,
    Average,
    Count,
}

fn aggregate_numbers(
    args: &[Expr],
    ctx: &mut EvalContext<'_>,
    kind: AggregateKind,
) -> RuntimeValue {
    let mut sum = 0.0_f64;
    let mut count = 0_u64;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for arg in args {
        match arg {
            Expr::Range(range, _, _) => {
                for cell in range.iter() {
                    let value = ctx.evaluate_cell_runtime(cell).to_scalar();
                    match value {
                        Value::Error(err) => return RuntimeValue::scalar(Value::Error(err)),
                        Value::Blank => {}
                        _ => match coerce_number(&value) {
                            Ok(num) => {
                                sum += num;
                                count += 1;
                                if num < min {
                                    min = num;
                                }
                                if num > max {
                                    max = num;
                                }
                            }
                            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
                        },
                    }
                }
            }
            _ => {
                for value in ctx.evaluate_expr(arg).flatten_values() {
                    match value {
                        Value::Error(err) => return RuntimeValue::scalar(Value::Error(err)),
                        Value::Blank => {}
                        _ => match coerce_number(&value) {
                            Ok(num) => {
                                sum += num;
                                count += 1;
                                if num < min {
                                    min = num;
                                }
                                if num > max {
                                    max = num;
                                }
                            }
                            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
                        },
                    }
                }
            }
        }
    }

    let out = match kind {
        AggregateKind::Sum => Value::Number(sum),
        AggregateKind::Min => {
            if count > 0 {
                Value::Number(min)
            } else {
                Value::Number(0.0)
            }
        }
        AggregateKind::Max => {
            if count > 0 {
                Value::Number(max)
            } else {
                Value::Number(0.0)
            }
        }
        AggregateKind::Average => {
            if count == 0 {
                Value::Number(0.0)
            } else {
                Value::Number(sum / count as f64)
            }
        }
        AggregateKind::Count => Value::Number(count as f64),
    };
    RuntimeValue::scalar(out)
}

fn eval_if(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 3 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "IF expects exactly 3 arguments".to_string(),
        )));
    }
    let condition = ctx.evaluate_expr(&args[0]).to_scalar();
    let condition = match coerce_bool(&condition) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    if condition {
        ctx.evaluate_expr(&args[1])
    } else {
        ctx.evaluate_expr(&args[2])
    }
}

fn eval_and(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "AND expects at least 1 argument".to_string(),
        )));
    }
    for arg in args {
        for value in expand_argument(arg, ctx) {
            match coerce_bool(&value) {
                Ok(false) => return RuntimeValue::scalar(Value::Bool(false)),
                Ok(true) => {}
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            }
        }
    }
    RuntimeValue::scalar(Value::Bool(true))
}

fn eval_or(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "OR expects at least 1 argument".to_string(),
        )));
    }
    for arg in args {
        for value in expand_argument(arg, ctx) {
            match coerce_bool(&value) {
                Ok(true) => return RuntimeValue::scalar(Value::Bool(true)),
                Ok(false) => {}
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            }
        }
    }
    RuntimeValue::scalar(Value::Bool(false))
}

fn eval_not(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "NOT expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    match coerce_bool(&value) {
        Ok(v) => RuntimeValue::scalar(Value::Bool(!v)),
        Err(err) => RuntimeValue::scalar(Value::Error(err)),
    }
}

fn eval_abs(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "ABS", |v| Ok(v.abs()))
}

fn eval_int(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "INT", |v| Ok(v.floor()))
}

fn eval_round(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() || args.len() > 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ROUND expects 1 or 2 arguments".to_string(),
        )));
    }
    let value = match coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar()) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    let digits = if args.len() == 2 {
        match coerce_number(&ctx.evaluate_expr(&args[1]).to_scalar()) {
            Ok(v) => v.round() as i32,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        0
    };

    let factor = 10f64.powi(digits);
    RuntimeValue::scalar(Value::Number((value * factor).round() / factor))
}

fn eval_sign(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "SIGN", |v| {
        Ok(if v > 0.0 {
            1.0
        } else if v < 0.0 {
            -1.0
        } else {
            0.0
        })
    })
}

fn eval_sqrt(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "SQRT", |v| {
        if v < 0.0 {
            return Err(CellError::Value(
                "SQRT expects a non-negative input".to_string(),
            ));
        }
        Ok(v.sqrt())
    })
}

fn eval_exp(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "EXP", |v| Ok(v.exp()))
}

fn eval_ln(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "LN", |v| {
        if v <= 0.0 {
            return Err(CellError::Value("LN expects input > 0".to_string()));
        }
        Ok(v.ln())
    })
}

fn eval_log10(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "LOG10", |v| {
        if v <= 0.0 {
            return Err(CellError::Value("LOG10 expects input > 0".to_string()));
        }
        Ok(v.log10())
    })
}

fn eval_sin(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "SIN", |v| Ok(v.sin()))
}

fn eval_cos(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "COS", |v| Ok(v.cos()))
}

fn eval_tan(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "TAN", |v| Ok(v.tan()))
}

fn eval_atn(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    eval_unary_number(args, ctx, "ATN", |v| Ok(v.atan()))
}

fn eval_pi(args: &[Expr]) -> RuntimeValue {
    if !args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "PI expects no arguments".to_string(),
        )));
    }
    RuntimeValue::scalar(Value::Number(std::f64::consts::PI))
}

fn eval_npv(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() < 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "NPV expects at least 2 arguments".to_string(),
        )));
    }

    let rate = match coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar()) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    if rate == -1.0 {
        return RuntimeValue::scalar(Value::Error(CellError::DivisionByZero));
    }

    let mut discount_index = 1.0;
    let mut npv = 0.0;
    for arg in &args[1..] {
        for value in expand_argument(arg, ctx) {
            match coerce_number(&value) {
                Ok(v) => {
                    npv += v / (1.0 + rate).powf(discount_index);
                    discount_index += 1.0;
                }
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            }
        }
    }

    RuntimeValue::scalar(Value::Number(npv))
}

fn eval_pv(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    let (rate, nper, pmt, fv, ty) = match parse_time_value_args("PV", args, ctx) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };

    let pv = if rate == 0.0 {
        -(fv + pmt * nper)
    } else {
        let term = (1.0 + rate).powf(nper);
        -(fv + pmt * (1.0 + rate * ty) * ((term - 1.0) / rate)) / term
    };
    RuntimeValue::scalar(Value::Number(pv))
}

fn eval_fv(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    let (rate, nper, pmt, pv, ty) = match parse_time_value_args("FV", args, ctx) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };

    let fv = if rate == 0.0 {
        -(pv + pmt * nper)
    } else {
        let term = (1.0 + rate).powf(nper);
        -(pv * term + pmt * (1.0 + rate * ty) * ((term - 1.0) / rate))
    };
    RuntimeValue::scalar(Value::Number(fv))
}

fn eval_pmt(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    let (rate, nper, pv, fv, ty) = match parse_time_value_args("PMT", args, ctx) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };

    if nper == 0.0 {
        return RuntimeValue::scalar(Value::Error(CellError::DivisionByZero));
    }

    let payment = if rate == 0.0 {
        -(pv + fv) / nper
    } else {
        let term = (1.0 + rate).powf(nper);
        -(rate * (fv + pv * term)) / ((1.0 + rate * ty) * (term - 1.0))
    };
    RuntimeValue::scalar(Value::Number(payment))
}

fn eval_lookup(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "LOOKUP expects exactly 2 arguments".to_string(),
        )));
    }

    let lookup = match coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar()) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    let Expr::Range(table, _, _) = &args[1] else {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "LOOKUP expects a range as the second argument".to_string(),
        )));
    };

    let rows = table.end.row - table.start.row + 1;
    let cols = table.end.col - table.start.col + 1;

    if rows == 2 {
        let mut match_col: Option<u16> = None;
        for col in table.start.col..=table.end.col {
            let key_cell = CellRef {
                col,
                row: table.start.row,
            };
            let key = match coerce_number(&ctx.evaluate_cell_runtime(key_cell).to_scalar()) {
                Ok(v) => v,
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            };
            if key <= lookup {
                match_col = Some(col);
            }
        }
        let Some(col) = match_col else {
            return RuntimeValue::scalar(Value::Error(CellError::Ref(
                "LOOKUP could not find a matching key".to_string(),
            )));
        };
        let value_cell = CellRef {
            col,
            row: table.start.row + 1,
        };
        return RuntimeValue::scalar(ctx.evaluate_cell_runtime(value_cell).to_scalar());
    }

    if cols == 2 {
        let mut match_row: Option<u16> = None;
        for row in table.start.row..=table.end.row {
            let key_cell = CellRef {
                col: table.start.col,
                row,
            };
            let key = match coerce_number(&ctx.evaluate_cell_runtime(key_cell).to_scalar()) {
                Ok(v) => v,
                Err(err) => return RuntimeValue::scalar(Value::Error(err)),
            };
            if key <= lookup {
                match_row = Some(row);
            }
        }
        let Some(row) = match_row else {
            return RuntimeValue::scalar(Value::Error(CellError::Ref(
                "LOOKUP could not find a matching key".to_string(),
            )));
        };
        let value_cell = CellRef {
            col: table.start.col + 1,
            row,
        };
        return RuntimeValue::scalar(ctx.evaluate_cell_runtime(value_cell).to_scalar());
    }

    RuntimeValue::scalar(Value::Error(CellError::Value(
        "LOOKUP expects a 2-row or 2-column range".to_string(),
    )))
}

fn eval_na(args: &[Expr]) -> RuntimeValue {
    if !args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "NA expects no arguments".to_string(),
        )));
    }
    RuntimeValue::scalar(Value::Error(CellError::Na))
}

fn eval_error(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() > 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ERROR expects zero or one argument".to_string(),
        )));
    }
    if args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value("ERROR".to_string())));
    }

    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Error(CellError::Value(value_to_text(&value))))
}

fn eval_iferror(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "IFERROR expects exactly 2 arguments".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    if matches!(value, Value::Error(_)) {
        ctx.evaluate_expr(&args[1])
    } else {
        RuntimeValue::scalar(value)
    }
}

fn eval_ifna(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "IFNA expects exactly 2 arguments".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    if matches!(value, Value::Error(CellError::Na)) {
        ctx.evaluate_expr(&args[1])
    } else {
        RuntimeValue::scalar(value)
    }
}

fn eval_iserror(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISERROR expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Error(_))))
}

fn eval_isna(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISNA expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Error(CellError::Na))))
}

fn eval_isblank(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISBLANK expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Blank)))
}

fn eval_istext(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISTEXT expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Text(_))))
}

fn eval_isnumber(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISNUMBER expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Number(_))))
}

fn eval_islogical(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ISLOGICAL expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    RuntimeValue::scalar(Value::Bool(matches!(value, Value::Bool(_))))
}

fn eval_error_type(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ERROR.TYPE expects exactly 1 argument".to_string(),
        )));
    }
    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    match value {
        Value::Error(err) => RuntimeValue::scalar(Value::Number(err.error_type_number() as f64)),
        _ => RuntimeValue::scalar(Value::Error(CellError::Na)),
    }
}

fn eval_concat(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() {
        return RuntimeValue::scalar(Value::Text(String::new()));
    }

    let mut out = String::new();
    for arg in args {
        for value in expand_argument(arg, ctx) {
            if let Value::Error(err) = value {
                return RuntimeValue::scalar(Value::Error(err));
            }
            out.push_str(&value_to_text(&value));
        }
    }
    RuntimeValue::scalar(Value::Text(out))
}

fn eval_len(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "LEN expects exactly 1 argument".to_string(),
        )));
    }

    let value = ctx.evaluate_expr(&args[0]).to_scalar();
    if let Value::Error(err) = value {
        return RuntimeValue::scalar(Value::Error(err));
    }
    RuntimeValue::scalar(Value::Number(value_to_text(&value).chars().count() as f64))
}

fn eval_let(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() < 3 || args.len() % 2 == 0 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "LET expects name/value pairs followed by a final expression".to_string(),
        )));
    }

    ctx.push_scope();

    let mut pair_index = 0usize;
    while pair_index + 1 < args.len() - 1 {
        let name = match parse_local_binding_name(&args[pair_index]) {
            Ok(name) => name,
            Err(err) => {
                ctx.pop_scope();
                return RuntimeValue::scalar(Value::Error(err));
            }
        };
        let value = ctx.evaluate_expr(&args[pair_index + 1]);
        ctx.bind_local(&name, value);
        pair_index += 2;
    }

    let result = ctx.evaluate_expr(&args[args.len() - 1]);
    ctx.pop_scope();
    result
}

fn eval_lambda(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "LAMBDA expects at least a body expression".to_string(),
        )));
    }

    let mut params = Vec::new();
    for param_expr in &args[..args.len() - 1] {
        let param = match parse_local_binding_name(param_expr) {
            Ok(name) => name,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        };
        if params.iter().any(|existing: &String| existing == &param) {
            return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
                "duplicate lambda parameter: {param}"
            ))));
        }
        params.push(param);
    }

    let body = args[args.len() - 1].clone();
    let captured = ctx.collect_visible_locals();
    RuntimeValue::Lambda(LambdaValue {
        params,
        body,
        captured,
    })
}

fn eval_map(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() < 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "MAP expects at least one array and one lambda".to_string(),
        )));
    }

    let lambda_runtime = ctx.evaluate_expr(args.last().expect("non-empty args"));
    let lambda = match lambda_runtime {
        RuntimeValue::Lambda(lambda) => lambda,
        _ => {
            return RuntimeValue::scalar(Value::Error(CellError::Value(
                "MAP expects LAMBDA as its final argument".to_string(),
            )));
        }
    };

    let mut input_arrays = Vec::new();
    for arg in &args[..args.len() - 1] {
        let runtime = runtime_from_argument(arg, ctx);
        let array = runtime.to_array_value();
        input_arrays.push(array);
    }

    let mut rows = 1usize;
    let mut cols = 1usize;
    for array in &input_arrays {
        let shape = match broadcast_shape(rows, cols, array.rows(), array.cols()) {
            Ok(shape) => shape,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        };
        rows = shape.0;
        cols = shape.1;
    }

    let mut mapped = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            let mut lambda_args = Vec::with_capacity(input_arrays.len());
            for array in &input_arrays {
                let src_row = if array.rows() == 1 { 0 } else { row };
                let src_col = if array.cols() == 1 { 0 } else { col };
                lambda_args.push(RuntimeValue::scalar(array.value_at(src_row, src_col)));
            }
            let item = ctx.invoke_lambda_runtime(&lambda, lambda_args);
            if matches!(item, RuntimeValue::Lambda(_)) {
                return RuntimeValue::scalar(Value::Error(CellError::Value(
                    "MAP lambda may not return a lambda value".to_string(),
                )));
            }
            mapped.push(item);
        }
    }

    let mut item_rows = 1usize;
    let mut item_cols = 1usize;
    for item in &mapped {
        let (next_rows, next_cols) = match item {
            RuntimeValue::Scalar(_) => (1usize, 1usize),
            RuntimeValue::Array(array) => (array.rows(), array.cols()),
            RuntimeValue::Lambda(_) => unreachable!("lambda return is rejected above"),
        };
        let shape = match broadcast_shape(item_rows, item_cols, next_rows, next_cols) {
            Ok(shape) => shape,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        };
        item_rows = shape.0;
        item_cols = shape.1;
    }

    let out_rows = rows * item_rows;
    let out_cols = cols * item_cols;
    let mut out = Vec::with_capacity(out_rows * out_cols);
    for row in 0..rows {
        for item_row in 0..item_rows {
            for col in 0..cols {
                let item = &mapped[row * cols + col];
                match item {
                    RuntimeValue::Scalar(value) => {
                        for _ in 0..item_cols {
                            out.push(value.clone());
                        }
                    }
                    RuntimeValue::Array(array) => {
                        let src_row = if array.rows() == 1 { 0 } else { item_row };
                        for item_col in 0..item_cols {
                            let src_col = if array.cols() == 1 { 0 } else { item_col };
                            out.push(array.value_at(src_row, src_col));
                        }
                    }
                    RuntimeValue::Lambda(_) => unreachable!("lambda return is rejected above"),
                }
            }
        }
    }

    if out_rows == 1 && out_cols == 1 {
        RuntimeValue::scalar(out.into_iter().next().unwrap_or(Value::Blank))
    } else {
        RuntimeValue::Array(ArrayValue::new(out_rows, out_cols, out))
    }
}

fn eval_indirect(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() || args.len() > 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "INDIRECT expects 1 or 2 arguments".to_string(),
        )));
    }

    let mut use_a1_style = true;
    if args.len() == 2 {
        use_a1_style = match coerce_bool(&ctx.evaluate_expr(&args[1]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        };
    }

    let text = value_to_text(&ctx.evaluate_expr(&args[0]).to_scalar());
    let target = if use_a1_style {
        resolve_reference_text_a1(&text, ctx.bounds)
    } else {
        resolve_reference_text_r1c1(&text, ctx.bounds, ctx.current_context_cell())
    };
    match target {
        Ok(ReferenceTarget::Cell(cell)) => {
            RuntimeValue::scalar(ctx.evaluate_cell_runtime(cell).to_scalar())
        }
        Ok(ReferenceTarget::Spill(cell)) => ctx.evaluate_spill_ref(cell),
        Ok(ReferenceTarget::Range(range)) => runtime_for_range(range, ctx),
        Err(err) => RuntimeValue::scalar(Value::Error(err)),
    }
}

fn eval_offset(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() < 3 || args.len() > 5 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "OFFSET expects between 3 and 5 arguments".to_string(),
        )));
    }

    let base_range = match resolve_reference_argument(&args[0], ctx) {
        Ok(range) => range,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    let row_delta = match coerce_number(&ctx.evaluate_expr(&args[1]).to_scalar()) {
        Ok(v) => v.round() as i32,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    let col_delta = match coerce_number(&ctx.evaluate_expr(&args[2]).to_scalar()) {
        Ok(v) => v.round() as i32,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };

    let base_height = (base_range.end.row - base_range.start.row + 1) as i32;
    let base_width = (base_range.end.col - base_range.start.col + 1) as i32;
    let height = if args.len() >= 4 {
        match coerce_number(&ctx.evaluate_expr(&args[3]).to_scalar()) {
            Ok(v) => v.round() as i32,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        base_height
    };
    let width = if args.len() >= 5 {
        match coerce_number(&ctx.evaluate_expr(&args[4]).to_scalar()) {
            Ok(v) => v.round() as i32,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        base_width
    };

    if height <= 0 || width <= 0 {
        return RuntimeValue::scalar(Value::Error(CellError::Ref(
            "OFFSET height/width must be positive".to_string(),
        )));
    }

    let start_col = base_range.start.col as i32 + col_delta;
    let start_row = base_range.start.row as i32 + row_delta;
    let end_col = start_col + width - 1;
    let end_row = start_row + height - 1;

    if start_col < 1
        || start_row < 1
        || end_col > ctx.bounds.max_columns as i32
        || end_row > ctx.bounds.max_rows as i32
    {
        return RuntimeValue::scalar(Value::Error(CellError::Ref(
            "OFFSET result is out of sheet bounds".to_string(),
        )));
    }

    let range = CellRange::new(
        CellRef {
            col: start_col as u16,
            row: start_row as u16,
        },
        CellRef {
            col: end_col as u16,
            row: end_row as u16,
        },
    );
    runtime_for_range(range, ctx)
}

fn eval_row(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() > 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "ROW expects zero or one argument".to_string(),
        )));
    }
    let row = if args.is_empty() {
        match ctx.current_context_cell() {
            Some(cell) => cell.row,
            None => {
                return RuntimeValue::scalar(Value::Error(CellError::Ref(
                    "ROW() has no current-cell context".to_string(),
                )));
            }
        }
    } else {
        match resolve_reference_argument(&args[0], ctx) {
            Ok(range) => range.start.row,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    };
    RuntimeValue::scalar(Value::Number(row as f64))
}

fn eval_column(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() > 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "COLUMN expects zero or one argument".to_string(),
        )));
    }
    let col = if args.is_empty() {
        match ctx.current_context_cell() {
            Some(cell) => cell.col,
            None => {
                return RuntimeValue::scalar(Value::Error(CellError::Ref(
                    "COLUMN() has no current-cell context".to_string(),
                )));
            }
        }
    } else {
        match resolve_reference_argument(&args[0], ctx) {
            Ok(range) => range.start.col,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    };
    RuntimeValue::scalar(Value::Number(col as f64))
}

fn eval_sequence(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() || args.len() > 4 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "SEQUENCE expects between 1 and 4 arguments".to_string(),
        )));
    }
    let rows = match eval_dimension_arg(ctx, &args[0], "rows") {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    let cols = if args.len() >= 2 {
        match eval_dimension_arg(ctx, &args[1], "columns") {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1
    };
    let start = if args.len() >= 3 {
        match coerce_number(&ctx.evaluate_expr(&args[2]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1.0
    };
    let step = if args.len() >= 4 {
        match coerce_number(&ctx.evaluate_expr(&args[3]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1.0
    };

    let total = rows.saturating_mul(cols);
    if total > 10_000 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "SEQUENCE result too large (max 10_000 cells)".to_string(),
        )));
    }

    let mut values = Vec::with_capacity(total);
    for index in 0..total {
        values.push(Value::Number(start + step * index as f64));
    }
    RuntimeValue::Array(ArrayValue::new(rows, cols, values))
}

fn eval_randarray(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.len() > 5 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "RANDARRAY expects up to 5 arguments".to_string(),
        )));
    }
    let rows = if !args.is_empty() {
        match eval_dimension_arg(ctx, &args[0], "rows") {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1
    };
    let cols = if args.len() >= 2 {
        match eval_dimension_arg(ctx, &args[1], "columns") {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1
    };
    let min = if args.len() >= 3 {
        match coerce_number(&ctx.evaluate_expr(&args[2]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        0.0
    };
    let max = if args.len() >= 4 {
        match coerce_number(&ctx.evaluate_expr(&args[3]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        1.0
    };
    let whole = if args.len() >= 5 {
        match coerce_bool(&ctx.evaluate_expr(&args[4]).to_scalar()) {
            Ok(v) => v,
            Err(err) => return RuntimeValue::scalar(Value::Error(err)),
        }
    } else {
        false
    };

    if max < min {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "RANDARRAY requires min <= max".to_string(),
        )));
    }

    let total = rows.saturating_mul(cols);
    if total > 10_000 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "RANDARRAY result too large (max 10_000 cells)".to_string(),
        )));
    }

    let mut values = Vec::with_capacity(total);
    if whole {
        let min_i = min.ceil() as i64;
        let max_i = max.floor() as i64;
        if min_i > max_i {
            return RuntimeValue::scalar(Value::Error(CellError::Value(
                "RANDARRAY integer bounds are invalid".to_string(),
            )));
        }
        let span = (max_i - min_i + 1) as f64;
        for _ in 0..total {
            let mut value = min_i + (ctx.next_rand_unit() * span).floor() as i64;
            if value > max_i {
                value = max_i;
            }
            values.push(Value::Number(value as f64));
        }
    } else {
        let span = max - min;
        for _ in 0..total {
            let n = ctx.next_rand_unit();
            values.push(Value::Number(min + span * n));
        }
    }

    RuntimeValue::Array(ArrayValue::new(rows, cols, values))
}

#[derive(Debug, Clone, Copy)]
enum ReferenceTarget {
    Cell(CellRef),
    Spill(CellRef),
    Range(CellRange),
}

fn parse_local_binding_name(expr: &Expr) -> Result<String, CellError> {
    let Expr::Name(name) = expr else {
        return Err(CellError::Value(
            "expected identifier for local binding name".to_string(),
        ));
    };
    let upper = name.to_ascii_uppercase();
    let mut chars = upper.chars();
    let Some(first) = chars.next() else {
        return Err(CellError::Value("binding name cannot be empty".to_string()));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(CellError::Value(
            "binding name must start with a letter or '_'".to_string(),
        ));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(CellError::Value(
            "binding name may only contain letters, digits, or '_'".to_string(),
        ));
    }
    if upper == "TRUE" || upper == "FALSE" {
        return Err(CellError::Value(
            "binding name cannot be TRUE or FALSE".to_string(),
        ));
    }
    if is_cell_reference_like(&upper) {
        return Err(CellError::Value(format!(
            "binding name '{upper}' conflicts with a cell reference"
        )));
    }
    if SUPPORTED_FUNCTIONS.contains(&upper.as_str()) {
        return Err(CellError::Value(format!(
            "binding name '{upper}' conflicts with a built-in function"
        )));
    }
    Ok(upper)
}

fn is_cell_reference_like(name: &str) -> bool {
    let mut seen_letter = false;
    let mut seen_digit = false;
    let mut in_digits = false;
    for ch in name.chars() {
        if ch.is_ascii_alphabetic() {
            if in_digits {
                return false;
            }
            seen_letter = true;
        } else if ch.is_ascii_digit() {
            in_digits = true;
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_letter && seen_digit
}

fn runtime_from_argument(arg: &Expr, ctx: &mut EvalContext<'_>) -> RuntimeValue {
    match arg {
        Expr::Range(range, _, _) => runtime_for_range(*range, ctx),
        _ => ctx.evaluate_expr(arg),
    }
}

fn runtime_for_range(range: CellRange, ctx: &mut EvalContext<'_>) -> RuntimeValue {
    let rows = (range.end.row - range.start.row + 1) as usize;
    let cols = (range.end.col - range.start.col + 1) as usize;
    let mut values = Vec::with_capacity(rows * cols);
    for row in range.start.row..=range.end.row {
        for col in range.start.col..=range.end.col {
            values.push(ctx.evaluate_cell_runtime(CellRef { col, row }).to_scalar());
        }
    }
    if rows == 1 && cols == 1 {
        RuntimeValue::scalar(values.into_iter().next().unwrap_or(Value::Blank))
    } else {
        RuntimeValue::Array(ArrayValue::new(rows, cols, values))
    }
}

fn resolve_reference_text_a1(
    input: &str,
    bounds: SheetBounds,
) -> Result<ReferenceTarget, CellError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CellError::Ref("reference text is empty".to_string()));
    }

    if let Some(anchor) = trimmed.strip_suffix('#') {
        let cell = parse_cell_ref(anchor, bounds).map_err(address_err_to_ref)?;
        return Ok(ReferenceTarget::Spill(cell));
    }

    if let Some((left, right)) = split_range_text(trimmed) {
        let start = parse_cell_ref(left, bounds).map_err(address_err_to_ref)?;
        let end = parse_cell_ref(right, bounds).map_err(address_err_to_ref)?;
        return Ok(ReferenceTarget::Range(CellRange::new(start, end)));
    }

    let cell = parse_cell_ref(trimmed, bounds).map_err(address_err_to_ref)?;
    Ok(ReferenceTarget::Cell(cell))
}

fn resolve_reference_text_r1c1(
    input: &str,
    bounds: SheetBounds,
    context: Option<CellRef>,
) -> Result<ReferenceTarget, CellError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CellError::Ref("reference text is empty".to_string()));
    }

    if let Some(anchor) = trimmed.strip_suffix('#') {
        let cell = parse_r1c1_ref(anchor, bounds, context)?;
        return Ok(ReferenceTarget::Spill(cell));
    }

    if let Some((left, right)) = split_range_text(trimmed) {
        let start = parse_r1c1_ref(left, bounds, context)?;
        let end = parse_r1c1_ref(right, bounds, context)?;
        return Ok(ReferenceTarget::Range(CellRange::new(start, end)));
    }

    let cell = parse_r1c1_ref(trimmed, bounds, context)?;
    Ok(ReferenceTarget::Cell(cell))
}

fn split_range_text(input: &str) -> Option<(&str, &str)> {
    if let Some((left, right)) = input.split_once(':') {
        return Some((left.trim(), right.trim()));
    }
    if let Some((left, right)) = input.split_once("...") {
        return Some((left.trim(), right.trim()));
    }
    None
}

fn address_err_to_ref(err: AddressError) -> CellError {
    CellError::Ref(err.to_string())
}

fn resolve_reference_argument(
    arg: &Expr,
    ctx: &mut EvalContext<'_>,
) -> Result<CellRange, CellError> {
    match arg {
        Expr::Cell(cell, _) => Ok(CellRange::new(*cell, *cell)),
        Expr::Range(range, _, _) => Ok(*range),
        Expr::SpillRef(cell) => match ctx.evaluate_spill_ref(*cell) {
            RuntimeValue::Array(array) => {
                let end = CellRef {
                    col: cell.col + array.cols() as u16 - 1,
                    row: cell.row + array.rows() as u16 - 1,
                };
                Ok(CellRange::new(*cell, end))
            }
            RuntimeValue::Scalar(Value::Error(err)) => Err(err),
            _ => Err(CellError::Ref(format!(
                "{cell} does not contain a spilled range"
            ))),
        },
        _ => {
            let text = value_to_text(&ctx.evaluate_expr(arg).to_scalar());
            match resolve_reference_text_a1(&text, ctx.bounds)? {
                ReferenceTarget::Cell(cell) => Ok(CellRange::new(cell, cell)),
                ReferenceTarget::Range(range) => Ok(range),
                ReferenceTarget::Spill(cell) => {
                    let runtime = ctx.evaluate_spill_ref(cell);
                    let RuntimeValue::Array(array) = runtime else {
                        return Err(CellError::Ref(format!(
                            "{cell} does not contain a spilled range"
                        )));
                    };
                    let end = CellRef {
                        col: cell.col + array.cols() as u16 - 1,
                        row: cell.row + array.rows() as u16 - 1,
                    };
                    Ok(CellRange::new(cell, end))
                }
            }
        }
    }
}

fn parse_r1c1_ref(
    input: &str,
    bounds: SheetBounds,
    context: Option<CellRef>,
) -> Result<CellRef, CellError> {
    let upper = input.trim().to_ascii_uppercase();
    let Some(stripped) = upper.strip_prefix('R') else {
        return Err(CellError::Ref(format!("invalid R1C1 reference: {input}")));
    };
    let Some(c_idx) = stripped.find('C') else {
        return Err(CellError::Ref(format!("invalid R1C1 reference: {input}")));
    };
    let row_part = &stripped[..c_idx];
    let col_part = &stripped[c_idx + 1..];
    if col_part.contains('C') {
        return Err(CellError::Ref(format!("invalid R1C1 reference: {input}")));
    }

    let row = parse_r1c1_axis(row_part, context.map(|c| c.row as i32), "row", input)?;
    let col = parse_r1c1_axis(col_part, context.map(|c| c.col as i32), "column", input)?;
    if row < 1 || row > bounds.max_rows as i32 || col < 1 || col > bounds.max_columns as i32 {
        return Err(CellError::Ref(format!(
            "R1C1 reference {input} is out of sheet bounds"
        )));
    }

    Ok(CellRef {
        col: col as u16,
        row: row as u16,
    })
}

fn parse_r1c1_axis(
    part: &str,
    current: Option<i32>,
    axis_name: &str,
    original: &str,
) -> Result<i32, CellError> {
    if part.is_empty() {
        return current.ok_or_else(|| {
            CellError::Ref(format!(
                "R1C1 {axis_name} in {original} requires a current-cell context"
            ))
        });
    }

    if part.starts_with('[') {
        if !part.ends_with(']') || part.len() < 3 {
            return Err(CellError::Ref(format!(
                "invalid R1C1 reference: {original}"
            )));
        }
        let delta = part[1..part.len() - 1]
            .parse::<i32>()
            .map_err(|_| CellError::Ref(format!("invalid R1C1 reference: {original}")))?;
        let base = current.ok_or_else(|| {
            CellError::Ref(format!(
                "R1C1 {axis_name} in {original} requires a current-cell context"
            ))
        })?;
        return Ok(base + delta);
    }

    part.parse::<i32>()
        .map_err(|_| CellError::Ref(format!("invalid R1C1 reference: {original}")))
}

fn eval_dimension_arg(
    ctx: &mut EvalContext<'_>,
    expr: &Expr,
    arg_name: &str,
) -> Result<usize, CellError> {
    let value = ctx.evaluate_expr(expr).to_scalar();
    let as_number = coerce_number(&value)?;
    if !as_number.is_finite() || as_number <= 0.0 {
        return Err(CellError::Value(format!("{arg_name} must be > 0")));
    }
    Ok(as_number.round() as usize)
}

fn eval_unary_number<F>(args: &[Expr], ctx: &mut EvalContext<'_>, name: &str, f: F) -> RuntimeValue
where
    F: FnOnce(f64) -> Result<f64, CellError>,
{
    if args.len() != 1 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(format!(
            "{name} expects exactly 1 argument"
        ))));
    }
    let value = match coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar()) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    RuntimeValue::scalar(f(value).map_or_else(Value::Error, Value::Number))
}

fn parse_time_value_args(
    name: &str,
    args: &[Expr],
    ctx: &mut EvalContext<'_>,
) -> Result<(f64, f64, f64, f64, f64), CellError> {
    if args.len() < 3 || args.len() > 5 {
        return Err(CellError::Value(format!(
            "{name} expects between 3 and 5 arguments"
        )));
    }

    let rate = coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar())?;
    let nper = coerce_number(&ctx.evaluate_expr(&args[1]).to_scalar())?;
    let v3 = coerce_number(&ctx.evaluate_expr(&args[2]).to_scalar())?;
    let v4 = if args.len() >= 4 {
        coerce_number(&ctx.evaluate_expr(&args[3]).to_scalar())?
    } else {
        0.0
    };
    let ty = if args.len() >= 5 {
        coerce_number(&ctx.evaluate_expr(&args[4]).to_scalar())?
    } else {
        0.0
    };
    if ty != 0.0 && ty != 1.0 {
        return Err(CellError::Value(format!(
            "{name} expects payment type 0 or 1"
        )));
    }
    Ok((rate, nper, v3, v4, ty))
}

fn expand_argument(arg: &Expr, ctx: &mut EvalContext<'_>) -> Vec<Value> {
    match arg {
        Expr::Range(range, _, _) => range
            .iter()
            .map(|cell| ctx.evaluate_cell_runtime(cell).to_scalar())
            .collect(),
        _ => ctx.evaluate_expr(arg).flatten_values(),
    }
}

fn coerce_number(value: &Value) -> Result<f64, CellError> {
    match value {
        Value::Number(n) => Ok(*n),
        Value::Bool(true) => Ok(1.0),
        Value::Bool(false) => Ok(0.0),
        Value::Text(_) => Err(CellError::Value(
            "text cannot be coerced to number".to_string(),
        )),
        Value::Blank => Ok(0.0),
        Value::Error(err) => Err(err.clone()),
    }
}

fn coerce_bool(value: &Value) -> Result<bool, CellError> {
    match value {
        Value::Bool(v) => Ok(*v),
        Value::Number(n) => Ok(*n != 0.0),
        Value::Text(_) => Err(CellError::Value(
            "text cannot be coerced to bool".to_string(),
        )),
        Value::Blank => Ok(false),
        Value::Error(err) => Err(err.clone()),
    }
}

fn eval_now(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if !args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "NOW expects 0 arguments".to_string(),
        )));
    }
    RuntimeValue::scalar(Value::Number(ctx.now_timestamp))
}

fn eval_rand(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if !args.is_empty() {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "RAND expects 0 arguments".to_string(),
        )));
    }
    let n = ctx.next_rand_unit();
    RuntimeValue::scalar(Value::Number(n))
}

fn eval_stream(args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    if args.is_empty() || args.len() > 2 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "STREAM expects 1-2 arguments (period [, lambda])".to_string(),
        )));
    }

    let period = match coerce_number(&ctx.evaluate_expr(&args[0]).to_scalar()) {
        Ok(v) => v,
        Err(err) => return RuntimeValue::scalar(Value::Error(err)),
    };
    if !period.is_finite() || period <= 0.0 {
        return RuntimeValue::scalar(Value::Error(CellError::Value(
            "STREAM period must be a positive number".to_string(),
        )));
    }

    let cell = match ctx.stack.last() {
        Some(EvalStackNode::Cell(cell)) => *cell,
        _ => {
            return RuntimeValue::scalar(Value::Error(CellError::Value(
                "STREAM can only be used in a cell formula".to_string(),
            )));
        }
    };

    ctx.stream_registrations.insert(
        cell,
        StreamRegistration {
            period_secs: period,
        },
    );
    let counter = ctx.stream_counters.get(&cell).copied().unwrap_or(0);

    if args.len() == 2 {
        let lambda_runtime = ctx.evaluate_expr(&args[1]);
        let lambda = match lambda_runtime {
            RuntimeValue::Lambda(lambda) => lambda,
            _ => {
                return RuntimeValue::scalar(Value::Error(CellError::Value(
                    "STREAM expects LAMBDA as its second argument".to_string(),
                )));
            }
        };
        let lambda_args = vec![RuntimeValue::scalar(Value::Number(counter as f64))];
        ctx.invoke_lambda_runtime(&lambda, lambda_args)
    } else {
        RuntimeValue::scalar(Value::Number(counter as f64))
    }
}

fn value_to_text(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        Value::Number(n) => {
            let mut s = n.to_string();
            if s.contains('.') {
                while s.ends_with('0') {
                    s.pop();
                }
                if s.ends_with('.') {
                    s.pop();
                }
            }
            if s.is_empty() { "0".to_string() } else { s }
        }
        Value::Bool(true) => "TRUE".to_string(),
        Value::Bool(false) => "FALSE".to_string(),
        Value::Blank => String::new(),
        Value::Error(err) => format!("#ERR {err}"),
    }
}
