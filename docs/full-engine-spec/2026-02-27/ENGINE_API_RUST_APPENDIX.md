# Engine API Rust Mapping Appendix

This appendix contains non-normative Rust-specific mapping and implementation notes moved out of `ENGINE_API.md`.

## 1. Per-Function Rust Method Mapping (from prior `Maps to` annotations)

| API Section | Rust Mapping |
|-------------|--------------|
| dvc_engine_create | `Engine::new()` |
| dvc_engine_create_with_bounds | `Engine::with_bounds()` |
| dvc_engine_destroy | `drop(Engine)` |
| dvc_engine_clear | `Engine::clear()` |
| dvc_engine_bounds | `Engine::bounds()` |
| dvc_engine_get_recalc_mode | `Engine::recalc_mode()` |
| dvc_engine_set_recalc_mode | `Engine::set_recalc_mode()` |
| dvc_engine_committed_epoch | `Engine::committed_epoch()` |
| dvc_engine_stabilized_epoch | `Engine::stabilized_epoch()` |
| dvc_engine_is_stable | comparison of `Engine::stabilized_epoch()` and `Engine::committed_epoch()` |
| dvc_cell_set_number | `Engine::set_number()` |
| dvc_cell_set_text | `Engine::set_text()` |
| dvc_cell_set_formula | `Engine::set_formula()` |
| dvc_cell_clear | `Engine::clear_cell()` |
| dvc_cell_get_state | `Engine::cell_state()` |
| dvc_cell_get_text | `Engine::cell_state()` → `Value::Text` |
| dvc_cell_get_input_type | `Engine::cell_input()` |
| dvc_cell_get_input_text | `Engine::cell_input()` |
| 5. Cell Functions (A1 string addressing) | `Engine::set_number_a1()`, `set_text_a1()`, `set_formula_a1()`, `clear_cell_a1()`, `cell_state_a1()`, `cell_input_a1()` |
| dvc_name_set_number | `Engine::set_name_number()` |
| dvc_name_set_text | `Engine::set_name_text()` |
| dvc_name_set_formula | `Engine::set_name_formula()` |
| dvc_name_clear | `Engine::clear_name()` |
| dvc_name_get_input_type | `Engine::name_input()` |
| dvc_name_get_input_text | `Engine::name_input()` |
| dvc_recalculate | `Engine::recalculate()` |
| dvc_has_volatile_cells | `Engine::has_volatile_cells()` |
| dvc_has_externally_invalidated_cells | `Engine::has_externally_invalidated_cells()` |
| dvc_invalidate_volatile | `Engine::invalidate_volatile()` |
| dvc_has_stream_cells | `Engine::has_stream_cells()` |
| dvc_tick_streams | `Engine::tick_streams()` |
| dvc_invalidate_udf | `Engine::invalidate_udf()` |
| dvc_cell_get_format | `Engine::cell_format()` |
| dvc_cell_set_format | `Engine::set_cell_format()` |
| dvc_cell_get_format_a1 / dvc_cell_set_format_a1 | `Engine::cell_format_a1()`, `Engine::set_cell_format_a1()` |
| dvc_cell_spill_role | `Engine::spill_anchor_for_cell()`, `Engine::spill_range_for_cell()` |
| dvc_cell_spill_anchor | `Engine::spill_anchor_for_cell()` |
| dvc_cell_spill_range | `Engine::spill_range_for_cell()` |
| Cell Input Iterator | `Engine::all_cell_inputs()` |
| Name Input Iterator | `Engine::all_name_inputs()` |
| Format Iterator | `Engine::all_cell_formats()` |
| dvc_insert_row | `Engine::insert_row()` |
| dvc_delete_row | `Engine::delete_row()` |
| dvc_insert_col | `Engine::insert_col()` |
| dvc_delete_col | `Engine::delete_col()` |
| dvc_engine_get_iteration_config | `Engine::iteration_config()` |
| dvc_engine_set_iteration_config | `Engine::set_iteration_config()` |
| dvc_control_define | `Engine::define_control()` |
| dvc_control_remove | `Engine::remove_control()` |
| dvc_control_set_value | `Engine::set_control_value()` |
| dvc_control_get_value | `Engine::control_value()` |
| dvc_control_get_def | `Engine::control_definition()` |
| Control Iterator | `Engine::all_controls()` |
| dvc_chart_define | `Engine::define_chart()` |
| dvc_chart_remove | `Engine::remove_chart()` |
| dvc_chart_get_output | `Engine::chart_output()` |
| Chart Iterator | `Engine::all_charts()` |
| dvc_udf_register | `Engine::register_udf()` |
| dvc_udf_unregister | `Engine::unregister_udf()` |
| dvc_change_tracking_enable | `Engine::enable_change_tracking()` |
| dvc_change_tracking_disable | `Engine::disable_change_tracking()` |
| dvc_change_tracking_is_enabled | `Engine::is_change_tracking_enabled()` |
| Change Iterator | `Engine::drain_changes()` |
| dvc_cell_error_message | `CellError::Display` |
| dvc_palette_color_name | `PaletteColor::as_name()` |
| dvc_parse_cell_ref | `parse_cell_ref()` |

## 2. Rust Implementation Notes (moved from former `ENGINE_API.md` §21)

The future `dnavisicalc-cabi` crate will wrap the engine:

```rust
// Internal wrapper (not exposed through C API)
struct DvcEngine {
    inner: Engine,
    last_error: Option<String>,
    last_error_kind: DvcStatus,
    last_reject_kind: DvcRejectKind,
    last_reject_context: DvcLastRejectContext,
}
```

Each `#[no_mangle] extern "C"` function will:
1. Validate pointer arguments (NULL → `DVC_ERR_NULL_POINTER`)
2. Call the corresponding `Engine` method
3. Map result to `DvcStatus` outcome (`DVC_OK`, `DVC_REJECT_*`, or `DVC_ERR_*`)
4. Update diagnostics per §20 and return status

`DvcStatus` mapping from Rust errors:

| Rust Error | DvcStatus |
|-----------|-----------|
| `EngineError::Address(_)` | `DVC_ERR_INVALID_ADDRESS` |
| `EngineError::Parse(_)` | `DVC_ERR_PARSE` |
| `EngineError::Dependency(_)` | `DVC_ERR_DEPENDENCY` |
| `EngineError::Name(_)` | `DVC_ERR_INVALID_NAME` |
| `EngineError::OutOfBounds(_)` | `DVC_ERR_OUT_OF_BOUNDS` |

`DVC_REJECT_*` statuses are produced by explicit policy/constraint checks (for example structural-constraint rejections), not by `EngineError` mapping.


## 3. API Coverage Cross-Reference (moved from former `ENGINE_API.md` §22)

### Engine methods → API functions

| Engine method | API function |
|--------------|-------------|
| `Engine::new()` | `dvc_engine_create` |
| `Engine::with_bounds()` | `dvc_engine_create_with_bounds` |
| `drop(Engine)` | `dvc_engine_destroy` |
| `Engine::clear()` | `dvc_engine_clear` |
| `Engine::bounds()` | `dvc_engine_bounds` |
| `Engine::recalc_mode()` | `dvc_engine_get_recalc_mode` |
| `Engine::set_recalc_mode()` | `dvc_engine_set_recalc_mode` |
| `Engine::committed_epoch()` | `dvc_engine_committed_epoch` |
| `Engine::stabilized_epoch()` | `dvc_engine_stabilized_epoch` |
| `Engine::set_number()` | `dvc_cell_set_number` |
| `Engine::set_text()` | `dvc_cell_set_text` |
| `Engine::set_formula()` | `dvc_cell_set_formula` |
| `Engine::clear_cell()` | `dvc_cell_clear` |
| `Engine::set_number_a1()` | `dvc_cell_set_number_a1` |
| `Engine::set_text_a1()` | `dvc_cell_set_text_a1` |
| `Engine::set_formula_a1()` | `dvc_cell_set_formula_a1` |
| `Engine::clear_cell_a1()` | `dvc_cell_clear_a1` |
| `Engine::cell_state()` | `dvc_cell_get_state` |
| `Engine::cell_state_a1()` | `dvc_cell_get_state_a1` |
| `Engine::cell_input()` | `dvc_cell_get_input_type`, `dvc_cell_get_input_text` |
| `Engine::cell_input_a1()` | `dvc_cell_get_input_type_a1`, `dvc_cell_get_input_text_a1` |
| `Engine::set_cell_input()` | via typed setters (`set_number`, `set_text`, `set_formula`) |
| `Engine::set_cell_input_a1()` | via typed A1 setters |
| `Engine::set_name_number()` | `dvc_name_set_number` |
| `Engine::set_name_text()` | `dvc_name_set_text` |
| `Engine::set_name_formula()` | `dvc_name_set_formula` |
| `Engine::set_name_input()` | via typed name setters |
| `Engine::clear_name()` | `dvc_name_clear` |
| `Engine::name_input()` | `dvc_name_get_input_type`, `dvc_name_get_input_text` |
| `Engine::recalculate()` | `dvc_recalculate` |
| `Engine::has_volatile_cells()` | `dvc_has_volatile_cells` |
| `Engine::has_externally_invalidated_cells()` | `dvc_has_externally_invalidated_cells` |
| `Engine::invalidate_volatile()` | `dvc_invalidate_volatile` |
| `Engine::has_stream_cells()` | `dvc_has_stream_cells` |
| `Engine::tick_streams()` | `dvc_tick_streams` |
| `Engine::invalidate_udf()` | `dvc_invalidate_udf` |
| `Engine::cell_format()` | `dvc_cell_get_format` |
| `Engine::cell_format_a1()` | `dvc_cell_get_format_a1` |
| `Engine::set_cell_format()` | `dvc_cell_set_format` |
| `Engine::set_cell_format_a1()` | `dvc_cell_set_format_a1` |
| `Engine::spill_anchor_for_cell()` | `dvc_cell_spill_anchor` |
| `Engine::spill_range_for_cell()` | `dvc_cell_spill_range` |
| `Engine::spill_range_for_anchor()` | `dvc_cell_spill_range` (subsumes) |
| `Engine::all_cell_inputs()` | `dvc_cell_iterate` + iterator functions |
| `Engine::all_name_inputs()` | `dvc_name_iterate` + iterator functions |
| `Engine::all_cell_formats()` | `dvc_format_iterate` + iterator functions |
| `Engine::insert_row()` | `dvc_insert_row` |
| `Engine::delete_row()` | `dvc_delete_row` |
| `Engine::insert_col()` | `dvc_insert_col` |
| `Engine::delete_col()` | `dvc_delete_col` |
| `Engine::iteration_config()` | `dvc_engine_get_iteration_config` |
| `Engine::set_iteration_config()` | `dvc_engine_set_iteration_config` |
| `Engine::define_control()` | `dvc_control_define` |
| `Engine::remove_control()` | `dvc_control_remove` |
| `Engine::set_control_value()` | `dvc_control_set_value` |
| `Engine::control_value()` | `dvc_control_get_value` |
| `Engine::control_definition()` | `dvc_control_get_def` |
| `Engine::all_controls()` | `dvc_control_iterate` + iterator functions |
| `Engine::define_chart()` | `dvc_chart_define` |
| `Engine::remove_chart()` | `dvc_chart_remove` |
| `Engine::chart_output()` | `dvc_chart_get_output` + output accessors |
| `Engine::all_charts()` | `dvc_chart_iterate` + iterator functions |
| `Engine::register_udf()` | `dvc_udf_register` |
| `Engine::unregister_udf()` | `dvc_udf_unregister` |
| `Engine::enable_change_tracking()` | `dvc_change_tracking_enable` |
| `Engine::disable_change_tracking()` | `dvc_change_tracking_disable` |
| `Engine::is_change_tracking_enabled()` | `dvc_change_tracking_is_enabled` |
| `Engine::drain_changes()` | `dvc_change_iterate` + iterator functions |
| `PaletteColor::as_name()` | `dvc_palette_color_name` |
| `parse_cell_ref()` | `dvc_parse_cell_ref` |
| Handle-local diagnostic state | `dvc_last_error_message`, `dvc_last_error_kind`, `dvc_last_reject_kind`, `dvc_last_reject_context` |

### Intentionally excluded from C API

| Engine method | Reason |
|--------------|--------|
| `Engine::calc_tree()` | AST internals; stays behind the boundary |
| `Engine::formula_source_a1()` | Subsumed by `dvc_cell_get_input_text_a1` |
| `Engine::dynamic_array_strategy()` | Implementation detail; not a public contract |
| `Engine::set_dynamic_array_strategy()` | Implementation detail |
| `Engine::spill_anchor_for_cell_a1()` | A1 variant not needed; caller can use `dvc_parse_cell_ref` + addr-based function |
| `Engine::spill_range_for_cell_a1()` | Same reasoning |
| `Engine::spill_range_for_anchor()` | Subsumed by `dvc_cell_spill_range` which works for any cell in the range |

### TUI (app.rs) call coverage

Every `self.engine.*` call in `app.rs` maps to a C API function:

| app.rs call | C API function |
|------------|---------------|
| `engine.recalculate()` | `dvc_recalculate` |
| `engine.tick_streams(elapsed)` | `dvc_tick_streams` |
| `engine.has_volatile_cells()` | `dvc_has_volatile_cells` |
| `engine.has_stream_cells()` | `dvc_has_stream_cells` |
| `engine.set_name_number(name, val)` | `dvc_name_set_number` |
| `engine.committed_epoch()` | `dvc_engine_committed_epoch` |
| `engine.cell_state(cell)` | `dvc_cell_get_state` |
| `engine.cell_format(cell)` | `dvc_cell_get_format` |
| `engine.spill_range_for_cell(cell)` | `dvc_cell_spill_range` |
| `engine.cell_input(cell)` | `dvc_cell_get_input_type` + `dvc_cell_get_input_text` |
| `engine.set_recalc_mode(mode)` | `dvc_engine_set_recalc_mode` |
| `engine.bounds()` | `dvc_engine_bounds` |
| `engine.clear_name(name)` | `dvc_name_clear` |
| `engine.clear_cell(cell)` | `dvc_cell_clear` |
| `engine.set_number(cell, n)` | `dvc_cell_set_number` |
| `engine.set_text(cell, t)` | `dvc_cell_set_text` |
| `engine.set_formula(cell, f)` | `dvc_cell_set_formula` |
| `engine.set_cell_format(cell, fmt)` | `dvc_cell_set_format` |
| `engine.spill_anchor_for_cell(cell)` | `dvc_cell_spill_anchor` |
| `engine.set_number_a1(ref, n)` | `dvc_cell_set_number_a1` |
| `engine.set_text_a1(ref, t)` | `dvc_cell_set_text_a1` |
| `engine.set_formula_a1(ref, f)` | `dvc_cell_set_formula_a1` |
| `engine.set_name_formula(name, f)` | `dvc_name_set_formula` |
| `engine.set_name_number(name, n)` | `dvc_name_set_number` |
| `engine.set_name_text(name, t)` | `dvc_name_set_text` |
| `engine.insert_row(at)` | `dvc_insert_row` |
| `engine.delete_row(at)` | `dvc_delete_row` |
| `engine.insert_col(at)` | `dvc_insert_col` |
| `engine.delete_col(at)` | `dvc_delete_col` |

Future TUI calls (after engine-backed controls/charts migration):

| app.rs call (planned) | C API function |
|------------|---------------|
| `engine.define_control(name, def)` | `dvc_control_define` |
| `engine.set_control_value(name, val)` | `dvc_control_set_value` |
| `engine.control_value(name)` | `dvc_control_get_value` |
| `engine.all_controls()` | `dvc_control_iterate` |
| `engine.define_chart(name, def)` | `dvc_chart_define` |
| `engine.chart_output(name)` | `dvc_chart_get_output` |
| `engine.enable_change_tracking()` | `dvc_change_tracking_enable` |
| `engine.drain_changes()` | `dvc_change_iterate` |

### File I/O (lib.rs) call coverage

| dnavisicalc-file call | C API function |
|----------------------|---------------|
| `engine.recalc_mode()` | `dvc_engine_get_recalc_mode` |
| `engine.all_cell_inputs()` | `dvc_cell_iterate` |
| `engine.all_name_inputs()` | `dvc_name_iterate` |
| `engine.all_cell_formats()` | `dvc_format_iterate` |
| `engine.set_recalc_mode(Manual)` | `dvc_engine_set_recalc_mode` |
| `engine.set_cell_input(cell, input)` | `dvc_cell_set_number` / `set_text` / `set_formula` |
| `engine.set_name_input(name, input)` | `dvc_name_set_number` / `set_text` / `set_formula` |
| `engine.set_cell_format(cell, fmt)` | `dvc_cell_set_format` |
| `engine.recalculate()` | `dvc_recalculate` |
| `engine.set_recalc_mode(mode)` | `dvc_engine_set_recalc_mode` |
