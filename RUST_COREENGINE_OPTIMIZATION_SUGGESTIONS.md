# Optimization Suggestions

Post-round-3 state (2026-03-02). Three optimization rounds completed:
- Round 1: Rc<Expr>, cached eval maps (32-37% improvement)
- Round 2: Lazy cache, SCC skip, pre-sizing (18-27% improvement)
- Round 3: FxHashMap, CellGrid flat array, dirty bitset (61-66% improvement)

## Post-Round-3 Baselines (debug build, 6-profile suite)

| # | Profile | p50 | p95 | mean | min |
|---|---------|-----|-----|------|-----|
| 1 | small (30x150/15i) | 19ms | 23ms | 18ms | 6ms |
| 2 | med-rows (30x254/20i) | 33ms | 39ms | 31ms | 11ms |
| 3 | med-cols (63x150/20i) | 45ms | 52ms | 40ms | 19ms |
| 4 | med-both (50x230/25i) | 55ms | 62ms | 50ms | 18ms |
| 5 | large (63x254/20i) | 73ms | 80ms | 66ms | 25ms |
| 6 | large-full (63x254/20i/full) | 72ms | 82ms | 66ms | 25ms |

## Remaining Optimization Ideas

### 1. Pack CellRef into u32

**Description:** Union col (u16) and row (u16) into a single u32 for faster hashing, comparison, and reduced struct size. Currently CellRef is 4 bytes as two u16 fields, but the compiler may pad it. Packing into u32 makes hashing trivial (identity hash with FxHash) and comparison a single instruction.

**Estimated impact:** Medium. Would further accelerate FxHashMap lookups on CellRef keys and reduce memory for any Vec<CellRef>.

**Effort:** Moderate. Touches all CellRef construction and field access across the entire codebase.

**Rationale for deferral:** CellRef is already 4 bytes and FxHash handles it well. The CellGrid migration in round 3 removed most CellRef-keyed HashMap lookups from the hot path, reducing the marginal benefit.

### 2. Guard stream_counters HashMap

**Description:** Skip stream_counters allocation and lookup when no stream functions are registered. The `stream_counters` FxHashMap is built on every recalc even when no STREAM() functions exist.

**Estimated impact:** Low. Only one HashMap construction per recalc; the map is typically empty.

**Effort:** Trivial. Wrap in `if !self.stream_cells.is_empty()` guard.

**Rationale for deferral:** Marginal gain. Stream functions are rare in typical workloads.

### 3. Avoid to_scalar().clone() in aggregates

**Description:** SUM, COUNT, AVERAGE, and other aggregate functions currently clone values through `runtime.to_scalar()`. Could borrow or use the value directly to avoid allocation.

**Estimated impact:** Low-medium. Reduces allocations proportional to formula count per recalc iteration.

**Effort:** Moderate. Requires refactoring the aggregate evaluation paths in eval.rs to work with references instead of owned values.

**Rationale for deferral:** The eval loop is no longer the dominant bottleneck after round 3 optimizations. Most overhead is now in the expression tree walk itself.

### 4. Avoid String allocation in coerce_number

**Description:** `value.to_string()` followed by `.parse::<f64>()` in coerce_number allocates a temporary String. Could match on `Value::Text` and parse the inner `&str` directly.

**Estimated impact:** Low. Only affects text-to-number coercion paths.

**Effort:** Trivial. Simple match arm refactor.

**Rationale for deferral:** Extremely narrow hot path. Most formulas operate on numeric values and never hit this code path.

### 5. Bytecode-compile expressions

**Description:** Replace `Rc<Expr>` tree-walk interpretation with a flat bytecode array and register-based VM. Eliminates Rc pointer chasing, reduces branch misprediction from recursive enum matching, and improves data locality.

**Estimated impact:** High. Could reduce per-cell eval time by 2-4x. The expression tree walk (match on Expr variants, recursive calls, Rc dereferences) is the remaining dominant cost after rounds 1-3.

**Effort:** High. Requires a new bytecode module, compiler from Expr to bytecode, and a VM interpreter. Significant design and testing effort.

**Rationale for deferral:** Largest remaining opportunity, but also the largest implementation effort. Should be considered when further performance gains are needed beyond the current ~60% round-3 improvement.

### 6. Copy-on-write Rc<str> for Value::Text

**Description:** Replace `String` with `Rc<str>` in `Value::Text` to avoid cloning string data during evaluation. Currently every `value.clone()` on a text value allocates a new String.

**Estimated impact:** Medium on text-heavy sheets. Negligible on numeric-heavy sheets.

**Effort:** Moderate. Changes the Value enum's Text variant, affecting all pattern matches and construction sites.

**Rationale for deferral:** Most VisiCalc workloads are numeric-dominant. The benefit is proportional to the number of text values in the sheet.

### 7. Arena allocation for RuntimeValue

**Description:** Use a per-recalc bump allocator (e.g., `bumpalo`) for RuntimeValue to reduce allocation pressure. All RuntimeValues from a single recalc cycle could share one arena that's freed at once.

**Estimated impact:** Medium. Reduces allocator contention and fragmentation. Most beneficial when thousands of cells are evaluated per recalc.

**Effort:** Moderate. Requires lifetime annotations or unsafe code to tie RuntimeValue lifetimes to the arena. May conflict with the cache (which persists across evaluations within a single recalc).

**Rationale for deferral:** Rust's allocator is already fast for small allocations. The CellGrid cache migration in round 3 reduced the number of HashMap-based allocations significantly.

### 8. Parallel SCC evaluation

**Description:** Independent SCCs in the condensation DAG can be evaluated in parallel using rayon. Non-dependent SCCs have no data dependencies and can safely execute concurrently.

**Estimated impact:** High on multi-core systems. Theoretical speedup proportional to available cores for sheets with parallel dependency chains.

**Effort:** High. EvalContext is not thread-safe (mutable cache, stack, local scopes). Would require either partitioned caches, thread-local contexts, or a lock-free design.

**Rationale for deferral:** VisiCalc's 63x254 grid is relatively small. The single-threaded recalc is already fast enough for interactive use after round 3. Parallelism introduces complexity and potential correctness issues.

### 9. SIMD-accelerated numeric operations

**Description:** For sheets dominated by simple numeric arithmetic (SUM over ranges, element-wise operations), batch cell evaluations using SIMD intrinsics (SSE2/AVX2) to process 4-8 values per instruction.

**Estimated impact:** Medium for numeric-heavy sheets with large contiguous ranges. Negligible for sparse or mixed-type sheets.

**Effort:** High. Requires identifying SIMD-compatible evaluation patterns, aligning data, and writing platform-specific intrinsics or using a SIMD abstraction crate.

**Rationale for deferral:** Niche applicability. Most VisiCalc formulas are heterogeneous (different operations per cell) rather than the uniform operations that benefit from SIMD. The expression tree walk overhead dominates over raw arithmetic cost.
