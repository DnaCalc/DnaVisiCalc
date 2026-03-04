use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use dnavisicalc_engine::{
    CoreEngineId, Engine, EngineConfig, IterationConfig, RecalcMode, col_index_to_label,
};

const DEFAULT_ITERATIONS: usize = 40;
const DEFAULT_FORMULA_COLS: u16 = 40;
const DEFAULT_FORMULA_ROWS: u16 = 220;

#[derive(Debug, Clone)]
struct BenchResult {
    backend_label: String,
    setup_ms: f64,
    initial_recalc_ms: f64,
    recalc_min_ms: f64,
    recalc_p50_ms: f64,
    recalc_p95_ms: f64,
    recalc_mean_ms: f64,
    recalc_max_ms: f64,
    final_committed_epoch: u64,
}

#[derive(Debug, Clone, Copy)]
struct WorkloadConfig {
    fill_full_grid_with_data: bool,
    formula_cols: u16,
    formula_rows: u16,
    fixed_mutation_col: Option<u16>,
    fixed_mutation_row: Option<u16>,
    force_iteration_enabled: bool,
    simple_formula: bool,
}

#[derive(Debug, Clone)]
struct BenchTarget {
    label: String,
    coreengine: CoreEngineId,
    dll_path: Option<PathBuf>,
}

fn main() {
    let mut iterations = DEFAULT_ITERATIONS;
    let mut output_path: Option<PathBuf> = None;
    let mut fill_full_grid_with_data = true;
    let mut formula_cols = DEFAULT_FORMULA_COLS;
    let mut formula_rows = DEFAULT_FORMULA_ROWS;
    let mut fixed_mutation_col: Option<u16> = None;
    let mut fixed_mutation_row: Option<u16> = None;
    let mut force_iteration_enabled = false;
    let mut simple_formula = false;
    let mut include_ocaml = false;
    let mut dotnet_dll: Option<PathBuf> = None;
    let mut rust_dll: Option<PathBuf> = None;
    let mut ocaml_dll: Option<PathBuf> = None;
    let mut backend_filters: Vec<String> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iterations" => {
                if let Some(raw) = args.next()
                    && let Ok(value) = raw.parse::<usize>()
                    && value > 0
                {
                    iterations = value;
                }
            }
            "--output" => {
                if let Some(raw) = args.next() {
                    output_path = Some(PathBuf::from(raw));
                }
            }
            "--formula-cols" => {
                if let Some(raw) = args.next()
                    && let Ok(value) = raw.parse::<u16>()
                    && value > 0
                {
                    formula_cols = value;
                }
            }
            "--formula-rows" => {
                if let Some(raw) = args.next()
                    && let Ok(value) = raw.parse::<u16>()
                    && value > 0
                {
                    formula_rows = value;
                }
            }
            "--fixed-mutation-col" => {
                if let Some(raw) = args.next()
                    && let Ok(value) = raw.parse::<u16>()
                    && value > 0
                {
                    fixed_mutation_col = Some(value);
                }
            }
            "--fixed-mutation-row" => {
                if let Some(raw) = args.next()
                    && let Ok(value) = raw.parse::<u16>()
                    && value > 0
                {
                    fixed_mutation_row = Some(value);
                }
            }
            "--full-data" => {
                if let Some(raw) = args.next() {
                    let normalized = raw.to_ascii_lowercase();
                    fill_full_grid_with_data = matches!(normalized.as_str(), "1" | "true" | "yes");
                }
            }
            "--force-iteration-enabled" => {
                force_iteration_enabled = true;
            }
            "--simple-formula" => {
                simple_formula = true;
            }
            "--include-ocaml" => {
                include_ocaml = true;
            }
            "--dotnet-dll" => {
                if let Some(raw) = args.next() {
                    dotnet_dll = Some(PathBuf::from(raw));
                }
            }
            "--rust-dll" => {
                if let Some(raw) = args.next() {
                    rust_dll = Some(PathBuf::from(raw));
                }
            }
            "--ocaml-dll" => {
                if let Some(raw) = args.next() {
                    ocaml_dll = Some(PathBuf::from(raw));
                }
            }
            "--backend" => {
                if let Some(raw) = args.next() {
                    backend_filters.push(raw.trim().to_ascii_lowercase());
                }
            }
            _ => {}
        }
    }
    let workload = WorkloadConfig {
        fill_full_grid_with_data,
        formula_cols,
        formula_rows,
        fixed_mutation_col,
        fixed_mutation_row,
        force_iteration_enabled,
        simple_formula,
    };
    let mutation_mode = match (workload.fixed_mutation_col, workload.fixed_mutation_row) {
        (Some(c), Some(r)) => format!("fixed(col={c},row={r})"),
        (Some(c), None) => format!("fixed(col={c}), sweep(row)"),
        (None, Some(r)) => format!("sweep(col), fixed(row={r})"),
        (None, None) => "sweep(col,row)".to_string(),
    };

    let mut targets = vec![
        BenchTarget {
            label: CoreEngineId::DotnetCore.as_str().to_string(),
            coreengine: CoreEngineId::DotnetCore,
            dll_path: dotnet_dll,
        },
        BenchTarget {
            label: CoreEngineId::RustCore.as_str().to_string(),
            coreengine: CoreEngineId::RustCore,
            dll_path: rust_dll,
        },
    ];
    if include_ocaml {
        targets.push(BenchTarget {
            label: "ocaml-core".to_string(),
            // The loader contract is C API + explicit DLL path; this id affects default lookup only.
            coreengine: CoreEngineId::DotnetCore,
            dll_path: Some(ocaml_dll.unwrap_or_else(|| {
                PathBuf::from("engines/ocaml/coreengine-ocaml-01/dist/dvc_coreengine_ocaml01.dll")
            })),
        });
    }
    if !backend_filters.is_empty() {
        let allow: HashSet<String> = backend_filters.into_iter().collect();
        targets.retain(|t| allow.contains(&t.label));
    }

    let mut lines = String::new();
    let _ = writeln!(
        lines,
        "Engine recalc benchmark (63x254 bounds), iterations={iterations}, full_data={}, formula_region={}x{}, mutation={mutation_mode}",
        workload.fill_full_grid_with_data, workload.formula_cols, workload.formula_rows
    );
    if workload.force_iteration_enabled {
        let _ = writeln!(lines, "Iteration config override: enabled=true (benchmark probe mode).");
    }
    if workload.simple_formula {
        let _ = writeln!(lines, "Formula override: simple (=up+left+diag).");
    }
    let _ = writeln!(
        lines,
        "Timing includes recalc after per-iteration input mutations (manual recalc mode)."
    );

    let mut results: Vec<BenchResult> = Vec::new();
    for target in &targets {
        match run_backend(target, iterations, workload) {
            Ok(result) => {
                let _ = writeln!(
                    lines,
                    "\n{}: setup={:.2}ms initial_recalc={:.2}ms recalc[min/p50/p95/mean/max]={:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms committed_epoch={}",
                    result.backend_label,
                    result.setup_ms,
                    result.initial_recalc_ms,
                    result.recalc_min_ms,
                    result.recalc_p50_ms,
                    result.recalc_p95_ms,
                    result.recalc_mean_ms,
                    result.recalc_max_ms,
                    result.final_committed_epoch
                );
                results.push(result);
            }
            Err(err) => {
                let _ = writeln!(lines, "\n{}: unavailable ({err})", target.label);
            }
        }
    }

    if results.len() == 2 {
        let a = &results[0];
        let b = &results[1];
        if a.recalc_mean_ms > 0.0 && b.recalc_mean_ms > 0.0 {
            let ratio = a.recalc_mean_ms / b.recalc_mean_ms;
            let _ = writeln!(
                lines,
                "\nrelative(mean recalc): {} / {} = {:.3}x",
                a.backend_label, b.backend_label, ratio
            );
        }
    }

    print!("{lines}");
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, lines);
    }
}

fn run_backend(
    target: &BenchTarget,
    iterations: usize,
    workload: WorkloadConfig,
) -> Result<BenchResult, String> {
    let mut engine = Engine::try_new_with_config(EngineConfig {
        coreengine: target.coreengine,
        coreengine_dll: target.dll_path.clone(),
    })
    .map_err(|err| err.to_string())?;

    if workload.force_iteration_enabled {
        engine.set_iteration_config(IterationConfig {
            enabled: true,
            max_iterations: 100,
            convergence_tolerance: 0.001,
        });
    }

    engine.set_recalc_mode(RecalcMode::Manual);
    let bounds = engine.bounds();

    let formula_rows = bounds.max_rows.min(workload.formula_rows);
    let formula_cols = bounds.max_columns.min(workload.formula_cols);
    let fixed_mutation_col = workload
        .fixed_mutation_col
        .map(|c| c.min(formula_cols).max(1));
    let fixed_mutation_row = workload
        .fixed_mutation_row
        .map(|r| r.min(formula_rows).max(1));

    let setup_start = Instant::now();
    if workload.fill_full_grid_with_data {
        // Seed all cells with numeric values first so the grid is fully populated with data.
        for row in 1..=bounds.max_rows {
            for col in 1..=bounds.max_columns {
                let a1 = to_a1(col, row);
                engine
                    .set_number_a1(&a1, (row as f64) * 0.25 + (col as f64) * 1.01)
                    .map_err(|err| err.to_string())?;
            }
        }
    } else {
        // Minimal seed mode: row 1 and column 1 as mutable inputs.
        for col in 1..=formula_cols {
            let a1 = to_a1(col, 1);
            engine
                .set_number_a1(&a1, (col as f64) * 1.01)
                .map_err(|err| err.to_string())?;
        }
        for row in 1..=formula_rows {
            let a1 = to_a1(1, row);
            engine
                .set_number_a1(&a1, (row as f64) * 0.99)
                .map_err(|err| err.to_string())?;
        }
    }

    // Fill a large region with formulas referencing row, column, and diagonal neighbors.
    for row in 2..=formula_rows {
        for col in 2..=formula_cols {
            let up = to_a1(col, row - 1);
            let left = to_a1(col - 1, row);
            let diag = to_a1(col - 1, row - 1);
            let formula = if workload.simple_formula {
                format!("={}+{}+{}", up, left, diag)
            } else {
                format!(
                    "=ROUND((({}*1.0001+{}*0.9999+{}*0.5)+({}-{})*0.1+({}+{})*0.05+({}+{}+{})*0.01)/2,6)",
                    up, left, diag, up, left, diag, left, up, left, diag
                )
            };
            let cell = to_a1(col, row);
            engine
                .set_formula_a1(&cell, &formula)
                .map_err(|err| err.to_string())?;
        }
    }
    let setup_ms = setup_start.elapsed().as_secs_f64() * 1000.0;

    let initial_start = Instant::now();
    engine.recalculate().map_err(|err| err.to_string())?;
    let initial_recalc_ms = initial_start.elapsed().as_secs_f64() * 1000.0;

    let mut recalc_times_ms: Vec<f64> = Vec::with_capacity(iterations);
    for i in 0..iterations {
        let col = fixed_mutation_col.unwrap_or(((i * 17) % formula_cols as usize) as u16 + 1);
        let row = fixed_mutation_row.unwrap_or(((i * 29) % formula_rows as usize) as u16 + 1);
        let top_input = to_a1(col, 1);
        let side_input = to_a1(1, row);

        engine
            .set_number_a1(&top_input, (1000 + i) as f64 * 1.0001)
            .map_err(|err| err.to_string())?;
        engine
            .set_number_a1(&side_input, (2000 + i) as f64 * 0.9999)
            .map_err(|err| err.to_string())?;

        let recalc_start = Instant::now();
        engine.recalculate().map_err(|err| err.to_string())?;
        recalc_times_ms.push(recalc_start.elapsed().as_secs_f64() * 1000.0);
    }

    recalc_times_ms.sort_by(f64::total_cmp);
    let recalc_min_ms = recalc_times_ms[0];
    let recalc_max_ms = recalc_times_ms[recalc_times_ms.len() - 1];
    let recalc_p50_ms = percentile(&recalc_times_ms, 0.50);
    let recalc_p95_ms = percentile(&recalc_times_ms, 0.95);
    let recalc_mean_ms =
        recalc_times_ms.iter().copied().sum::<f64>() / recalc_times_ms.len() as f64;

    Ok(BenchResult {
        backend_label: target.label.clone(),
        setup_ms,
        initial_recalc_ms,
        recalc_min_ms,
        recalc_p50_ms,
        recalc_p95_ms,
        recalc_mean_ms,
        recalc_max_ms,
        final_committed_epoch: engine.committed_epoch(),
    })
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn to_a1(col: u16, row: u16) -> String {
    format!("{}{}", col_index_to_label(col), row)
}
