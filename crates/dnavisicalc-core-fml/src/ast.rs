use crate::address::{CellRange, CellRef, SheetBounds, col_index_to_label};

/// Flags indicating whether column and/or row parts of a cell reference are
/// absolute (`$`-prefixed) or relative. Relative references are adjusted
/// when rows/columns are inserted or deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RefFlags {
    pub col_absolute: bool,
    pub row_absolute: bool,
}

impl RefFlags {
    pub const RELATIVE: Self = Self {
        col_absolute: false,
        row_absolute: false,
    };
    pub const ABSOLUTE: Self = Self {
        col_absolute: true,
        row_absolute: true,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Bool(bool),
    Cell(CellRef, RefFlags),
    Name(String),
    SpillRef(CellRef),
    Range(CellRange, RefFlags, RefFlags),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
    Invoke {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

// ---------------------------------------------------------------------------
// Structural mutation: reference rewriting for insert/delete row/column
// ---------------------------------------------------------------------------

/// Describes a structural mutation to the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralOp {
    InsertRow { at: u16 },
    DeleteRow { at: u16 },
    InsertCol { at: u16 },
    DeleteCol { at: u16 },
}

/// Result of rewriting a single cell reference coordinate.
enum CoordResult {
    /// The coordinate was shifted to a new value.
    Shifted(u16),
    /// The reference was invalidated (e.g., deleted row that was absolute).
    Invalidated,
    /// No change needed.
    Unchanged(u16),
}

/// Adjust a single coordinate (row or column) for an insert or delete.
/// `coord`: the current row or column (1-based)
/// `is_absolute`: whether this axis has a `$` prefix
/// `is_row_op`: true for row operations, false for column operations
/// `op`: the structural operation
fn adjust_coord(coord: u16, _is_absolute: bool, op: StructuralOp) -> CoordResult {
    match op {
        StructuralOp::InsertRow { at } | StructuralOp::InsertCol { at } => {
            // For insert: references at or after the insertion point shift down/right.
            // Absolute references also shift (Excel behaviour: $A$5 becomes $A$6
            // when a row is inserted at row 5).
            if coord >= at {
                CoordResult::Shifted(coord + 1)
            } else {
                CoordResult::Unchanged(coord)
            }
        }
        StructuralOp::DeleteRow { at } | StructuralOp::DeleteCol { at } => {
            if coord == at {
                // Reference points directly at the deleted row/col → #REF!
                CoordResult::Invalidated
            } else if coord > at {
                CoordResult::Shifted(coord - 1)
            } else {
                CoordResult::Unchanged(coord)
            }
        }
    }
}

/// Rewrite all cell references in an expression tree for a structural mutation.
/// Returns `None` if any reference was invalidated (the formula should become `#REF!`).
pub fn rewrite_expr(expr: &Expr, op: StructuralOp, bounds: SheetBounds) -> Option<Expr> {
    match expr {
        Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Name(_) => Some(expr.clone()),

        Expr::Cell(cell_ref, flags) => {
            let new_ref = rewrite_cell_ref(*cell_ref, *flags, op, bounds)?;
            Some(Expr::Cell(new_ref, *flags))
        }

        Expr::SpillRef(cell_ref) => {
            // SpillRef carries no flags, treat as relative.
            let new_ref = rewrite_cell_ref(*cell_ref, RefFlags::RELATIVE, op, bounds)?;
            Some(Expr::SpillRef(new_ref))
        }

        Expr::Range(range, start_flags, end_flags) => {
            let new_start = rewrite_cell_ref(range.start, *start_flags, op, bounds)?;
            let new_end = rewrite_cell_ref(range.end, *end_flags, op, bounds)?;
            Some(Expr::Range(
                CellRange::new(new_start, new_end),
                *start_flags,
                *end_flags,
            ))
        }

        Expr::Unary { op: uop, expr: sub } => {
            let new_sub = rewrite_expr(sub, op, bounds)?;
            Some(Expr::Unary {
                op: *uop,
                expr: Box::new(new_sub),
            })
        }

        Expr::Binary {
            op: bop,
            left,
            right,
        } => {
            let new_left = rewrite_expr(left, op, bounds)?;
            let new_right = rewrite_expr(right, op, bounds)?;
            Some(Expr::Binary {
                op: *bop,
                left: Box::new(new_left),
                right: Box::new(new_right),
            })
        }

        Expr::FunctionCall { name, args } => {
            let new_args: Option<Vec<Expr>> =
                args.iter().map(|a| rewrite_expr(a, op, bounds)).collect();
            Some(Expr::FunctionCall {
                name: name.clone(),
                args: new_args?,
            })
        }

        Expr::Invoke { callee, args } => {
            let new_callee = rewrite_expr(callee, op, bounds)?;
            let new_args: Option<Vec<Expr>> =
                args.iter().map(|a| rewrite_expr(a, op, bounds)).collect();
            Some(Expr::Invoke {
                callee: Box::new(new_callee),
                args: new_args?,
            })
        }
    }
}

/// Rewrite a single CellRef according to the structural operation.
/// Returns `None` if the reference is invalidated.
fn rewrite_cell_ref(
    cell: CellRef,
    flags: RefFlags,
    op: StructuralOp,
    bounds: SheetBounds,
) -> Option<CellRef> {
    let is_row_op = matches!(
        op,
        StructuralOp::InsertRow { .. } | StructuralOp::DeleteRow { .. }
    );

    let new_col = if is_row_op {
        cell.col // row operations don't affect columns
    } else {
        match adjust_coord(cell.col, flags.col_absolute, op) {
            CoordResult::Shifted(c) => c,
            CoordResult::Invalidated => return None,
            CoordResult::Unchanged(c) => c,
        }
    };

    let new_row = if !is_row_op {
        cell.row // column operations don't affect rows
    } else {
        match adjust_coord(cell.row, flags.row_absolute, op) {
            CoordResult::Shifted(r) => r,
            CoordResult::Invalidated => return None,
            CoordResult::Unchanged(r) => r,
        }
    };

    // Check bounds — if the shifted coordinate is out of range, invalidate.
    if new_col == 0 || new_col > bounds.max_columns || new_row == 0 || new_row > bounds.max_rows {
        return None;
    }

    Some(CellRef {
        col: new_col,
        row: new_row,
    })
}

// ---------------------------------------------------------------------------
// Formula reconstruction: Expr → formula string
// ---------------------------------------------------------------------------

fn format_cell_ref(cell: &CellRef, flags: &RefFlags) -> String {
    let col_part = if flags.col_absolute {
        format!("${}", col_index_to_label(cell.col))
    } else {
        col_index_to_label(cell.col)
    };
    let row_part = if flags.row_absolute {
        format!("${}", cell.row)
    } else {
        format!("{}", cell.row)
    };
    format!("{col_part}{row_part}")
}

/// Convert an expression AST back to a formula string (with leading `=`).
pub fn expr_to_formula(expr: &Expr) -> String {
    format!("={}", expr_to_string(expr))
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => format_number(*n),
        Expr::Text(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        Expr::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Expr::Cell(cell, flags) => format_cell_ref(cell, flags),
        Expr::Name(name) => name.clone(),
        Expr::SpillRef(cell) => format!("{}#", cell.to_a1()),
        Expr::Range(range, sf, ef) => {
            format!(
                "{}:{}",
                format_cell_ref(&range.start, sf),
                format_cell_ref(&range.end, ef),
            )
        }
        Expr::Unary { op, expr } => {
            let op_str = match op {
                UnaryOp::Plus => "+",
                UnaryOp::Minus => "-",
            };
            format!("{op_str}{}", expr_to_string(expr))
        }
        Expr::Binary { op, left, right } => {
            let op_str = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Pow => "^",
                BinaryOp::Concat => "&",
                BinaryOp::Eq => "=",
                BinaryOp::Ne => "<>",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
            };
            // Wrap in parens to preserve associativity — simple approach.
            format!(
                "({}{}{})",
                expr_to_string(left),
                op_str,
                expr_to_string(right),
            )
        }
        Expr::FunctionCall { name, args } => {
            let arg_str = args
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("{name}({arg_str})")
        }
        Expr::Invoke { callee, args } => {
            let arg_str = args
                .iter()
                .map(expr_to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("({})({})", expr_to_string(callee), arg_str)
        }
    }
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        // Integer-like: emit without decimals.
        format!("{:.0}", n)
    } else {
        format!("{}", n)
    }
}
