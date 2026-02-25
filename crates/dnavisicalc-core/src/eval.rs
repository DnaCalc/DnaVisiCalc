use std::collections::HashMap;
use std::fmt;

use crate::address::CellRef;
use crate::address::{CellRange, SheetBounds};
use crate::ast::{BinaryOp, Expr, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
    Blank,
    Error(CellError),
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
    "CONCAT",
    "LEN",
    "SEQUENCE",
    "RANDARRAY",
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
}

impl RuntimeValue {
    pub fn scalar(value: Value) -> Self {
        Self::Scalar(value)
    }

    pub fn to_scalar(&self) -> Value {
        match self {
            Self::Scalar(value) => value.clone(),
            Self::Array(array) => array.top_left(),
        }
    }

    pub fn as_array(&self) -> Option<&ArrayValue> {
        match self {
            Self::Array(array) => Some(array),
            Self::Scalar(_) => None,
        }
    }

    fn to_array_value(&self) -> ArrayValue {
        match self {
            Self::Scalar(value) => ArrayValue::from_scalar(value.clone()),
            Self::Array(array) => array.clone(),
        }
    }

    fn flatten_values(&self) -> Vec<Value> {
        match self {
            Self::Scalar(value) => vec![value.clone()],
            Self::Array(array) => array.iter().cloned().collect(),
        }
    }
}

pub struct EvalContext<'a> {
    formulas: &'a HashMap<CellRef, Expr>,
    literals: &'a HashMap<CellRef, f64>,
    text_literals: &'a HashMap<CellRef, String>,
    name_formulas: &'a HashMap<String, Expr>,
    name_literals: &'a HashMap<String, f64>,
    name_text_literals: &'a HashMap<String, String>,
    bounds: SheetBounds,
    cache: HashMap<CellRef, RuntimeValue>,
    name_cache: HashMap<String, RuntimeValue>,
    stack: Vec<EvalStackNode>,
    recalc_serial: u64,
    random_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvalStackNode {
    Cell(CellRef),
    Name(String),
}

impl EvalStackNode {
    fn label(&self) -> String {
        match self {
            Self::Cell(cell) => cell.to_string(),
            Self::Name(name) => format!("${name}"),
        }
    }
}

impl<'a> EvalContext<'a> {
    pub fn new(
        formulas: &'a HashMap<CellRef, Expr>,
        literals: &'a HashMap<CellRef, f64>,
        text_literals: &'a HashMap<CellRef, String>,
        name_formulas: &'a HashMap<String, Expr>,
        name_literals: &'a HashMap<String, f64>,
        name_text_literals: &'a HashMap<String, String>,
        bounds: SheetBounds,
        recalc_serial: u64,
    ) -> Self {
        Self {
            formulas,
            literals,
            text_literals,
            name_formulas,
            name_literals,
            name_text_literals,
            bounds,
            cache: HashMap::new(),
            name_cache: HashMap::new(),
            stack: Vec::new(),
            recalc_serial,
            random_counter: 0,
        }
    }

    pub(crate) fn evaluate_cell_runtime(&mut self, cell: CellRef) -> RuntimeValue {
        if let Some(value) = self.cache.get(&cell) {
            return value.clone();
        }
        if let Some(index) = self
            .stack
            .iter()
            .position(|node| *node == EvalStackNode::Cell(cell))
        {
            let mut cycle: Vec<String> = self.stack[index..]
                .iter()
                .map(EvalStackNode::label)
                .collect();
            cycle.push(cell.to_string());
            return RuntimeValue::scalar(Value::Error(CellError::Cycle(cycle)));
        }

        if let Some(expr) = self.formulas.get(&cell) {
            self.stack.push(EvalStackNode::Cell(cell));
            let value = self.evaluate_expr(expr);
            self.stack.pop();
            self.cache.insert(cell, value.clone());
            return value;
        }

        if let Some(number) = self.literals.get(&cell) {
            return RuntimeValue::scalar(Value::Number(*number));
        }
        if let Some(text) = self.text_literals.get(&cell) {
            return RuntimeValue::scalar(Value::Text(text.clone()));
        }

        if let Some(value) = self.resolve_spilled_cell_value(cell) {
            return RuntimeValue::scalar(value);
        }

        RuntimeValue::scalar(Value::Blank)
    }

    pub(crate) fn evaluate_name_runtime(&mut self, name: &str) -> RuntimeValue {
        let upper = name.to_ascii_uppercase();
        if let Some(value) = self.name_cache.get(&upper) {
            return value.clone();
        }
        if let Some(index) = self
            .stack
            .iter()
            .position(|node| *node == EvalStackNode::Name(upper.clone()))
        {
            let mut cycle: Vec<String> = self.stack[index..]
                .iter()
                .map(EvalStackNode::label)
                .collect();
            cycle.push(format!("${upper}"));
            return RuntimeValue::scalar(Value::Error(CellError::Cycle(cycle)));
        }

        if let Some(expr) = self.name_formulas.get(&upper) {
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

    pub(crate) fn evaluate_expr(&mut self, expr: &Expr) -> RuntimeValue {
        match expr {
            Expr::Number(n) => RuntimeValue::scalar(Value::Number(*n)),
            Expr::Text(text) => RuntimeValue::scalar(Value::Text(text.clone())),
            Expr::Bool(b) => RuntimeValue::scalar(Value::Bool(*b)),
            Expr::Cell(cell) => RuntimeValue::scalar(self.evaluate_cell_runtime(*cell).to_scalar()),
            Expr::Name(name) => RuntimeValue::scalar(self.evaluate_name_runtime(name).to_scalar()),
            Expr::SpillRef(cell) => self.evaluate_spill_ref(*cell),
            Expr::Range(_) => RuntimeValue::scalar(Value::Error(CellError::Value(
                "range cannot be used as a scalar value".to_string(),
            ))),
            Expr::Unary { op, expr } => self.eval_unary(*op, expr),
            Expr::Binary { op, left, right } => {
                let lval = self.evaluate_expr(left);
                if let Value::Error(err) = lval.to_scalar() {
                    return RuntimeValue::scalar(Value::Error(err));
                }
                let rval = self.evaluate_expr(right);
                if let Value::Error(err) = rval.to_scalar() {
                    return RuntimeValue::scalar(Value::Error(err));
                }
                evaluate_binary_runtime(*op, &lval, &rval)
            }
            Expr::FunctionCall { name, args } => evaluate_function(name, args, self),
        }
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
        }
    }

    fn next_rand_u64(&mut self) -> u64 {
        self.random_counter = self.random_counter.wrapping_add(1);
        let anchor = self
            .stack
            .last()
            .and_then(|node| match node {
                EvalStackNode::Cell(cell) => Some(*cell),
                EvalStackNode::Name(_) => None,
            })
            .unwrap_or(CellRef { col: 1, row: 1 });
        let mut x = self.recalc_serial
            ^ ((anchor.col as u64) << 16)
            ^ ((anchor.row as u64) << 32)
            ^ self.random_counter;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
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

fn evaluate_function(name: &str, args: &[Expr], ctx: &mut EvalContext<'_>) -> RuntimeValue {
    match name {
        "SUM" => aggregate_numbers(args, ctx, AggregateKind::Sum),
        "MIN" => aggregate_numbers(args, ctx, AggregateKind::Min),
        "MAX" => aggregate_numbers(args, ctx, AggregateKind::Max),
        "AVERAGE" => aggregate_numbers(args, ctx, AggregateKind::Average),
        "COUNT" => aggregate_numbers(args, ctx, AggregateKind::Count),
        "IF" => eval_if(args, ctx),
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
        "CONCAT" => eval_concat(args, ctx),
        "LEN" => eval_len(args, ctx),
        "SEQUENCE" => eval_sequence(args, ctx),
        "RANDARRAY" => eval_randarray(args, ctx),
        other => RuntimeValue::scalar(Value::Error(CellError::Name(other.to_string()))),
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
    let mut values: Vec<f64> = Vec::new();
    for arg in args {
        let flattened = expand_argument(arg, ctx);
        for value in flattened {
            match value {
                Value::Error(err) => return RuntimeValue::scalar(Value::Error(err)),
                Value::Blank => {}
                _ => match coerce_number(&value) {
                    Ok(num) => values.push(num),
                    Err(err) => return RuntimeValue::scalar(Value::Error(err)),
                },
            }
        }
    }

    let out = match kind {
        AggregateKind::Sum => Value::Number(values.iter().copied().sum()),
        AggregateKind::Min => {
            if let Some(min) = values.into_iter().reduce(f64::min) {
                Value::Number(min)
            } else {
                Value::Number(0.0)
            }
        }
        AggregateKind::Max => {
            if let Some(max) = values.into_iter().reduce(f64::max) {
                Value::Number(max)
            } else {
                Value::Number(0.0)
            }
        }
        AggregateKind::Average => {
            if values.is_empty() {
                Value::Number(0.0)
            } else {
                let sum: f64 = values.iter().copied().sum();
                Value::Number(sum / values.len() as f64)
            }
        }
        AggregateKind::Count => Value::Number(values.len() as f64),
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
    let Expr::Range(table) = &args[1] else {
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
    RuntimeValue::scalar(Value::Error(CellError::Value("NA".to_string())))
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
        let span = (max_i - min_i + 1) as u64;
        for _ in 0..total {
            let value = min_i + (ctx.next_rand_u64() % span) as i64;
            values.push(Value::Number(value as f64));
        }
    } else {
        let span = max - min;
        for _ in 0..total {
            let n = (ctx.next_rand_u64() >> 11) as f64 / ((1u64 << 53) as f64);
            values.push(Value::Number(min + span * n));
        }
    }

    RuntimeValue::Array(ArrayValue::new(rows, cols, values))
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
        Expr::Range(range) => range
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
