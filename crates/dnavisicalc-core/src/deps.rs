use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use crate::address::CellRef;
use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq)]
pub struct CalcNode {
    pub cell: CellRef,
    pub expr: Rc<Expr>,
    pub dependencies: HashSet<CellRef>,
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
    pub nodes: HashMap<CellRef, CalcNode>,
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
pub fn build_calc_tree(formulas: &HashMap<CellRef, Rc<Expr>>) -> Result<CalcTree, DependencyError> {
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
pub fn build_calc_tree_allow_cycles(formulas: &HashMap<CellRef, Rc<Expr>>) -> CalcTree {
    let (nodes, formula_edges) = build_nodes_and_edges(formulas);
    let sccs = tarjan_sccs(&nodes, &formula_edges);
    let order: Vec<CellRef> = sccs
        .iter()
        .flat_map(|scc| scc.cells.iter().copied())
        .collect();
    CalcTree { nodes, order, sccs }
}

fn build_nodes_and_edges(
    formulas: &HashMap<CellRef, Rc<Expr>>,
) -> (
    HashMap<CellRef, CalcNode>,
    HashMap<CellRef, HashSet<CellRef>>,
) {
    let mut nodes: HashMap<CellRef, CalcNode> = HashMap::new();
    for (cell, expr) in formulas {
        let dependencies = dependencies_for_expr(expr);
        nodes.insert(
            *cell,
            CalcNode {
                cell: *cell,
                expr: Rc::clone(expr),
                dependencies,
            },
        );
    }

    let formula_cells: HashSet<CellRef> = nodes.keys().copied().collect();
    let mut formula_edges: HashMap<CellRef, HashSet<CellRef>> = HashMap::new();
    for (cell, node) in &nodes {
        let mut deps = HashSet::new();
        for dep in &node.dependencies {
            if formula_cells.contains(dep) {
                deps.insert(*dep);
            }
        }
        formula_edges.insert(*cell, deps);
    }

    (nodes, formula_edges)
}

pub fn dependencies_for_expr(expr: &Expr) -> HashSet<CellRef> {
    let mut out = HashSet::new();
    collect_dependencies(expr, &mut out);
    out
}

fn collect_dependencies(expr: &Expr, out: &mut HashSet<CellRef>) {
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
// Iterative SCC algorithm (Kosaraju + condensation DAG ordering)
// ---------------------------------------------------------------------------

fn tarjan_sccs(
    nodes: &HashMap<CellRef, CalcNode>,
    edges: &HashMap<CellRef, HashSet<CellRef>>,
) -> Vec<Scc> {
    // Sort node keys for deterministic iteration order.
    let mut node_list: Vec<CellRef> = nodes.keys().copied().collect();
    node_list.sort();
    let reverse_edges = build_reverse_edges(&node_list, edges);
    let components = kosaraju_components_iterative(&node_list, edges, &reverse_edges);

    if components.is_empty() {
        return Vec::new();
    }

    let comp_index = build_component_index(&components);
    let ordered_component_ids = order_components_for_evaluation(&components, &comp_index, edges);

    ordered_component_ids
        .into_iter()
        .map(|id| {
            let cells = components[id].clone();
            let is_cyclic = is_cyclic_component(&cells, edges);
            Scc { cells, is_cyclic }
        })
        .collect()
}

fn build_reverse_edges(
    nodes: &[CellRef],
    edges: &HashMap<CellRef, HashSet<CellRef>>,
) -> HashMap<CellRef, HashSet<CellRef>> {
    let mut reverse_edges: HashMap<CellRef, HashSet<CellRef>> = HashMap::new();
    for node in nodes {
        reverse_edges.entry(*node).or_default();
    }
    for (src, deps) in edges {
        for dep in deps {
            reverse_edges.entry(*dep).or_default().insert(*src);
        }
    }
    reverse_edges
}

fn kosaraju_components_iterative(
    nodes: &[CellRef],
    edges: &HashMap<CellRef, HashSet<CellRef>>,
    reverse_edges: &HashMap<CellRef, HashSet<CellRef>>,
) -> Vec<Vec<CellRef>> {
    // Pass 1: finish order on original graph.
    let mut visited: HashSet<CellRef> = HashSet::new();
    let mut finish_order: Vec<CellRef> = Vec::with_capacity(nodes.len());
    for start in nodes {
        if visited.contains(start) {
            continue;
        }
        let mut stack: Vec<(CellRef, bool)> = vec![(*start, false)];
        while let Some((cell, expanded)) = stack.pop() {
            if expanded {
                finish_order.push(cell);
                continue;
            }
            if !visited.insert(cell) {
                continue;
            }
            stack.push((cell, true));
            if let Some(deps) = edges.get(&cell) {
                // Sort deps for deterministic traversal order.
                let mut sorted_deps: Vec<CellRef> = deps.iter().copied().collect();
                sorted_deps.sort();
                for dep in sorted_deps.iter().rev() {
                    if !visited.contains(dep) {
                        stack.push((*dep, false));
                    }
                }
            }
        }
    }

    // Pass 2: DFS on transpose graph, following reverse finish order.
    let mut assigned: HashSet<CellRef> = HashSet::new();
    let mut components: Vec<Vec<CellRef>> = Vec::new();
    for start in finish_order.iter().rev() {
        if assigned.contains(start) {
            continue;
        }
        let mut stack = vec![*start];
        assigned.insert(*start);
        let mut component: Vec<CellRef> = Vec::new();
        while let Some(cell) = stack.pop() {
            component.push(cell);
            if let Some(preds) = reverse_edges.get(&cell) {
                let mut sorted_preds: Vec<CellRef> = preds.iter().copied().collect();
                sorted_preds.sort();
                for pred in sorted_preds.iter().rev() {
                    if assigned.insert(*pred) {
                        stack.push(*pred);
                    }
                }
            }
        }
        component.sort();
        components.push(component);
    }

    components
}

fn build_component_index(components: &[Vec<CellRef>]) -> HashMap<CellRef, usize> {
    let mut index = HashMap::new();
    for (component_id, cells) in components.iter().enumerate() {
        for cell in cells {
            index.insert(*cell, component_id);
        }
    }
    index
}

fn order_components_for_evaluation(
    components: &[Vec<CellRef>],
    comp_index: &HashMap<CellRef, usize>,
    edges: &HashMap<CellRef, HashSet<CellRef>>,
) -> Vec<usize> {
    // Condensation graph edges are component(dependent) -> component(dependency).
    // For evaluation, we need dependencies first, so we topologically sort the
    // reversed condensation edges: dependency -> dependent.
    let count = components.len();
    let mut dep_to_dependents: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); count];
    let mut indegree: Vec<usize> = vec![0; count];

    for (src, deps) in edges {
        let Some(&src_comp) = comp_index.get(src) else {
            continue;
        };
        for dep in deps {
            let Some(&dep_comp) = comp_index.get(dep) else {
                continue;
            };
            if src_comp == dep_comp {
                continue;
            }
            if dep_to_dependents[dep_comp].insert(src_comp) {
                indegree[src_comp] += 1;
            }
        }
    }

    let mut ready: BTreeSet<usize> = BTreeSet::new();
    for (id, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.insert(id);
        }
    }

    let mut ordered: Vec<usize> = Vec::with_capacity(count);
    while let Some(&id) = ready.iter().next() {
        ready.remove(&id);
        ordered.push(id);
        for dependent in dep_to_dependents[id].iter().copied() {
            indegree[dependent] -= 1;
            if indegree[dependent] == 0 {
                ready.insert(dependent);
            }
        }
    }

    if ordered.len() != count {
        // Should not happen for a DAG; keep deterministic fallback.
        for id in 0..count {
            if !ordered.contains(&id) {
                ordered.push(id);
            }
        }
    }

    ordered
}

fn is_cyclic_component(cells: &[CellRef], edges: &HashMap<CellRef, HashSet<CellRef>>) -> bool {
    if cells.len() > 1 {
        return true;
    }
    let c = cells[0];
    edges.get(&c).is_some_and(|deps| deps.contains(&c))
}
