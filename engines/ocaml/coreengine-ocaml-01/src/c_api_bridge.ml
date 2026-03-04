(* c_api_bridge.ml - Register OCaml callbacks for the C FFI bridge.
   Each callback is registered with Callback.register so that dvc_engine.c
   can invoke OCaml functions via caml_named_value + caml_callback. *)

(* Global engine store - maps integer handles to engine instances *)
let engines : (int, Engine.t) Hashtbl.t = Hashtbl.create 4
let next_handle = ref 1

(* Global iterator state *)
type cell_iter_state = {
  ci_entries : (int * int * Types.input_type * string) list;
  mutable ci_pos : int;
}

type name_iter_state = {
  ni_entries : (string * Types.input_type * string) list;
  mutable ni_pos : int;
}

type format_iter_state = {
  fi_entries : (int * int * Types.cell_format) list;
  mutable fi_pos : int;
}

type control_iter_state = {
  cti_entries : Engine.control_entry list;
  mutable cti_pos : int;
}

type chart_iter_state = {
  chi_entries : Engine.chart_entry list;
  mutable chi_pos : int;
}

type change_iter_state = {
  chi2_entries : Engine.change_entry list;
  mutable chi2_pos : int;
}

let cell_iters : (int, cell_iter_state) Hashtbl.t = Hashtbl.create 4
let name_iters : (int, name_iter_state) Hashtbl.t = Hashtbl.create 4
let format_iters : (int, format_iter_state) Hashtbl.t = Hashtbl.create 4
let control_iters : (int, control_iter_state) Hashtbl.t = Hashtbl.create 4
let chart_iters : (int, chart_iter_state) Hashtbl.t = Hashtbl.create 4
let change_iters : (int, change_iter_state) Hashtbl.t = Hashtbl.create 4
let chart_outputs : (int, Engine.chart_output) Hashtbl.t = Hashtbl.create 4
let next_iter = ref 1000

let fresh_iter () =
  let h = !next_iter in
  incr next_iter;
  h

(* Engine create: returns (status, handle) *)
let cb_engine_create () =
  match Engine.create_default () with
  | Ok e ->
    let h = !next_handle in
    incr next_handle;
    Hashtbl.replace engines h e;
    (Types.Status.ok, h)
  | Error s -> (s, 0)

let cb_engine_create_with_bounds max_columns max_rows =
  match Engine.create_with_bounds ~max_columns ~max_rows with
  | Ok e ->
    let h = !next_handle in
    incr next_handle;
    Hashtbl.replace engines h e;
    (Types.Status.ok, h)
  | Error s -> (s, 0)

let cb_engine_destroy handle =
  Hashtbl.remove engines handle;
  Types.Status.ok

let cb_engine_clear handle =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.clear e

let cb_engine_bounds handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0)
  | Some e ->
    let (mc, mr) = Engine.bounds e in
    (Types.Status.ok, mc, mr)

let cb_get_recalc_mode handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, Engine.get_recalc_mode e)

let cb_set_recalc_mode handle mode =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.set_recalc_mode e mode

let cb_committed_epoch handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, Engine.committed_epoch e)

let cb_stabilized_epoch handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, Engine.stabilized_epoch e)

let cb_is_stable handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.is_stable e then 1 else 0)

let cb_cell_set_number handle col row value =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.set_number e (Address.make ~col ~row) value

let cb_cell_set_text handle col row text =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.set_text e (Address.make ~col ~row) text

let cb_cell_set_formula handle col row formula =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.set_formula e (Address.make ~col ~row) formula

let cb_cell_clear handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.cell_clear e (Address.make ~col ~row)

(* Returns (status, value_type, number, bool_val, error_kind, value_epoch, stale) *)
let cb_cell_get_state handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0.0, 0, 0, 0, 0)
  | Some e ->
    match Engine.get_state e (Address.make ~col ~row) with
    | Error s -> (s, 0, 0.0, 0, 0, 0, 0)
    | Ok st ->
      (Types.Status.ok,
       Types.value_type_int st.value,
       Types.value_number st.value,
       Types.value_bool_val st.value,
       Types.value_error_kind st.value,
       st.value_epoch,
       if st.stale then 1 else 0)

let cb_cell_get_text handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, "")
  | Some e ->
    match Engine.get_text e (Address.make ~col ~row) with
    | Error s -> (s, "")
    | Ok t -> (Types.Status.ok, t)

let cb_cell_get_input_type handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    match Engine.get_input_type e (Address.make ~col ~row) with
    | Error s -> (s, 0)
    | Ok it -> (Types.Status.ok, Types.input_type_to_int it)

let cb_cell_get_input_text handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, "")
  | Some e ->
    match Engine.get_input_text e (Address.make ~col ~row) with
    | Error s -> (s, "")
    | Ok t -> (Types.Status.ok, t)

let cb_name_set_number handle name value =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.name_set_number e name value

let cb_name_set_text handle name text =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.name_set_text e name text

let cb_name_set_formula handle name formula =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.name_set_formula e name formula

let cb_name_clear handle name =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.name_clear e name

let cb_name_get_input_type handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    match Engine.name_get_input_type e name with
    | Error s -> (s, 0)
    | Ok it -> (Types.Status.ok, Types.input_type_to_int it)

let cb_name_get_input_text handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, "")
  | Some e ->
    match Engine.name_get_input_text e name with
    | Error s -> (s, "")
    | Ok t -> (Types.Status.ok, t)

let cb_recalculate handle =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.recalculate e

let cb_has_volatile handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.has_volatile_cells e then 1 else 0)

let cb_has_ext_invalidated handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.has_externally_invalidated_cells e then 1 else 0)

let cb_invalidate_volatile handle =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.invalidate_volatile e

let cb_has_streams handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.has_stream_cells e then 1 else 0)

let cb_tick_streams handle elapsed =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let (s, adv) = Engine.tick_streams e elapsed in
    (s, if adv then 1 else 0)

let cb_invalidate_udf handle name =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.invalidate_udf e name

let cb_cell_get_format handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, false, 0, false, false, -1, -1)
  | Some e ->
    match Engine.get_format e (Address.make ~col ~row) with
    | Error s -> (s, false, 0, false, false, -1, -1)
    | Ok fmt -> (Types.Status.ok, fmt.has_decimals, fmt.decimals, fmt.bold, fmt.italic, fmt.fg, fmt.bg)

let cb_cell_set_format handle col row has_dec dec bold italic fg bg =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e ->
    Engine.set_format e (Address.make ~col ~row)
      { Types.has_decimals = has_dec; decimals = dec; bold; italic; fg; bg }

let cb_spill_role handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    match Engine.spill_role e (Address.make ~col ~row) with
    | Error s -> (s, 0)
    | Ok r -> (Types.Status.ok, r)

let cb_spill_anchor handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0, 0)
  | Some e ->
    match Engine.spill_anchor e (Address.make ~col ~row) with
    | Error s -> (s, 0, 0, 0)
    | Ok None -> (Types.Status.ok, 0, 0, 0)
    | Ok (Some (c, r)) -> (Types.Status.ok, 1, c, r)

let cb_spill_range handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0, 0, 0, 0)
  | Some e ->
    match Engine.spill_range e (Address.make ~col ~row) with
    | Error s -> (s, 0, 0, 0, 0, 0)
    | Ok None -> (Types.Status.ok, 0, 0, 0, 0, 0)
    | Ok (Some (c1, r1, c2, r2)) -> (Types.Status.ok, 1, c1, r1, c2, r2)

(* Cell iterator *)
let cb_cell_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.cell_iterate e in
    let h = fresh_iter () in
    Hashtbl.replace cell_iters h { ci_entries = entries; ci_pos = 0 };
    (Types.Status.ok, h)

let cb_cell_iter_next iter_h =
  match Hashtbl.find_opt cell_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0, 0, 1)
  | Some st ->
    let entries = st.ci_entries in
    let len = List.length entries in
    if st.ci_pos >= len then
      (Types.Status.ok, 0, 0, 0, 1)
    else begin
      let (col, row, it, _) = List.nth entries st.ci_pos in
      st.ci_pos <- st.ci_pos + 1;
      (Types.Status.ok, col, row, Types.input_type_to_int it, 0)
    end

let cb_cell_iter_get_text iter_h =
  match Hashtbl.find_opt cell_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some st ->
    if st.ci_pos <= 0 then (Types.Status.ok, "")
    else begin
      let (_, _, _, text) = List.nth st.ci_entries (st.ci_pos - 1) in
      (Types.Status.ok, text)
    end

let cb_cell_iter_destroy iter_h =
  Hashtbl.remove cell_iters iter_h;
  Types.Status.ok

(* Name iterator *)
let cb_name_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.name_iterate e in
    let h = fresh_iter () in
    Hashtbl.replace name_iters h { ni_entries = entries; ni_pos = 0 };
    (Types.Status.ok, h)

let cb_name_iter_next iter_h =
  match Hashtbl.find_opt name_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "", 0, 1)
  | Some st ->
    let entries = st.ni_entries in
    let len = List.length entries in
    if st.ni_pos >= len then
      (Types.Status.ok, "", 0, 1)
    else begin
      let (name, it, _) = List.nth entries st.ni_pos in
      st.ni_pos <- st.ni_pos + 1;
      (Types.Status.ok, name, Types.input_type_to_int it, 0)
    end

let cb_name_iter_get_text iter_h =
  match Hashtbl.find_opt name_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some st ->
    if st.ni_pos <= 0 then (Types.Status.ok, "")
    else begin
      let (_, _, text) = List.nth st.ni_entries (st.ni_pos - 1) in
      (Types.Status.ok, text)
    end

let cb_name_iter_destroy iter_h =
  Hashtbl.remove name_iters iter_h;
  Types.Status.ok

(* Format iterator *)
let cb_format_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.format_iterate e in
    let h = fresh_iter () in
    Hashtbl.replace format_iters h { fi_entries = entries; fi_pos = 0 };
    (Types.Status.ok, h)

let cb_format_iter_next iter_h =
  match Hashtbl.find_opt format_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0, false, 0, false, false, -1, -1, 1)
  | Some st ->
    let entries = st.fi_entries in
    let len = List.length entries in
    if st.fi_pos >= len then
      (Types.Status.ok, 0, 0, false, 0, false, false, -1, -1, 1)
    else begin
      let (col, row, fmt) = List.nth entries st.fi_pos in
      st.fi_pos <- st.fi_pos + 1;
      (Types.Status.ok, col, row, fmt.has_decimals, fmt.decimals, fmt.bold, fmt.italic, fmt.fg, fmt.bg, 0)
    end

let cb_format_iter_destroy iter_h =
  Hashtbl.remove format_iters iter_h;
  Types.Status.ok

(* Structural ops *)
let cb_insert_row handle at =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.insert_row e at

let cb_delete_row handle at =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.delete_row e at

let cb_insert_col handle at =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.insert_col e at

let cb_delete_col handle at =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.delete_col e at

(* Iteration config *)
let cb_get_iter_config handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, false, 0, 0.0)
  | Some e ->
    let cfg = Engine.get_iteration_config e in
    (Types.Status.ok, cfg.enabled, cfg.max_iterations, cfg.convergence_tolerance)

let cb_set_iter_config handle enabled max_iter tol =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e ->
    Engine.set_iteration_config e
      { Types.enabled; max_iterations = max_iter; convergence_tolerance = tol }

(* Controls *)
let cb_control_define handle name kind min max step =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e ->
    Engine.control_define e name
      { Types.kind = Types.control_kind_of_int kind; min; max; step }

let cb_control_remove handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.control_remove e name then 1 else 0)

let cb_control_set_value handle name value =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.control_set_value e name value

let cb_control_get_value handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0.0, 0)
  | Some e ->
    match Engine.control_get_value e name with
    | Ok v -> (Types.Status.ok, v, 1)
    | Error _ -> (Types.Status.ok, 0.0, 0)

let cb_control_get_def handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0.0, 0.0, 0.0, 0)
  | Some e ->
    match Engine.control_get_def e name with
    | Ok def -> (Types.Status.ok, Types.control_kind_to_int def.kind, def.min, def.max, def.step, 1)
    | Error _ -> (Types.Status.ok, 0, 0.0, 0.0, 0.0, 0)

let cb_control_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.control_list e in
    let h = fresh_iter () in
    Hashtbl.replace control_iters h { cti_entries = entries; cti_pos = 0 };
    (Types.Status.ok, h)

let cb_control_iter_next iter_h =
  match Hashtbl.find_opt control_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "", 0, 0.0, 0.0, 0.0, 0.0, 1)
  | Some st ->
    let entries = st.cti_entries in
    let len = List.length entries in
    if st.cti_pos >= len then
      (Types.Status.ok, "", 0, 0.0, 0.0, 0.0, 0.0, 1)
    else begin
      let entry = List.nth entries st.cti_pos in
      st.cti_pos <- st.cti_pos + 1;
      (Types.Status.ok, entry.ce_name,
       Types.control_kind_to_int entry.ce_def.kind,
       entry.ce_def.min, entry.ce_def.max, entry.ce_def.step,
       entry.ce_value, 0)
    end

let cb_control_iter_destroy iter_h =
  Hashtbl.remove control_iters iter_h;
  Types.Status.ok

(* Charts *)
let cb_chart_define handle name sc1 sr1 sc2 sr2 =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e ->
    Engine.chart_define e name
      { Types.source_start_col = sc1; source_start_row = sr1;
        source_end_col = sc2; source_end_row = sr2 }

let cb_chart_remove handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.chart_remove e name then 1 else 0)

let cb_chart_get_output handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0)
  | Some e ->
    match Engine.chart_get_output e name with
    | None -> (Types.Status.ok, 0, 0)
    | Some output ->
      let h = fresh_iter () in
      Hashtbl.replace chart_outputs h output;
      (Types.Status.ok, 1, h)

let cb_chart_series_count output_h =
  match Hashtbl.find_opt chart_outputs output_h with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some o -> (Types.Status.ok, Array.length o.co_series_names)

let cb_chart_label_count output_h =
  match Hashtbl.find_opt chart_outputs output_h with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some o -> (Types.Status.ok, Array.length o.co_labels)

let cb_chart_label output_h idx =
  match Hashtbl.find_opt chart_outputs output_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some o ->
    if idx >= Array.length o.co_labels then (Types.Status.err_out_of_bounds, "")
    else (Types.Status.ok, o.co_labels.(idx))

let cb_chart_series_name output_h idx =
  match Hashtbl.find_opt chart_outputs output_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some o ->
    if idx >= Array.length o.co_series_names then (Types.Status.err_out_of_bounds, "")
    else (Types.Status.ok, o.co_series_names.(idx))

let cb_chart_series_values output_h idx =
  match Hashtbl.find_opt chart_outputs output_h with
  | None -> (Types.Status.err_null_pointer, [||])
  | Some o ->
    if idx >= Array.length o.co_series_values then (Types.Status.err_out_of_bounds, [||])
    else (Types.Status.ok, o.co_series_values.(idx))

let cb_chart_output_free output_h =
  Hashtbl.remove chart_outputs output_h;
  Types.Status.ok

let cb_chart_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.chart_list e in
    let h = fresh_iter () in
    Hashtbl.replace chart_iters h { chi_entries = entries; chi_pos = 0 };
    (Types.Status.ok, h)

let cb_chart_iter_next iter_h =
  match Hashtbl.find_opt chart_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "", 0, 0, 0, 0, 1)
  | Some st ->
    let entries = st.chi_entries in
    let len = List.length entries in
    if st.chi_pos >= len then
      (Types.Status.ok, "", 0, 0, 0, 0, 1)
    else begin
      let entry = List.nth entries st.chi_pos in
      st.chi_pos <- st.chi_pos + 1;
      (Types.Status.ok, entry.ch_name,
       entry.ch_def.source_start_col, entry.ch_def.source_start_row,
       entry.ch_def.source_end_col, entry.ch_def.source_end_row, 0)
    end

let cb_chart_iter_destroy iter_h =
  Hashtbl.remove chart_iters iter_h;
  Types.Status.ok

(* UDF - note: actual callback goes through C bridge *)
let cb_udf_register handle name _volatility_int =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some _e -> Types.Status.ok

let cb_udf_unregister handle name =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.udf_unregister e name then 1 else 0)

(* Change tracking *)
let cb_change_enable handle =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.change_tracking_enable e

let cb_change_disable handle =
  match Hashtbl.find_opt engines handle with
  | None -> Types.Status.err_null_pointer
  | Some e -> Engine.change_tracking_disable e

let cb_change_is_enabled handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, if Engine.change_tracking_is_enabled e then 1 else 0)

let cb_change_iterate handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e ->
    let entries = Engine.drain_changes e in
    let h = fresh_iter () in
    Hashtbl.replace change_iters h { chi2_entries = entries; chi2_pos = 0 };
    (Types.Status.ok, h)

let cb_change_iter_next iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0, 1)
  | Some st ->
    let entries = st.chi2_entries in
    let len = List.length entries in
    if st.chi2_pos >= len then
      (Types.Status.ok, 0, 0, 1)
    else begin
      let entry = List.nth entries st.chi2_pos in
      st.chi2_pos <- st.chi2_pos + 1;
      (Types.Status.ok, Types.change_type_to_int entry.change_type, entry.epoch, 0)
    end

let cb_change_get_cell iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0)
  | Some st ->
    if st.chi2_pos <= 0 then (Types.Status.ok, 0, 0)
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      (Types.Status.ok, entry.cell_col, entry.cell_row)
    end

let cb_change_get_name iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some st ->
    if st.chi2_pos <= 0 then (Types.Status.ok, "")
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      (Types.Status.ok, entry.name)
    end

let cb_change_get_chart_name iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, "")
  | Some st ->
    if st.chi2_pos <= 0 then (Types.Status.ok, "")
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      (Types.Status.ok, entry.chart_name)
    end

let cb_change_get_spill iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
  | Some st ->
    if st.chi2_pos <= 0 then (Types.Status.ok, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      let (oc1, or1, oc2, or2, ho) = match entry.spill_old_range with
        | Some (c1, r1, c2, r2) -> (c1, r1, c2, r2, 1) | None -> (0, 0, 0, 0, 0) in
      let (nc1, nr1, nc2, nr2, hn) = match entry.spill_new_range with
        | Some (c1, r1, c2, r2) -> (c1, r1, c2, r2, 1) | None -> (0, 0, 0, 0, 0) in
      (Types.Status.ok, entry.spill_anchor_col, entry.spill_anchor_row,
       oc1, or1, oc2, or2, ho, nc1, nr1, nc2, nr2)
      |> fun (s, ac, ar, _oc1, _or1, _oc2, _or2, _ho, _nc1, _nr1, _nc2, _nr2) ->
        (s, ac, ar, ho, oc1, or1, oc2, or2, hn, nc1, nr1, nc2)
    end

let cb_change_get_format iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, 0,
             false, 0, false, false, -1, -1,
             false, 0, false, false, -1, -1)
  | Some st ->
    if st.chi2_pos <= 0 then
      (Types.Status.ok, 0, 0,
       false, 0, false, false, -1, -1,
       false, 0, false, false, -1, -1)
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      let o = entry.old_fmt and n = entry.new_fmt in
      (Types.Status.ok, entry.fmt_col, entry.fmt_row,
       o.has_decimals, o.decimals, o.bold, o.italic, o.fg, o.bg,
       n.has_decimals, n.decimals, n.bold, n.italic, n.fg, n.bg)
    end

let cb_change_get_diagnostic iter_h =
  match Hashtbl.find_opt change_iters iter_h with
  | None -> (Types.Status.err_null_pointer, 0, "")
  | Some st ->
    if st.chi2_pos <= 0 then (Types.Status.ok, 0, "")
    else begin
      let entry = List.nth st.chi2_entries (st.chi2_pos - 1) in
      (Types.Status.ok, entry.diag_code, entry.diag_message)
    end

let cb_change_iter_destroy iter_h =
  Hashtbl.remove change_iters iter_h;
  Types.Status.ok

(* Error/reject info *)
let cb_last_error_message handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, "")
  | Some e -> (Types.Status.ok, Engine.last_error_message e)

let cb_last_error_kind handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, Engine.last_error_kind e)

let cb_last_reject_kind handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0)
  | Some e -> (Types.Status.ok, Engine.last_reject_kind e)

let cb_last_reject_context handle =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
  | Some e ->
    let ctx = Engine.last_reject_context e in
    (Types.Status.ok, ctx.reject_kind,
     Types.structural_op_to_int ctx.op_kind, ctx.op_index,
     (if ctx.has_cell then 1 else 0), ctx.cell_col, ctx.cell_row,
     (if ctx.has_range then 1 else 0),
     ctx.range_start_col, ctx.range_start_row,
     ctx.range_end_col, ctx.range_end_row)

let cb_cell_error_message handle col row =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, "")
  | Some e ->
    match Engine.error_message e (Address.make ~col ~row) with
    | Error s -> (s, "")
    | Ok m -> (Types.Status.ok, m)

let cb_parse_cell_ref handle ref_str =
  match Hashtbl.find_opt engines handle with
  | None -> (Types.Status.err_null_pointer, 0, 0)
  | Some e ->
    match Engine.parse_cell_ref e ref_str with
    | Ok (c, r) -> (Types.Status.ok, c, r)
    | Error s -> (s, 0, 0)

let cb_palette_color_name color =
  if color >= 0 && color < 16 then
    (Types.Status.ok, Address.palette_names.(color))
  else
    (Types.Status.err_invalid_argument, "")

(* Register all callbacks *)
let () =
  Callback.register "dvc_engine_create" cb_engine_create;
  Callback.register "dvc_engine_create_with_bounds" cb_engine_create_with_bounds;
  Callback.register "dvc_engine_destroy" cb_engine_destroy;
  Callback.register "dvc_engine_clear" cb_engine_clear;
  Callback.register "dvc_engine_bounds" cb_engine_bounds;
  Callback.register "dvc_get_recalc_mode" cb_get_recalc_mode;
  Callback.register "dvc_set_recalc_mode" cb_set_recalc_mode;
  Callback.register "dvc_committed_epoch" cb_committed_epoch;
  Callback.register "dvc_stabilized_epoch" cb_stabilized_epoch;
  Callback.register "dvc_is_stable" cb_is_stable;
  Callback.register "dvc_cell_set_number" cb_cell_set_number;
  Callback.register "dvc_cell_set_text" cb_cell_set_text;
  Callback.register "dvc_cell_set_formula" cb_cell_set_formula;
  Callback.register "dvc_cell_clear" cb_cell_clear;
  Callback.register "dvc_cell_get_state" cb_cell_get_state;
  Callback.register "dvc_cell_get_text" cb_cell_get_text;
  Callback.register "dvc_cell_get_input_type" cb_cell_get_input_type;
  Callback.register "dvc_cell_get_input_text" cb_cell_get_input_text;
  Callback.register "dvc_name_set_number" cb_name_set_number;
  Callback.register "dvc_name_set_text" cb_name_set_text;
  Callback.register "dvc_name_set_formula" cb_name_set_formula;
  Callback.register "dvc_name_clear" cb_name_clear;
  Callback.register "dvc_name_get_input_type" cb_name_get_input_type;
  Callback.register "dvc_name_get_input_text" cb_name_get_input_text;
  Callback.register "dvc_recalculate" cb_recalculate;
  Callback.register "dvc_has_volatile" cb_has_volatile;
  Callback.register "dvc_has_ext_invalidated" cb_has_ext_invalidated;
  Callback.register "dvc_invalidate_volatile" cb_invalidate_volatile;
  Callback.register "dvc_has_streams" cb_has_streams;
  Callback.register "dvc_tick_streams" cb_tick_streams;
  Callback.register "dvc_invalidate_udf" cb_invalidate_udf;
  Callback.register "dvc_cell_get_format" cb_cell_get_format;
  Callback.register "dvc_cell_set_format" cb_cell_set_format;
  Callback.register "dvc_spill_role" cb_spill_role;
  Callback.register "dvc_spill_anchor" cb_spill_anchor;
  Callback.register "dvc_spill_range" cb_spill_range;
  Callback.register "dvc_cell_iterate" cb_cell_iterate;
  Callback.register "dvc_cell_iter_next" cb_cell_iter_next;
  Callback.register "dvc_cell_iter_get_text" cb_cell_iter_get_text;
  Callback.register "dvc_cell_iter_destroy" cb_cell_iter_destroy;
  Callback.register "dvc_name_iterate" cb_name_iterate;
  Callback.register "dvc_name_iter_next" cb_name_iter_next;
  Callback.register "dvc_name_iter_get_text" cb_name_iter_get_text;
  Callback.register "dvc_name_iter_destroy" cb_name_iter_destroy;
  Callback.register "dvc_format_iterate" cb_format_iterate;
  Callback.register "dvc_format_iter_next" cb_format_iter_next;
  Callback.register "dvc_format_iter_destroy" cb_format_iter_destroy;
  Callback.register "dvc_insert_row" cb_insert_row;
  Callback.register "dvc_delete_row" cb_delete_row;
  Callback.register "dvc_insert_col" cb_insert_col;
  Callback.register "dvc_delete_col" cb_delete_col;
  Callback.register "dvc_get_iter_config" cb_get_iter_config;
  Callback.register "dvc_set_iter_config" cb_set_iter_config;
  Callback.register "dvc_control_define" cb_control_define;
  Callback.register "dvc_control_remove" cb_control_remove;
  Callback.register "dvc_control_set_value" cb_control_set_value;
  Callback.register "dvc_control_get_value" cb_control_get_value;
  Callback.register "dvc_control_get_def" cb_control_get_def;
  Callback.register "dvc_control_iterate" cb_control_iterate;
  Callback.register "dvc_control_iter_next" cb_control_iter_next;
  Callback.register "dvc_control_iter_destroy" cb_control_iter_destroy;
  Callback.register "dvc_chart_define" cb_chart_define;
  Callback.register "dvc_chart_remove" cb_chart_remove;
  Callback.register "dvc_chart_get_output" cb_chart_get_output;
  Callback.register "dvc_chart_series_count" cb_chart_series_count;
  Callback.register "dvc_chart_label_count" cb_chart_label_count;
  Callback.register "dvc_chart_label" cb_chart_label;
  Callback.register "dvc_chart_series_name" cb_chart_series_name;
  Callback.register "dvc_chart_series_values" cb_chart_series_values;
  Callback.register "dvc_chart_output_free" cb_chart_output_free;
  Callback.register "dvc_chart_iterate" cb_chart_iterate;
  Callback.register "dvc_chart_iter_next" cb_chart_iter_next;
  Callback.register "dvc_chart_iter_destroy" cb_chart_iter_destroy;
  Callback.register "dvc_udf_register" cb_udf_register;
  Callback.register "dvc_udf_unregister" cb_udf_unregister;
  Callback.register "dvc_change_enable" cb_change_enable;
  Callback.register "dvc_change_disable" cb_change_disable;
  Callback.register "dvc_change_is_enabled" cb_change_is_enabled;
  Callback.register "dvc_change_iterate" cb_change_iterate;
  Callback.register "dvc_change_iter_next" cb_change_iter_next;
  Callback.register "dvc_change_get_cell" cb_change_get_cell;
  Callback.register "dvc_change_get_name" cb_change_get_name;
  Callback.register "dvc_change_get_chart_name" cb_change_get_chart_name;
  Callback.register "dvc_change_get_spill" cb_change_get_spill;
  Callback.register "dvc_change_get_format" cb_change_get_format;
  Callback.register "dvc_change_get_diagnostic" cb_change_get_diagnostic;
  Callback.register "dvc_change_iter_destroy" cb_change_iter_destroy;
  Callback.register "dvc_last_error_message" cb_last_error_message;
  Callback.register "dvc_last_error_kind" cb_last_error_kind;
  Callback.register "dvc_last_reject_kind" cb_last_reject_kind;
  Callback.register "dvc_last_reject_context" cb_last_reject_context;
  Callback.register "dvc_cell_error_message" cb_cell_error_message;
  Callback.register "dvc_parse_cell_ref" cb_parse_cell_ref;
  Callback.register "dvc_palette_color_name" cb_palette_color_name
