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
    Cycle(Vec<CellRef>),
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::Value(msg) => write!(f, "value error: {msg}"),
            Self::Name(name) => write!(f, "unknown function: {name}"),
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

pub struct EvalContext<'a> {
    formulas: &'a HashMap<CellRef, Expr>,
    literals: &'a HashMap<CellRef, f64>,
    cache: HashMap<CellRef, Value>,
    stack: Vec<CellRef>,
}

impl<'a> EvalContext<'a> {
    pub fn new(formulas: &'a HashMap<CellRef, Expr>, literals: &'a HashMap<CellRef, f64>) -> Self {
        Self {
            formulas,
            literals,
            cache: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn evaluate_cell(&mut self, cell: CellRef) -> Value {
        if let Some(value) = self.cache.get(&cell) {
            return value.clone();
        }
        if let Some(index) = self.stack.iter().position(|c| *c == cell) {
            let mut cycle = self.stack[index..].to_vec();
            cycle.push(cell);
            return Value::Error(CellError::Cycle(cycle));
        }

        if let Some(expr) = self.formulas.get(&cell) {
            self.stack.push(cell);
            let value = self.evaluate_expr(expr);
            self.stack.pop();
            self.cache.insert(cell, value.clone());
            return value;
        }

        if let Some(number) = self.literals.get(&cell) {
            return Value::Number(*number);
        }

        Value::Blank
    }

    pub fn evaluate_expr(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Number(n) => Value::Number(*n),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Cell(cell) => self.evaluate_cell(*cell),
            Expr::Range(_) => Value::Error(CellError::Value(
                "range cannot be used as a scalar value".to_string(),
            )),
            Expr::Unary { op, expr } => {
                let value = self.evaluate_expr(expr);
                match op {
                    UnaryOp::Plus => coerce_number(&value).map_or_else(Value::Error, Value::Number),
                    UnaryOp::Minus => {
                        coerce_number(&value).map_or_else(Value::Error, |n| Value::Number(-n))
                    }
                }
            }
            Expr::Binary { op, left, right } => {
                let lval = self.evaluate_expr(left);
                if let Value::Error(err) = &lval {
                    return Value::Error(err.clone());
                }
                let rval = self.evaluate_expr(right);
                if let Value::Error(err) = &rval {
                    return Value::Error(err.clone());
                }
                evaluate_binary(*op, &lval, &rval)
            }
            Expr::FunctionCall { name, args } => evaluate_function(name, args, self),
        }
    }
}

fn evaluate_binary(op: BinaryOp, lhs: &Value, rhs: &Value) -> Value {
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

fn evaluate_function(name: &str, args: &[Expr], ctx: &mut EvalContext<'_>) -> Value {
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
        other => Value::Error(CellError::Name(other.to_string())),
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

fn aggregate_numbers(args: &[Expr], ctx: &mut EvalContext<'_>, kind: AggregateKind) -> Value {
    let mut values: Vec<f64> = Vec::new();
    for arg in args {
        let flattened = expand_argument(arg, ctx);
        for value in flattened {
            match value {
                Value::Error(err) => return Value::Error(err),
                Value::Blank => {}
                _ => match coerce_number(&value) {
                    Ok(num) => values.push(num),
                    Err(err) => return Value::Error(err),
                },
            }
        }
    }

    match kind {
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
    }
}

fn eval_if(args: &[Expr], ctx: &mut EvalContext<'_>) -> Value {
    if args.len() != 3 {
        return Value::Error(CellError::Value(
            "IF expects exactly 3 arguments".to_string(),
        ));
    }
    let condition = ctx.evaluate_expr(&args[0]);
    let condition = match coerce_bool(&condition) {
        Ok(v) => v,
        Err(err) => return Value::Error(err),
    };
    if condition {
        ctx.evaluate_expr(&args[1])
    } else {
        ctx.evaluate_expr(&args[2])
    }
}

fn eval_and(args: &[Expr], ctx: &mut EvalContext<'_>) -> Value {
    if args.is_empty() {
        return Value::Error(CellError::Value(
            "AND expects at least 1 argument".to_string(),
        ));
    }
    for arg in args {
        for value in expand_argument(arg, ctx) {
            match coerce_bool(&value) {
                Ok(false) => return Value::Bool(false),
                Ok(true) => {}
                Err(err) => return Value::Error(err),
            }
        }
    }
    Value::Bool(true)
}

fn eval_or(args: &[Expr], ctx: &mut EvalContext<'_>) -> Value {
    if args.is_empty() {
        return Value::Error(CellError::Value(
            "OR expects at least 1 argument".to_string(),
        ));
    }
    for arg in args {
        for value in expand_argument(arg, ctx) {
            match coerce_bool(&value) {
                Ok(true) => return Value::Bool(true),
                Ok(false) => {}
                Err(err) => return Value::Error(err),
            }
        }
    }
    Value::Bool(false)
}

fn eval_not(args: &[Expr], ctx: &mut EvalContext<'_>) -> Value {
    if args.len() != 1 {
        return Value::Error(CellError::Value(
            "NOT expects exactly 1 argument".to_string(),
        ));
    }
    let value = ctx.evaluate_expr(&args[0]);
    match coerce_bool(&value) {
        Ok(v) => Value::Bool(!v),
        Err(err) => Value::Error(err),
    }
}

fn expand_argument(arg: &Expr, ctx: &mut EvalContext<'_>) -> Vec<Value> {
    match arg {
        Expr::Range(range) => range.iter().map(|cell| ctx.evaluate_cell(cell)).collect(),
        _ => vec![ctx.evaluate_expr(arg)],
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
