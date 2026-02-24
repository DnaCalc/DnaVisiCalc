use std::collections::HashMap;
use std::fmt;

use crate::address::CellRef;
use crate::ast::{BinaryOp, Expr, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
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
    Ref(String),
    Spill(String),
    Cycle(Vec<CellRef>),
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::Value(msg) => write!(f, "value error: {msg}"),
            Self::Name(name) => write!(f, "unknown function: {name}"),
            Self::Ref(msg) => write!(f, "reference error: {msg}"),
            Self::Spill(msg) => write!(f, "spill error: {msg}"),
            Self::Cycle(path) => {
                let joined = path
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                write!(f, "circular reference during evaluation: {joined}")
            }
        }
    }
}

impl std::error::Error for CellError {}

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
    cache: HashMap<CellRef, RuntimeValue>,
    stack: Vec<CellRef>,
    recalc_serial: u64,
    random_counter: u64,
}

impl<'a> EvalContext<'a> {
    pub fn new(
        formulas: &'a HashMap<CellRef, Expr>,
        literals: &'a HashMap<CellRef, f64>,
        recalc_serial: u64,
    ) -> Self {
        Self {
            formulas,
            literals,
            cache: HashMap::new(),
            stack: Vec::new(),
            recalc_serial,
            random_counter: 0,
        }
    }

    pub(crate) fn evaluate_cell_runtime(&mut self, cell: CellRef) -> RuntimeValue {
        if let Some(value) = self.cache.get(&cell) {
            return value.clone();
        }
        if let Some(index) = self.stack.iter().position(|c| *c == cell) {
            let mut cycle = self.stack[index..].to_vec();
            cycle.push(cell);
            return RuntimeValue::scalar(Value::Error(CellError::Cycle(cycle)));
        }

        if let Some(expr) = self.formulas.get(&cell) {
            self.stack.push(cell);
            let value = self.evaluate_expr(expr);
            self.stack.pop();
            self.cache.insert(cell, value.clone());
            return value;
        }

        if let Some(number) = self.literals.get(&cell) {
            return RuntimeValue::scalar(Value::Number(*number));
        }

        RuntimeValue::scalar(Value::Blank)
    }

    pub(crate) fn evaluate_expr(&mut self, expr: &Expr) -> RuntimeValue {
        match expr {
            Expr::Number(n) => RuntimeValue::scalar(Value::Number(*n)),
            Expr::Bool(b) => RuntimeValue::scalar(Value::Bool(*b)),
            Expr::Cell(cell) => RuntimeValue::scalar(self.evaluate_cell_runtime(*cell).to_scalar()),
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
            .copied()
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
        Value::Blank => Ok(0.0),
        Value::Error(err) => Err(err.clone()),
    }
}

fn coerce_bool(value: &Value) -> Result<bool, CellError> {
    match value {
        Value::Bool(v) => Ok(*v),
        Value::Number(n) => Ok(*n != 0.0),
        Value::Blank => Ok(false),
        Value::Error(err) => Err(err.clone()),
    }
}
