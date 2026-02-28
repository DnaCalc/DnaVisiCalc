use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use dnavisicalc_engine::{CoreEngineId, Engine, EngineConfig, RecalcMode, col_index_to_label};

const DEFAULT_ITERATIONS: usize = 40;
const DEFAULT_FORMULA_COLS: u16 = 40;
const DEFAULT_FORMULA_ROWS: u16 = 220;

#[derive(Debug, Clone)]
struct BenchResult {
    backend: CoreEngineId,
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
}

fn main() {
    let mut iterations = DEFAULT_ITERATIONS;
    let mut output_path: Option<PathBuf> = None;
    let mut fill_full_grid_with_data = true;
    let mut formula_cols = DEFAULT_FORMULA_COLS;
    let mut formula_rows = DEFAULT_FORMULA_ROWS;

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
            "--full-data" => {
                if let Some(raw) = args.next() {
                    let normalized = raw.to_ascii_lowercase();
                    fill_full_grid_with_data = matches!(normalized.as_str(), "1" | "true" | "yes");
                }
            }
            _ => {}
        }
    }
    let workload = WorkloadConfig {
        fill_full_grid_with_data,
        formula_cols,
        formula_rows,
    };

    let mut lines = String::new();
    let _ = writeln!(
        lines,
        "Engine recalc benchmark (63x254 bounds), iterations={iterations}, full_data={}, formula_region={}x{}",
        workload.fill_full_grid_with_data, workload.formula_cols, workload.formula_rows
    );
    let _ = writeln!(
        lines,
        "Timing includes recalc after per-iteration input mutations (manual recalc mode)."
    );

    let backends = [CoreEngineId::DotnetCore, CoreEngineId::RustCore];
    let mut results: Vec<BenchResult> = Vec::new();
    for backend in backends {
        match run_backend(backend, iterations, workload) {
            Ok(result) => {
                let _ = writeln!(
                    lines,
                    "\n{}: setup={:.2}ms initial_recalc={:.2}ms recalc[min/p50/p95/mean/max]={:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms committed_epoch={}",
                    result.backend.as_str(),
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
                let _ = writeln!(lines, "\n{}: unavailable ({err})", backend.as_str());
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
                a.backend.as_str(),
                b.backend.as_str(),
                ratio
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
    backend: CoreEngineId,
    iterations: usize,
    workload: WorkloadConfig,
) -> Result<BenchResult, String> {
    let mut engine = Engine::try_new_with_config(EngineConfig {
        coreengine: backend,
        coreengine_dll: None,
    })
    .map_err(|err| err.to_string())?;

    engine.set_recalc_mode(RecalcMode::Manual);
    let bounds = engine.bounds();

    let formula_rows = bounds.max_rows.min(workload.formula_rows);
    let formula_cols = bounds.max_columns.min(workload.formula_cols);

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
            let formula = format!(
                "=ROUND((({}*1.0001+{}*0.9999+{}*0.5)+({}-{})*0.1+({}+{})*0.05+({}+{}+{})*0.01)/2,6)",
                up, left, diag, up, left, diag, left, up, left, diag
            );
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
        let col = ((i * 17) % formula_cols as usize) as u16 + 1;
        let row = ((i * 29) % formula_rows as usize) as u16 + 1;
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
        backend,
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
