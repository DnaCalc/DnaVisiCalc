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

/// A strongly connected component in the dependency graph.
#[derive(Debug, Clone, PartialEq)]
pub struct Scc {
    /// Cells in this SCC, in evaluation order (topological within acyclic SCCs).
    pub cells: Vec<CellRef>,
    /// True if this SCC contains a cycle (self-loop or mutual dependency).
    pub is_cyclic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalcTree {
    pub nodes: BTreeMap<CellRef, CalcNode>,
    /// Flat topological evaluation order (only valid when there are no cycles).
    pub order: Vec<CellRef>,
    /// SCCs in reverse-topological order of the condensation DAG
    /// (dependencies come before dependents). Each SCC lists its member cells.
    pub sccs: Vec<Scc>,
}

impl CalcTree {
    /// Returns true if the dependency graph contains any cycles.
    pub fn has_cycles(&self) -> bool {
        self.sccs.iter().any(|scc| scc.is_cyclic)
    }
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

/// Builds the calculation tree with cycle detection. Returns an error if any
/// circular dependency is found. This is the original strict-mode behaviour.
pub fn build_calc_tree(formulas: &HashMap<CellRef, Expr>) -> Result<CalcTree, DependencyError> {
    let (nodes, formula_edges) = build_nodes_and_edges(formulas);
    let sccs = tarjan_sccs(&nodes, &formula_edges);

    // Check for cycles — report the first one found.
    for scc in &sccs {
        if scc.is_cyclic {
            let mut cycle = scc.cells.clone();
            cycle.push(scc.cells[0]);
            return Err(DependencyError::Cycle(cycle));
        }
    }

    let order: Vec<CellRef> = sccs
        .iter()
        .flat_map(|scc| scc.cells.iter().copied())
        .collect();
    Ok(CalcTree { nodes, order, sccs })
}

/// Builds the calculation tree allowing circular dependencies. Cycles are
/// captured as cyclic SCCs rather than producing errors. The Engine can then
/// choose to iterate cyclic SCCs or report errors.
pub fn build_calc_tree_allow_cycles(formulas: &HashMap<CellRef, Expr>) -> CalcTree {
    let (nodes, formula_edges) = build_nodes_and_edges(formulas);
    let sccs = tarjan_sccs(&nodes, &formula_edges);
    let order: Vec<CellRef> = sccs
        .iter()
        .flat_map(|scc| scc.cells.iter().copied())
        .collect();
    CalcTree { nodes, order, sccs }
}

fn build_nodes_and_edges(
    formulas: &HashMap<CellRef, Expr>,
) -> (
    BTreeMap<CellRef, CalcNode>,
    BTreeMap<CellRef, BTreeSet<CellRef>>,
) {
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

    (nodes, formula_edges)
}

pub fn dependencies_for_expr(expr: &Expr) -> BTreeSet<CellRef> {
    let mut out = BTreeSet::new();
    collect_dependencies(expr, &mut out);
    out
}

fn collect_dependencies(expr: &Expr, out: &mut BTreeSet<CellRef>) {
    match expr {
        Expr::Cell(cell, _) => {
            out.insert(*cell);
        }
        Expr::SpillRef(cell) => {
            out.insert(*cell);
        }
        Expr::Range(range, _, _) => {
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
        Expr::Invoke { callee, args } => {
            collect_dependencies(callee, out);
            for arg in args {
                collect_dependencies(arg, out);
            }
        }
        Expr::Number(_) | Expr::Text(_) | Expr::Bool(_) | Expr::Name(_) => {}
    }
}

// ---------------------------------------------------------------------------
// Tarjan's SCC algorithm
// ---------------------------------------------------------------------------

struct TarjanState {
    index_counter: usize,
    stack: Vec<CellRef>,
    on_stack: BTreeSet<CellRef>,
    indices: BTreeMap<CellRef, usize>,
    lowlinks: BTreeMap<CellRef, usize>,
    sccs: Vec<Scc>,
}

fn tarjan_sccs(
    nodes: &BTreeMap<CellRef, CalcNode>,
    edges: &BTreeMap<CellRef, BTreeSet<CellRef>>,
) -> Vec<Scc> {
    let mut state = TarjanState {
        index_counter: 0,
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        indices: BTreeMap::new(),
        lowlinks: BTreeMap::new(),
        sccs: Vec::new(),
    };

    for cell in nodes.keys() {
        if !state.indices.contains_key(cell) {
            tarjan_visit(*cell, edges, &mut state);
        }
    }

    // For our dependency graph convention (edges from dependents to
    // dependencies), Tarjan naturally produces SCCs in evaluation order:
    // dependencies before dependents. No reversal needed.
    state.sccs
}

fn tarjan_visit(
    cell: CellRef,
    edges: &BTreeMap<CellRef, BTreeSet<CellRef>>,
    state: &mut TarjanState,
) {
    let idx = state.index_counter;
    state.index_counter += 1;
    state.indices.insert(cell, idx);
    state.lowlinks.insert(cell, idx);
    state.stack.push(cell);
    state.on_stack.insert(cell);

    if let Some(deps) = edges.get(&cell) {
        for dep in deps {
            if !state.indices.contains_key(dep) {
                tarjan_visit(*dep, edges, state);
                let dep_lowlink = state.lowlinks[dep];
                let cell_lowlink = state.lowlinks.get_mut(&cell).unwrap();
                if dep_lowlink < *cell_lowlink {
                    *cell_lowlink = dep_lowlink;
                }
            } else if state.on_stack.contains(dep) {
                let dep_index = state.indices[dep];
                let cell_lowlink = state.lowlinks.get_mut(&cell).unwrap();
                if dep_index < *cell_lowlink {
                    *cell_lowlink = dep_index;
                }
            }
        }
    }

    if state.lowlinks[&cell] == state.indices[&cell] {
        let mut scc_cells = Vec::new();
        loop {
            let w = state.stack.pop().unwrap();
            state.on_stack.remove(&w);
            scc_cells.push(w);
            if w == cell {
                break;
            }
        }
        // Tarjan pops in reverse order; reverse for a more natural ordering.
        scc_cells.reverse();

        let is_cyclic = if scc_cells.len() > 1 {
            true
        } else {
            // Single-node SCC: cyclic only if it has a self-edge.
            let c = scc_cells[0];
            edges.get(&c).map_or(false, |deps| deps.contains(&c))
        };

        state.sccs.push(Scc {
            cells: scc_cells,
            is_cyclic,
        });
    }
}
