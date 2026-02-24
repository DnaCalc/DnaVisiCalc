use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use crate::address::CellRef;
use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct CalcNode {
    pub cell: CellRef,
    pub expr: Expr,
    pub dependencies: BTreeSet<CellRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalcTree {
    pub nodes: BTreeMap<CellRef, CalcNode>,
    pub order: Vec<CellRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DependencyError {
    Cycle(Vec<CellRef>),
}

impl fmt::Display for DependencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle(path) => {
                let joined = path
                    .iter()
                    .map(|cell| cell.to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                write!(f, "circular reference detected: {joined}")
            }
        }
    }
}

impl std::error::Error for DependencyError {}

pub fn build_calc_tree(formulas: &HashMap<CellRef, Expr>) -> Result<CalcTree, DependencyError> {
    let mut nodes: BTreeMap<CellRef, CalcNode> = BTreeMap::new();
    for (cell, expr) in formulas {
        let dependencies = dependencies_for_expr(expr);
        nodes.insert(
            *cell,
            CalcNode {
                cell: *cell,
                expr: expr.clone(),
                dependencies,
            },
        );
    }

    let formula_cells: BTreeSet<CellRef> = nodes.keys().copied().collect();
    let mut formula_edges: BTreeMap<CellRef, BTreeSet<CellRef>> = BTreeMap::new();
    for (cell, node) in &nodes {
        let mut deps = BTreeSet::new();
        for dep in &node.dependencies {
            if formula_cells.contains(dep) {
                deps.insert(*dep);
            }
        }
        formula_edges.insert(*cell, deps);
    }

    let mut marks: BTreeMap<CellRef, VisitMark> = BTreeMap::new();
    let mut order: Vec<CellRef> = Vec::new();
    let mut stack: Vec<CellRef> = Vec::new();

    for cell in nodes.keys().copied() {
        if marks.get(&cell).is_none() {
            visit(cell, &formula_edges, &mut marks, &mut stack, &mut order)?;
        }
    }

    Ok(CalcTree { nodes, order })
}

pub fn dependencies_for_expr(expr: &Expr) -> BTreeSet<CellRef> {
    let mut out = BTreeSet::new();
    collect_dependencies(expr, &mut out);
    out
}

fn collect_dependencies(expr: &Expr, out: &mut BTreeSet<CellRef>) {
    match expr {
        Expr::Cell(cell) => {
            out.insert(*cell);
        }
        Expr::SpillRef(cell) => {
            out.insert(*cell);
        }
        Expr::Range(range) => {
            for cell in range.iter() {
                out.insert(cell);
            }
        }
        Expr::Unary { expr, .. } => {
            collect_dependencies(expr, out);
        }
        Expr::Binary { left, right, .. } => {
            collect_dependencies(left, out);
            collect_dependencies(right, out);
        }
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                collect_dependencies(arg, out);
            }
        }
        Expr::Number(_) | Expr::Bool(_) => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitMark {
    Visiting,
    Visited,
}

fn visit(
    cell: CellRef,
    edges: &BTreeMap<CellRef, BTreeSet<CellRef>>,
    marks: &mut BTreeMap<CellRef, VisitMark>,
    stack: &mut Vec<CellRef>,
    order: &mut Vec<CellRef>,
) -> Result<(), DependencyError> {
    marks.insert(cell, VisitMark::Visiting);
    stack.push(cell);

    if let Some(deps) = edges.get(&cell) {
        for dep in deps {
            match marks.get(dep).copied() {
                Some(VisitMark::Visiting) => {
                    let start_idx = stack.iter().position(|c| c == dep).unwrap_or(0);
                    let mut cycle = stack[start_idx..].to_vec();
                    cycle.push(*dep);
                    return Err(DependencyError::Cycle(cycle));
                }
                Some(VisitMark::Visited) => {}
                None => visit(*dep, edges, marks, stack, order)?,
            }
        }
    }

    stack.pop();
    marks.insert(cell, VisitMark::Visited);
    order.push(cell);
    Ok(())
}
