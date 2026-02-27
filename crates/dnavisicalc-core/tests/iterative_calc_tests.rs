use dnavisicalc_core::{Engine, IterationConfig, Value};

fn engine_with_iteration() -> Engine {
    let mut engine = Engine::new();
    engine.set_iteration_config(IterationConfig {
        enabled: true,
        max_iterations: 100,
        convergence_tolerance: 0.001,
    });
    engine
}

// --- Non-iterative cycle behavior (Excel-like fallback semantics) ---

#[test]
fn cycles_are_accepted_when_iteration_disabled() {
    let mut engine = Engine::new();
    engine.set_formula_a1("A1", "=B1").expect("set");
    // B1 references A1 — this creates a cycle at the next mutation.
    let result = engine.set_formula_a1("B1", "=A1");
    assert!(result.is_ok(), "cycle should be accepted");
    assert_eq!(engine.cell_state_a1("A1").expect("A1").value, Value::Blank);
    assert_eq!(engine.cell_state_a1("B1").expect("B1").value, Value::Blank);
}

// --- Simple self-referencing cell ---

#[test]
fn self_referencing_cell_converges() {
    // A1 = A1 + 0 should stabilize at 0 (seeded value)
    let mut engine = engine_with_iteration();
    engine.set_formula_a1("A1", "=A1").expect("set");
    let value = engine.cell_state_a1("A1").expect("query").value;
    assert_eq!(value, Value::Number(0.0));
}

// --- Classic interest calculation pattern ---
// Balance = Principal + Balance * Rate
// A1 = 1000 (principal)
// B1 = 0.05 (annual rate)
// C1 = A1 + C1 * B1  (balance = principal + balance * rate)
// Solving: C1 = 1000 / (1 - 0.05) = 1052.631...

#[test]
fn interest_circular_reference_converges() {
    let mut engine = engine_with_iteration();
    engine.set_number_a1("A1", 1000.0).expect("principal");
    engine.set_number_a1("B1", 0.05).expect("rate");
    engine
        .set_formula_a1("C1", "=A1+C1*B1")
        .expect("set formula");

    let value = engine.cell_state_a1("C1").expect("query").value;
    match value {
        Value::Number(n) => {
            let expected = 1000.0 / (1.0 - 0.05);
            assert!((n - expected).abs() < 0.01, "expected ~{expected}, got {n}");
        }
        other => panic!("expected number, got {other:?}"),
    }
}

// --- Two-cell mutual dependency ---
// A1 = B1 + 1
// B1 = A1 * 0.5
// Solving: A1 = A1*0.5 + 1 → A1 = 2, B1 = 1

#[test]
fn two_cell_mutual_cycle_converges() {
    let mut engine = engine_with_iteration();
    engine.set_formula_a1("A1", "=B1+1").expect("A1");
    engine.set_formula_a1("B1", "=A1*0.5").expect("B1");

    let a1 = engine.cell_state_a1("A1").expect("query A1").value;
    let b1 = engine.cell_state_a1("B1").expect("query B1").value;

    match (a1, b1) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a - 2.0).abs() < 0.01, "A1 expected ~2.0, got {a}");
            assert!((b - 1.0).abs() < 0.01, "B1 expected ~1.0, got {b}");
        }
        other => panic!("expected numbers, got {other:?}"),
    }
}

// --- Max iterations limit ---

#[test]
fn divergent_cycle_stops_at_max_iterations() {
    let mut engine = Engine::new();
    engine.set_iteration_config(IterationConfig {
        enabled: true,
        max_iterations: 5,
        convergence_tolerance: 0.001,
    });
    // A1 = A1 + 1 diverges: 0→1→2→3→4→5
    engine.set_formula_a1("A1", "=A1+1").expect("set");

    let value = engine.cell_state_a1("A1").expect("query").value;
    match value {
        Value::Number(n) => {
            // After 5 iterations: seed=0, iter1=1, iter2=2, iter3=3, iter4=4, iter5=5
            assert_eq!(n, 5.0, "expected 5.0 after 5 iterations, got {n}");
        }
        other => panic!("expected number, got {other:?}"),
    }
}

// --- Acyclic cells still evaluate correctly with iteration enabled ---

#[test]
fn acyclic_formulas_work_with_iteration_enabled() {
    let mut engine = engine_with_iteration();
    engine.set_number_a1("A1", 10.0).expect("A1");
    engine.set_formula_a1("B1", "=A1*2").expect("B1");
    engine.set_formula_a1("C1", "=B1+A1").expect("C1");

    assert_eq!(
        engine.cell_state_a1("B1").expect("query").value,
        Value::Number(20.0)
    );
    assert_eq!(
        engine.cell_state_a1("C1").expect("query").value,
        Value::Number(30.0)
    );
}

// --- Mixed acyclic and cyclic ---

#[test]
fn mixed_acyclic_and_cyclic_cells() {
    let mut engine = engine_with_iteration();
    // A1 = 100 (literal)
    engine.set_number_a1("A1", 100.0).expect("A1");
    // B1 = A1 + C1 * 0.1 (cyclic with C1)
    engine.set_formula_a1("B1", "=A1+C1*0.1").expect("B1");
    // C1 = B1 * 0.5 (cyclic with B1)
    engine.set_formula_a1("C1", "=B1*0.5").expect("C1");
    // D1 = C1 + 1 (acyclic, depends on cyclic result)
    engine.set_formula_a1("D1", "=C1+1").expect("D1");

    // Solve: B1 = 100 + C1*0.1, C1 = B1*0.5
    // → B1 = 100 + B1*0.5*0.1 = 100 + B1*0.05 → B1 = 100/0.95 ≈ 105.263
    // → C1 = 105.263*0.5 ≈ 52.632
    let b1 = engine.cell_state_a1("B1").expect("query").value;
    let c1 = engine.cell_state_a1("C1").expect("query").value;
    let d1 = engine.cell_state_a1("D1").expect("query").value;

    match (b1, c1, d1) {
        (Value::Number(b), Value::Number(c), Value::Number(d)) => {
            let expected_b = 100.0 / 0.95;
            let expected_c = expected_b * 0.5;
            assert!(
                (b - expected_b).abs() < 0.1,
                "B1 expected ~{expected_b}, got {b}"
            );
            assert!(
                (c - expected_c).abs() < 0.1,
                "C1 expected ~{expected_c}, got {c}"
            );
            assert!(
                (d - (expected_c + 1.0)).abs() < 0.1,
                "D1 expected ~{}, got {d}",
                expected_c + 1.0
            );
        }
        other => panic!("expected numbers, got {other:?}"),
    }
}

// --- SCC detection ---

#[test]
fn calc_tree_reports_cycles() {
    use dnavisicalc_core::{CellRef, Expr, RefFlags, SheetBounds, build_calc_tree_allow_cycles};
    use std::collections::HashMap;

    let bounds = SheetBounds {
        max_columns: 63,
        max_rows: 254,
    };
    let a1 = CellRef::new(1, 1, bounds).unwrap();
    let b1 = CellRef::new(2, 1, bounds).unwrap();

    let mut formulas = HashMap::new();
    formulas.insert(a1, Expr::Cell(b1, RefFlags::RELATIVE));
    formulas.insert(b1, Expr::Cell(a1, RefFlags::RELATIVE));

    let tree = build_calc_tree_allow_cycles(&formulas);
    assert!(tree.has_cycles());
    assert_eq!(tree.sccs.len(), 1);
    assert!(tree.sccs[0].is_cyclic);
    assert_eq!(tree.sccs[0].cells.len(), 2);
}

#[test]
fn calc_tree_no_cycles_for_dag() {
    use dnavisicalc_core::{CellRef, Expr, RefFlags, SheetBounds, build_calc_tree_allow_cycles};
    use std::collections::HashMap;

    let bounds = SheetBounds {
        max_columns: 63,
        max_rows: 254,
    };
    let a1 = CellRef::new(1, 1, bounds).unwrap();
    let b1 = CellRef::new(2, 1, bounds).unwrap();

    let mut formulas = HashMap::new();
    formulas.insert(b1, Expr::Cell(a1, RefFlags::RELATIVE));
    // a1 is not a formula cell, so b1 has no formula-dependency edge

    let tree = build_calc_tree_allow_cycles(&formulas);
    assert!(!tree.has_cycles());
}

// --- Convergence tolerance ---

#[test]
fn tight_tolerance_requires_more_iterations() {
    let mut engine = Engine::new();
    engine.set_iteration_config(IterationConfig {
        enabled: true,
        max_iterations: 3,
        convergence_tolerance: 0.0000001,
    });
    // A1 = 1000 + A1*0.05 converges to ~1052.63
    engine.set_formula_a1("A1", "=1000+A1*0.05").expect("set");

    let value = engine.cell_state_a1("A1").expect("query").value;
    match value {
        Value::Number(n) => {
            // With only 3 iterations and tight tolerance, won't fully converge
            // but should be partway there
            assert!(n > 0.0, "should have a positive value, got {n}");
        }
        other => panic!("expected number, got {other:?}"),
    }
}

// --- IFERROR with iterative calc ---

#[test]
fn iferror_works_in_iterative_context() {
    let mut engine = engine_with_iteration();
    // A1 = IFERROR(B1, 0) + 1
    // B1 = A1 * 0.5
    // With iteration: A1 and B1 form a cycle
    // Solving: A1 = B1 + 1, B1 = A1*0.5 → A1 = 2, B1 = 1
    engine.set_formula_a1("A1", "=IFERROR(B1,0)+1").expect("A1");
    engine.set_formula_a1("B1", "=A1*0.5").expect("B1");

    let a1 = engine.cell_state_a1("A1").expect("query").value;
    let b1 = engine.cell_state_a1("B1").expect("query").value;

    match (a1, b1) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a - 2.0).abs() < 0.01, "A1 expected ~2.0, got {a}");
            assert!((b - 1.0).abs() < 0.01, "B1 expected ~1.0, got {b}");
        }
        other => panic!("expected numbers, got {other:?}"),
    }
}
