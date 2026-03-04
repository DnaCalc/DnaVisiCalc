(* engine.ml - Main engine implementing the DVC API.
   All business logic lives here in OCaml. The C bridge (dvc_engine.c)
   only handles FFI marshaling. *)

type change_entry = {
  change_type : Types.change_type;
  epoch : int;
  (* cell value change *)
  cell_col : int;
  cell_row : int;
  (* name value change *)
  name : string;
  (* chart output change *)
  chart_name : string;
  (* spill region change *)
  spill_anchor_col : int;
  spill_anchor_row : int;
  spill_old_range : (int * int * int * int) option;
  spill_new_range : (int * int * int * int) option;
  (* format change *)
  fmt_col : int;
  fmt_row : int;
  old_fmt : Types.cell_format;
  new_fmt : Types.cell_format;
  (* diagnostic *)
  diag_code : int;
  diag_message : string;
}

type udf_entry = {
  udf_name : string;
  udf_callback : Types.value array -> Types.value;
  udf_volatility : Types.volatility;
}

type name_entry = {
  ne_name : string;
  mutable ne_input : Sheet.cell_input;
}

type control_entry = {
  ce_name : string;
  mutable ce_def : Types.control_def;
  mutable ce_value : float;
}

type chart_entry = {
  ch_name : string;
  mutable ch_def : Types.chart_def;
}

type chart_output = {
  co_labels : string array;
  co_series_names : string array;
  co_series_values : float array array;
}

type t = {
  sheet : Sheet.t;
  mutable recalc_mode : int;
  mutable committed_epoch : int;
  mutable stabilized_epoch : int;
  mutable iter_config : Types.iteration_config;
  mutable names : name_entry list;
  mutable controls : control_entry list;
  mutable charts : chart_entry list;
  mutable udfs : udf_entry list;
  mutable change_tracking : bool;
  mutable changes : change_entry list;
  mutable last_error_kind : int;
  mutable last_error_message : string;
  mutable last_reject_kind : int;
  mutable last_reject_context : Types.reject_context;
  dep_graph : Incremental_runtime.dep_graph;
  incr_rt : Incremental_runtime.runtime;
  mutable incremental_runtime_available : bool;
  mutable dep_graph_dirty : bool;
  mutable feature_flags_dirty : bool;
  mutable has_dynamic_array_features_cache : bool;
  mutable has_volatile_or_stream_features_cache : bool;
  mutable cycle_nodes_cache : bool array option;
  mutable has_cycles_cache : bool;
  mutable stream_layout_dirty : bool;
  mutable dirty_input_cells_rev : int list;
  mutable compiled_formula_src : string option array;
  mutable compiled_formula_prog : Eval.compiled_expr option array;
  rand_counter : int ref;
}

let default_max_columns = 63
let default_max_rows = 254

let create_default () =
  let sheet = Sheet.create ~max_columns:default_max_columns ~max_rows:default_max_rows in
  let n = Sheet.cell_count sheet in
  Ok {
    sheet;
    recalc_mode = Types.Recalc_mode.automatic;
    committed_epoch = 0;
    stabilized_epoch = 0;
    iter_config = { enabled = false; max_iterations = 100; convergence_tolerance = 0.001 };
    names = []; controls = []; charts = []; udfs = [];
    change_tracking = false; changes = [];
    last_error_kind = 0; last_error_message = "";
    last_reject_kind = 0; last_reject_context = Types.empty_reject_context;
    dep_graph = Incremental_runtime.create_graph n;
    incr_rt = Incremental_runtime.create_runtime n;
    incremental_runtime_available = true;
    dep_graph_dirty = true;
    feature_flags_dirty = true;
    has_dynamic_array_features_cache = false;
    has_volatile_or_stream_features_cache = false;
    cycle_nodes_cache = None;
    has_cycles_cache = false;
    stream_layout_dirty = true;
    dirty_input_cells_rev = [];
    compiled_formula_src = Array.make n None;
    compiled_formula_prog = Array.make n None;
    rand_counter = ref 0;
  }

let create_with_bounds ~max_columns ~max_rows =
  if max_columns <= 0 || max_rows <= 0 || max_columns > 1024 || max_rows > 4096 then
    Error Types.Status.err_invalid_argument
  else begin
    let sheet = Sheet.create ~max_columns ~max_rows in
    let n = Sheet.cell_count sheet in
    Ok {
      sheet;
      recalc_mode = Types.Recalc_mode.automatic;
      committed_epoch = 0;
      stabilized_epoch = 0;
      iter_config = { enabled = false; max_iterations = 100; convergence_tolerance = 0.001 };
      names = []; controls = []; charts = []; udfs = [];
      change_tracking = false; changes = [];
      last_error_kind = 0; last_error_message = "";
      last_reject_kind = 0; last_reject_context = Types.empty_reject_context;
      dep_graph = Incremental_runtime.create_graph n;
      incr_rt = Incremental_runtime.create_runtime n;
      incremental_runtime_available = true;
      dep_graph_dirty = true;
      feature_flags_dirty = true;
      has_dynamic_array_features_cache = false;
      has_volatile_or_stream_features_cache = false;
      cycle_nodes_cache = None;
      has_cycles_cache = false;
      stream_layout_dirty = true;
      dirty_input_cells_rev = [];
      compiled_formula_src = Array.make n None;
      compiled_formula_prog = Array.make n None;
      rand_counter = ref 0;
    }
  end

let clear_status e =
  e.last_error_kind <- 0;
  e.last_error_message <- "";
  e.last_reject_kind <- 0;
  e.last_reject_context <- Types.empty_reject_context

let push_change e entry =
  if e.change_tracking then
    e.changes <- entry :: e.changes

let push_cell_change e col row =
  if e.change_tracking then
    push_change e {
      change_type = Types.Change_cell_value; epoch = e.committed_epoch;
      cell_col = col; cell_row = row; name = ""; chart_name = "";
      spill_anchor_col = 0; spill_anchor_row = 0;
      spill_old_range = None; spill_new_range = None;
      fmt_col = 0; fmt_row = 0;
      old_fmt = Types.default_format; new_fmt = Types.default_format;
      diag_code = 0; diag_message = "";
    }

let push_name_change e name =
  if e.change_tracking then
    push_change e {
      change_type = Types.Change_name_value; epoch = e.committed_epoch;
      cell_col = 0; cell_row = 0; name; chart_name = "";
      spill_anchor_col = 0; spill_anchor_row = 0;
      spill_old_range = None; spill_new_range = None;
      fmt_col = 0; fmt_row = 0;
      old_fmt = Types.default_format; new_fmt = Types.default_format;
      diag_code = 0; diag_message = "";
    }

let push_diagnostic e code message =
  if e.change_tracking then
    push_change e {
      change_type = Types.Change_diagnostic; epoch = e.committed_epoch;
      cell_col = 0; cell_row = 0; name = ""; chart_name = "";
      spill_anchor_col = 0; spill_anchor_row = 0;
      spill_old_range = None; spill_new_range = None;
      fmt_col = 0; fmt_row = 0;
      old_fmt = Types.default_format; new_fmt = Types.default_format;
      diag_code = code; diag_message = message;
    }

(* Build eval context *)
let make_eval_ctx e col row names_for_eval udfs_for_eval =
  {
    Eval.sheet = e.sheet;
    self_col = col;
    self_row = row;
    committed_epoch = e.committed_epoch;
    rand_counter = e.rand_counter;
    let_bindings = [];
    names = names_for_eval;
    udfs = udfs_for_eval;
  }

(* Evaluate a single cell and write computed results *)
let eval_cell e i names_for_eval udfs_for_eval =
  let col = (i mod e.sheet.max_columns) + 1 in
  let row = (i / e.sheet.max_columns) + 1 in
  let comp = e.sheet.computed.(i) in
  match e.sheet.cells.(i) with
  | Sheet.Cell_empty ->
    comp.value <- Types.Blank;
    comp.text <- "";
    comp.error_message <- ""
  | Sheet.Cell_number n ->
    comp.value <- Types.Number n;
    comp.text <- "";
    comp.error_message <- ""
  | Sheet.Cell_text s ->
    comp.value <- Types.Text s;
    comp.text <- s;
    comp.error_message <- ""
  | Sheet.Cell_formula formula ->
    let ctx = make_eval_ctx e col row names_for_eval udfs_for_eval in
    let cache_fresh =
      match e.compiled_formula_src.(i) with
      | Some cached -> String.equal cached formula
      | None -> false
    in
    if not cache_fresh then begin
      e.compiled_formula_src.(i) <- Some formula;
      e.compiled_formula_prog.(i) <- Eval.try_compile formula
    end;
    let result =
      match e.compiled_formula_prog.(i) with
      | Some compiled -> Eval.eval_compiled ctx compiled 0
      | None -> Eval.eval_expr ctx formula 0
    in
    match result with
    | Eval.Arr arr ->
      (* Dynamic array - spill *)
      let rows = Array.length arr in
      let cols = if rows > 0 then Array.length arr.(0) else 0 in
      (* Check if spill region fits *)
      let fits = ref true in
      for r = 0 to rows - 1 do
        for c = 0 to cols - 1 do
          let tc = col + c and tr = row + r in
          if not (Sheet.in_bounds e.sheet tc tr) then fits := false
          else if r > 0 || c > 0 then begin
            let ti = Sheet.idx e.sheet tc tr in
            match e.sheet.cells.(ti) with
            | Sheet.Cell_empty -> ()
            | _ -> fits := false
          end
        done
      done;
      if !fits then begin
        (* Write anchor *)
        comp.value <- Eval.result_to_value arr.(0).(0);
        comp.spill_role <- 1;
        comp.spill_range_start_col <- col;
        comp.spill_range_start_row <- row;
        comp.spill_range_end_col <- col + cols - 1;
        comp.spill_range_end_row <- row + rows - 1;
        comp.value_epoch <- e.committed_epoch;
        comp.error_message <- "";
        (* Write spill members *)
        for r = 0 to rows - 1 do
          for c = 0 to cols - 1 do
            if r > 0 || c > 0 then begin
              let tc = col + c and tr = row + r in
              let ti = Sheet.idx e.sheet tc tr in
              let mc = e.sheet.computed.(ti) in
              mc.value <- Eval.result_to_value arr.(r).(c);
              mc.spill_role <- 2;
              mc.spill_anchor_col <- col;
              mc.spill_anchor_row <- row;
              mc.spill_range_start_col <- col;
              mc.spill_range_start_row <- row;
              mc.spill_range_end_col <- col + cols - 1;
              mc.spill_range_end_row <- row + rows - 1;
              mc.value_epoch <- e.committed_epoch;
              mc.error_message <- "";
              push_cell_change e tc tr
            end
          done
        done;
        push_cell_change e col row
      end else begin
        comp.value <- Types.Error (Types.Err_spill, "#SPILL!");
        comp.spill_role <- 0;
        comp.value_epoch <- e.committed_epoch;
        comp.error_message <- "#SPILL!";
        push_cell_change e col row
      end
    | _ ->
      comp.value <- Eval.result_to_value result;
      comp.spill_role <- 0;
      comp.value_epoch <- e.committed_epoch;
      (match result with
       | Eval.Txt s -> comp.text <- s
       | Eval.Err (_, m) -> comp.error_message <- m
       | _ -> comp.text <- ""; comp.error_message <- "");
      push_cell_change e col row

(* Setup stream state for STREAM formulas *)
let setup_streams e =
  let n = Sheet.cell_count e.sheet in
  for i = 0 to n - 1 do
    match e.sheet.cells.(i) with
    | Sheet.Cell_formula f ->
      if Parser.formula_has_function f "STREAM" then begin
        let st = e.sheet.streams.(i) in
        if not st.active then begin
          (* Parse period from STREAM(period) *)
          let upper = String.uppercase_ascii f in
          let period = try
            let sp = String.index upper '(' in
            let ep = String.index_from upper sp ')' in
            float_of_string (String.sub upper (sp + 1) (ep - sp - 1))
          with _ -> 1.0 in
          st.active <- true;
          st.period <- period;
          st.elapsed <- 0.0;
          st.counter <- 0
        end
      end
    | _ ->
      let st = e.sheet.streams.(i) in
      st.active <- false
  done

let mark_dependency_layout_dirty e =
  e.dep_graph_dirty <- true;
  e.feature_flags_dirty <- true;
  e.cycle_nodes_cache <- None;
  e.has_cycles_cache <- false

let mark_stream_layout_dirty e =
  e.stream_layout_dirty <- true

let mark_input_cell_dirty e i =
  e.dirty_input_cells_rev <- i :: e.dirty_input_cells_rev;
  Incremental_runtime.touch_cell e.incr_rt i

let clear_compiled_formula_cache_cell e i =
  if i >= 0 && i < Array.length e.compiled_formula_src then begin
    e.compiled_formula_src.(i) <- None;
    e.compiled_formula_prog.(i) <- None
  end

let apply_literal_cell_compute e i =
  let c = e.sheet.computed.(i) in
  let col = (i mod e.sheet.max_columns) + 1 in
  let row = (i / e.sheet.max_columns) + 1 in
  (match e.sheet.cells.(i) with
   | Sheet.Cell_empty ->
     c.value <- Types.Blank;
     c.text <- "";
     c.error_message <- "";
     c.spill_role <- 0
   | Sheet.Cell_number n ->
     c.value <- Types.Number n;
     c.text <- "";
     c.error_message <- "";
     c.spill_role <- 0
   | Sheet.Cell_text s ->
     c.value <- Types.Text s;
     c.text <- s;
     c.error_message <- "";
     c.spill_role <- 0
   | Sheet.Cell_formula _ -> ());
  c.value_epoch <- e.committed_epoch;
  push_cell_change e col row

let flush_literal_dirty_cells e =
  let seen = Hashtbl.create 32 in
  List.iter (fun i ->
    if not (Hashtbl.mem seen i) then begin
      Hashtbl.add seen i ();
      match e.sheet.cells.(i) with
      | Sheet.Cell_formula _ -> ()
      | _ -> apply_literal_cell_compute e i
    end
  ) e.dirty_input_cells_rev;
  e.dirty_input_cells_rev <- []

let has_dynamic_array_features e =
  let n = Sheet.cell_count e.sheet in
  let found = ref false in
  for i = 0 to n - 1 do
    if not !found then
      match e.sheet.cells.(i) with
      | Sheet.Cell_formula f ->
        if String.contains f '#' ||
           Parser.formula_has_function f "SEQUENCE" ||
           Parser.formula_has_function f "RANDARRAY" ||
           Parser.formula_has_function f "MAP" then
          found := true
      | _ -> ()
  done;
  !found

let has_volatile_or_stream_features e =
  let n = Sheet.cell_count e.sheet in
  let found = ref false in
  for i = 0 to n - 1 do
    if not !found then
      match e.sheet.cells.(i) with
      | Sheet.Cell_formula f ->
        if Parser.formula_has_function f "RAND" ||
           Parser.formula_has_function f "RANDARRAY" ||
           Parser.formula_has_function f "NOW" ||
           Parser.formula_has_function f "STREAM" then
          found := true
      | _ -> ()
  done;
  !found

let refresh_feature_flags_if_needed e =
  if e.feature_flags_dirty then begin
    e.has_dynamic_array_features_cache <- has_dynamic_array_features e;
    e.has_volatile_or_stream_features_cache <- has_volatile_or_stream_features e;
    e.feature_flags_dirty <- false
  end

let incremental_happy_path_allowed e has_cycles =
  e.incremental_runtime_available &&
  not has_cycles &&
  not e.iter_config.enabled &&
  e.names = [] &&
  e.udfs = [] &&
  not e.has_volatile_or_stream_features_cache &&
  not e.has_dynamic_array_features_cache

(* Full recalculation *)
let do_recalculate e =
  let n = Sheet.cell_count e.sheet in
  let names_for_eval = List.map (fun ne -> (ne.ne_name, ne.ne_input)) e.names in
  let udfs_for_eval = List.map (fun u -> (u.udf_name, u.udf_callback)) e.udfs in
  let graph_rebuilt = ref false in
  let cycle_nodes, has_cycles =
    if e.dep_graph_dirty || e.cycle_nodes_cache = None then begin
      graph_rebuilt := true;
      Incremental_runtime.rebuild_deps e.dep_graph e.sheet;
      let cycle_nodes = Incremental_runtime.detect_cycles e.dep_graph in
      let has_cycles = Array.exists (fun x -> x) cycle_nodes in
      e.cycle_nodes_cache <- Some cycle_nodes;
      e.has_cycles_cache <- has_cycles;
      e.dep_graph_dirty <- false;
      (cycle_nodes, has_cycles)
    end else
      match e.cycle_nodes_cache with
      | Some cycle_nodes -> (cycle_nodes, e.has_cycles_cache)
      | None ->
        Incremental_runtime.rebuild_deps e.dep_graph e.sheet;
        let cycle_nodes = Incremental_runtime.detect_cycles e.dep_graph in
        let has_cycles = Array.exists (fun x -> x) cycle_nodes in
        e.cycle_nodes_cache <- Some cycle_nodes;
        e.has_cycles_cache <- has_cycles;
        e.dep_graph_dirty <- false;
        (cycle_nodes, has_cycles)
  in
  if !graph_rebuilt then begin
    try
      Incremental_runtime.rebuild_runtime e.incr_rt e.dep_graph e.sheet;
      Incremental_runtime.touch_global e.incr_rt;
      e.incremental_runtime_available <- true
    with exn ->
      e.incremental_runtime_available <- false;
      mark_dependency_layout_dirty e;
      push_diagnostic e 0 ("Incremental runtime disabled: " ^ Printexc.to_string exn)
  end;
  refresh_feature_flags_if_needed e;

  let run_full_recompute_path () =
    e.dirty_input_cells_rev <- [];
    let prior_values =
      if has_cycles then Some (Array.init n (fun i -> e.sheet.computed.(i).value))
      else None
    in
    if has_cycles && not e.iter_config.enabled then
      push_diagnostic e 0 "Circular reference detected";
    (* Clear spill state *)
    for i = 0 to n - 1 do
      let c = e.sheet.computed.(i) in
      c.spill_role <- 0;
      c.spill_anchor_col <- 0; c.spill_anchor_row <- 0;
      c.spill_range_start_col <- 0; c.spill_range_start_row <- 0;
      c.spill_range_end_col <- 0; c.spill_range_end_row <- 0
    done;
    let cycle_results =
      if has_cycles && not e.iter_config.enabled then begin
        for i = 0 to n - 1 do
          if cycle_nodes.(i) then begin
            (match prior_values with
             | Some prior_values ->
               (match prior_values.(i) with
                | Types.Blank -> e.sheet.computed.(i).value <- Types.Number 0.0
                | v -> e.sheet.computed.(i).value <- v)
             | None -> ())
          end
        done;
        let results = Array.make n Types.Blank in
        for i = 0 to n - 1 do
          if cycle_nodes.(i) then begin
            eval_cell e i names_for_eval udfs_for_eval;
            results.(i) <- e.sheet.computed.(i).value;
            (match prior_values with
             | Some prior_values ->
               (match prior_values.(i) with
                | Types.Blank -> e.sheet.computed.(i).value <- Types.Number 0.0
                | v -> e.sheet.computed.(i).value <- v)
             | None -> ())
          end
        done;
        Some results
      end else
        None
    in
    for i = 0 to n - 1 do
      if e.sheet.computed.(i).spill_role <> 2 then begin
        if cycle_nodes.(i) && not e.iter_config.enabled then begin
          let col = (i mod e.sheet.max_columns) + 1 in
          let row = (i / e.sheet.max_columns) + 1 in
          (match cycle_results with
           | Some results -> e.sheet.computed.(i).value <- results.(i)
           | None -> ());
          e.sheet.computed.(i).value_epoch <- e.committed_epoch;
          push_cell_change e col row
        end else
          eval_cell e i names_for_eval udfs_for_eval
      end
    done;
    if has_cycles && e.iter_config.enabled then begin
      for _iter = 1 to e.iter_config.max_iterations do
        let max_delta = ref 0.0 in
        for i = 0 to n - 1 do
          if cycle_nodes.(i) then begin
            let old_val = Types.value_number e.sheet.computed.(i).value in
            eval_cell e i names_for_eval udfs_for_eval;
            let new_val = Types.value_number e.sheet.computed.(i).value in
            let delta = abs_float (new_val -. old_val) in
            if delta > !max_delta then max_delta := delta
          end
        done;
        if !max_delta <= e.iter_config.convergence_tolerance then
          ()
      done
    end
  in

  if incremental_happy_path_allowed e has_cycles then begin
    flush_literal_dirty_cells e;
    try
      let changed_formulas = Incremental_runtime.stabilize_and_take_changed_formulas e.incr_rt in
      List.iter (fun i -> eval_cell e i names_for_eval udfs_for_eval) changed_formulas
    with exn ->
      e.incremental_runtime_available <- false;
      mark_dependency_layout_dirty e;
      push_diagnostic e 0 ("Incremental stabilize fallback: " ^ Printexc.to_string exn);
      run_full_recompute_path ()
  end else begin
    run_full_recompute_path ()
  end;
  (* Setup stream state *)
  if e.stream_layout_dirty then begin
    setup_streams e;
    e.stream_layout_dirty <- false
  end;
  (* Update stream cell values *)
  for i = 0 to n - 1 do
    let st = e.sheet.streams.(i) in
    if st.active then begin
      e.sheet.computed.(i).value <- Types.Number (float_of_int st.counter);
      e.sheet.computed.(i).value_epoch <- e.committed_epoch
    end
  done;
  (* Mark stabilized *)
  e.stabilized_epoch <- e.committed_epoch;
  (* Clear stale flags *)
  for i = 0 to n - 1 do
    e.sheet.computed.(i).value_epoch <- e.committed_epoch
  done

let bump_epoch e =
  e.committed_epoch <- e.committed_epoch + 1

let auto_recalc e =
  if e.recalc_mode = Types.Recalc_mode.automatic then
    do_recalculate e

(* ----- Public API ----- *)

let bounds e = (e.sheet.max_columns, e.sheet.max_rows)

let get_recalc_mode e = e.recalc_mode
let set_recalc_mode e mode =
  clear_status e;
  e.recalc_mode <- mode;
  Types.Status.ok

let committed_epoch e = e.committed_epoch
let stabilized_epoch e = e.stabilized_epoch
let is_stable e = e.committed_epoch = e.stabilized_epoch

let set_number e addr value =
  clear_status e;
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    (match e.sheet.cells.(i) with
     | Sheet.Cell_formula _ ->
       mark_dependency_layout_dirty e;
       mark_stream_layout_dirty e
     | _ -> ());
    e.sheet.cells.(i) <- Sheet.Cell_number value;
    clear_compiled_formula_cache_cell e i;
    mark_input_cell_dirty e i;
    bump_epoch e;
    push_cell_change e addr.col addr.row;
    auto_recalc e;
    Types.Status.ok
  end

let set_text e addr text =
  clear_status e;
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    (match e.sheet.cells.(i) with
     | Sheet.Cell_formula _ ->
       mark_dependency_layout_dirty e;
       mark_stream_layout_dirty e
     | _ -> ());
    e.sheet.cells.(i) <- Sheet.Cell_text text;
    clear_compiled_formula_cache_cell e i;
    mark_input_cell_dirty e i;
    bump_epoch e;
    push_cell_change e addr.col addr.row;
    auto_recalc e;
    Types.Status.ok
  end

let set_formula e addr formula =
  clear_status e;
  if String.length formula = 0 then
    Types.Status.err_parse
  else if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    mark_dependency_layout_dirty e;
    mark_stream_layout_dirty e;
    e.sheet.cells.(i) <- Sheet.Cell_formula formula;
    e.compiled_formula_src.(i) <- Some formula;
    e.compiled_formula_prog.(i) <- Eval.try_compile formula;
    mark_input_cell_dirty e i;
    bump_epoch e;
    auto_recalc e;
    Types.Status.ok
  end

let cell_clear e addr =
  clear_status e;
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    (match e.sheet.cells.(i) with
     | Sheet.Cell_formula _ ->
       mark_dependency_layout_dirty e;
       mark_stream_layout_dirty e
     | _ -> ());
    Sheet.clear_cell e.sheet i;
    clear_compiled_formula_cache_cell e i;
    mark_input_cell_dirty e i;
    bump_epoch e;
    push_cell_change e addr.col addr.row;
    auto_recalc e;
    Types.Status.ok
  end

let get_state e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    let c = e.sheet.computed.(i) in
    Ok {
      Types.value = c.value;
      value_epoch = c.value_epoch;
      stale = c.value_epoch < e.committed_epoch;
    }
  end

let get_text e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    let c = e.sheet.computed.(i) in
    match c.value with
    | Types.Text s -> Ok s
    | Types.Number n ->
      if Float.is_integer n && n >= -1e15 && n <= 1e15 then
        Ok (string_of_int (Float.to_int n))
      else Ok (Printf.sprintf "%.17g" n)
    | Types.Bool true -> Ok "TRUE"
    | Types.Bool false -> Ok "FALSE"
    | Types.Blank -> Ok ""
    | Types.Error (_, m) -> Ok m
  end

let get_input_type e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    Ok (Sheet.get_input_type e.sheet i)
  end

let get_input_text e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    Ok (Sheet.get_input_text e.sheet i)
  end

let error_message e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    Ok e.sheet.computed.(i).error_message
  end

(* Spill info *)
let spill_role e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    Ok e.sheet.computed.(i).spill_role
  end

let spill_anchor e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    let c = e.sheet.computed.(i) in
    if c.spill_role = 2 then
      Ok (Some (c.spill_anchor_col, c.spill_anchor_row))
    else
      Ok None
  end

let spill_range e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    let c = e.sheet.computed.(i) in
    if c.spill_role = 1 then
      Ok (Some (c.spill_range_start_col, c.spill_range_start_row,
                c.spill_range_end_col, c.spill_range_end_row))
    else
      Ok None
  end

(* Named definitions *)
let name_set_number e name value =
  clear_status e;
  let upper = String.uppercase_ascii name in
  if not (Ast.is_valid_name name) || Ast.is_builtin name || Address.is_cell_like name then
    Types.Status.err_invalid_name
  else begin
    let found = ref false in
    e.names <- List.map (fun ne ->
      if String.uppercase_ascii ne.ne_name = upper then begin
        found := true;
        { ne with ne_input = Sheet.Cell_number value }
      end else ne
    ) e.names;
    if not !found then
      e.names <- e.names @ [{ ne_name = name; ne_input = Sheet.Cell_number value }];
    Incremental_runtime.touch_global e.incr_rt;
    bump_epoch e;
    push_name_change e name;
    auto_recalc e;
    Types.Status.ok
  end

let name_set_text e name text =
  clear_status e;
  let upper = String.uppercase_ascii name in
  if not (Ast.is_valid_name name) || Ast.is_builtin name || Address.is_cell_like name then
    Types.Status.err_invalid_name
  else begin
    let found = ref false in
    e.names <- List.map (fun ne ->
      if String.uppercase_ascii ne.ne_name = upper then begin
        found := true;
        { ne with ne_input = Sheet.Cell_text text }
      end else ne
    ) e.names;
    if not !found then
      e.names <- e.names @ [{ ne_name = name; ne_input = Sheet.Cell_text text }];
    Incremental_runtime.touch_global e.incr_rt;
    bump_epoch e;
    push_name_change e name;
    auto_recalc e;
    Types.Status.ok
  end

let name_set_formula e name formula =
  clear_status e;
  let upper = String.uppercase_ascii name in
  if not (Ast.is_valid_name name) || Ast.is_builtin name || Address.is_cell_like name then
    Types.Status.err_invalid_name
  else begin
    let found = ref false in
    e.names <- List.map (fun ne ->
      if String.uppercase_ascii ne.ne_name = upper then begin
        found := true;
        { ne with ne_input = Sheet.Cell_formula formula }
      end else ne
    ) e.names;
    if not !found then
      e.names <- e.names @ [{ ne_name = name; ne_input = Sheet.Cell_formula formula }];
    Incremental_runtime.touch_global e.incr_rt;
    bump_epoch e;
    push_name_change e name;
    auto_recalc e;
    Types.Status.ok
  end

let name_clear e name =
  clear_status e;
  let upper = String.uppercase_ascii name in
  e.names <- List.filter (fun ne -> String.uppercase_ascii ne.ne_name <> upper) e.names;
  Incremental_runtime.touch_global e.incr_rt;
  bump_epoch e;
  auto_recalc e;
  Types.Status.ok

let name_get_input_type e name =
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ne -> String.uppercase_ascii ne.ne_name = upper) e.names with
  | Some ne ->
    Ok (match ne.ne_input with
      | Sheet.Cell_empty -> Types.Empty
      | Sheet.Cell_number _ -> Types.Input_number
      | Sheet.Cell_text _ -> Types.Input_text
      | Sheet.Cell_formula _ -> Types.Input_formula)
  | None -> Error Types.Status.err_invalid_name

let name_get_input_text e name =
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ne -> String.uppercase_ascii ne.ne_name = upper) e.names with
  | Some ne ->
    Ok (match ne.ne_input with
      | Sheet.Cell_empty -> ""
      | Sheet.Cell_number n -> Printf.sprintf "%.17g" n
      | Sheet.Cell_text s -> s
      | Sheet.Cell_formula s -> s)
  | None -> Error Types.Status.err_invalid_name

(* Recalculate *)
let recalculate e =
  clear_status e;
  do_recalculate e;
  Types.Status.ok

(* Volatile *)
let has_volatile_cells e =
  let n = Sheet.cell_count e.sheet in
  let found = ref false in
  for i = 0 to n - 1 do
    if not !found then
      match e.sheet.cells.(i) with
      | Sheet.Cell_formula f ->
        if Parser.formula_has_function f "RAND" ||
           Parser.formula_has_function f "RANDARRAY" ||
           Parser.formula_has_function f "NOW" then
          found := true
      | _ -> ()
  done;
  !found

let has_externally_invalidated_cells e =
  let n = Sheet.cell_count e.sheet in
  let found = ref false in
  for i = 0 to n - 1 do
    if not !found then begin
      match e.sheet.cells.(i) with
      | Sheet.Cell_formula f ->
        if Parser.formula_has_function f "STREAM" then found := true
      | _ -> ()
    end
  done;
  if not !found then
    List.iter (fun u ->
      if u.udf_volatility = Types.Externally_invalidated then found := true
    ) e.udfs;
  !found

let invalidate_volatile e =
  clear_status e;
  Incremental_runtime.touch_global e.incr_rt;
  bump_epoch e;
  do_recalculate e;
  Types.Status.ok

let has_stream_cells e =
  let n = Sheet.cell_count e.sheet in
  let found = ref false in
  for i = 0 to n - 1 do
    if not !found && e.sheet.streams.(i).active then found := true
  done;
  !found

let tick_streams e elapsed =
  clear_status e;
  let any_advanced = ref false in
  let n = Sheet.cell_count e.sheet in
  for i = 0 to n - 1 do
    let st = e.sheet.streams.(i) in
    if st.active && st.period > 0.0 then begin
      st.elapsed <- st.elapsed +. elapsed;
      while st.elapsed >= st.period do
        st.elapsed <- st.elapsed -. st.period;
        st.counter <- st.counter + 1;
        any_advanced := true
      done
    end
  done;
  if !any_advanced then begin
    Incremental_runtime.touch_global e.incr_rt;
    bump_epoch e;
    auto_recalc e
  end;
  (Types.Status.ok, !any_advanced)

let invalidate_udf e _name =
  clear_status e;
  Incremental_runtime.touch_global e.incr_rt;
  bump_epoch e;
  auto_recalc e;
  Types.Status.ok

(* Format *)
let get_format e addr =
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Error Types.Status.err_out_of_bounds
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    Ok e.sheet.formats.(i)
  end

let set_format e addr fmt =
  clear_status e;
  if not (Sheet.in_bounds e.sheet addr.Address.col addr.Address.row) then
    Types.Status.err_out_of_bounds
  else if fmt.Types.decimals > 9 then
    Types.Status.err_invalid_argument
  else if (fmt.Types.fg <> -1 && (fmt.Types.fg < 0 || fmt.Types.fg > 15)) ||
          (fmt.Types.bg <> -1 && (fmt.Types.bg < 0 || fmt.Types.bg > 15)) then
    Types.Status.err_invalid_argument
  else begin
    let i = Sheet.idx e.sheet addr.col addr.row in
    let old_fmt = e.sheet.formats.(i) in
    e.sheet.formats.(i) <- fmt;
    bump_epoch e;
    push_change e {
      change_type = Types.Change_cell_format; epoch = e.committed_epoch;
      cell_col = 0; cell_row = 0; name = ""; chart_name = "";
      spill_anchor_col = 0; spill_anchor_row = 0;
      spill_old_range = None; spill_new_range = None;
      fmt_col = addr.col; fmt_row = addr.row;
      old_fmt; new_fmt = fmt;
      diag_code = 0; diag_message = "";
    };
    Types.Status.ok
  end

(* Structural operations *)

(* Check if a structural operation would split a spill region *)
let check_spill_constraint e op_kind idx =
  let n = Sheet.cell_count e.sheet in
  let blocked = ref None in
  for i = 0 to n - 1 do
    if !blocked = None then begin
      let c = e.sheet.computed.(i) in
      if c.spill_role = 1 then begin
        let sr1 = c.spill_range_start_row and sr2 = c.spill_range_end_row in
        let sc1 = c.spill_range_start_col and sc2 = c.spill_range_end_col in
        let col = (i mod e.sheet.max_columns) + 1 in
        let row = (i / e.sheet.max_columns) + 1 in
        let conflicts = match op_kind with
          | Types.Insert_row | Types.Delete_row ->
            idx >= sr1 && idx <= sr2 && (sr2 > sr1 || idx = sr1)
          | Types.Insert_col | Types.Delete_col ->
            idx >= sc1 && idx <= sc2 && (sc2 > sc1 || idx = sc1)
          | _ -> false
        in
        if conflicts then
          blocked := Some (col, row, sc1, sr1, sc2, sr2)
      end
    end
  done;
  !blocked

(* Rewrite a formula for structural operations *)
let rewrite_formula formula op_kind idx max_col max_row =
  let len = String.length formula in
  let buf = Buffer.create (len + 16) in
  let i = ref 0 in
  let in_str = ref false in
  while !i < len do
    if !in_str then begin
      Buffer.add_char buf formula.[!i];
      if formula.[!i] = '"' then in_str := false;
      incr i
    end else if formula.[!i] = '"' then begin
      Buffer.add_char buf formula.[!i];
      in_str := true;
      incr i
    end else begin
      (* Try to parse a cell reference at boundary *)
      let at_bound = !i = 0 || not (Parser.is_alnum formula.[!i - 1]) in
      if at_bound then begin
        match Address.parse_a1 formula !i with
        | Some (col, row, consumed, col_abs, row_abs) when col >= 1 && row >= 1 ->
          let new_col = ref col in
          let new_row = ref row in
          let deleted = ref false in
          (match op_kind with
           | Types.Insert_row ->
             if not row_abs && row >= idx then new_row := row + 1
           | Types.Delete_row ->
             if not row_abs then begin
               if row = idx then deleted := true
               else if row > idx then new_row := row - 1
             end
           | Types.Insert_col ->
             if not col_abs && col >= idx then new_col := col + 1
           | Types.Delete_col ->
             if not col_abs then begin
               if col = idx then deleted := true
               else if col > idx then new_col := col - 1
             end
           | _ -> ());
          if !deleted then begin
            Buffer.add_string buf "#REF!";
            i := !i + consumed;
            (* Skip trailing # if present *)
            if !i < len && formula.[!i] = '#' then incr i
          end else if !new_col >= 1 && !new_col <= max_col &&
                      !new_row >= 1 && !new_row <= max_row then begin
            let addr = Address.make ~col:!new_col ~row:!new_row in
            Buffer.add_string buf (Address.to_a1 ~col_abs ~row_abs addr);
            i := !i + consumed;
            (* Preserve trailing # *)
            if !i < len && formula.[!i] = '#' then begin
              Buffer.add_char buf '#';
              incr i
            end
          end else begin
            Buffer.add_string buf "#REF!";
            i := !i + consumed;
            if !i < len && formula.[!i] = '#' then incr i
          end
        | _ ->
          Buffer.add_char buf formula.[!i];
          incr i
      end else begin
        Buffer.add_char buf formula.[!i];
        incr i
      end
    end
  done;
  Buffer.contents buf

let insert_row e at =
  clear_status e;
  if at < 1 || at > e.sheet.max_rows then
    Types.Status.err_out_of_bounds
  else begin
    match check_spill_constraint e Types.Insert_row at with
    | Some (col, row, sc1, sr1, sc2, sr2) ->
      e.last_reject_kind <- 1;
      e.last_reject_context <- {
        reject_kind = 1; op_kind = Types.Insert_row; op_index = at;
        has_cell = true; cell_col = col; cell_row = row;
        has_range = true;
        range_start_col = sc1; range_start_row = sr1;
        range_end_col = sc2; range_end_row = sr2;
      };
      Types.Status.reject_structural
    | None ->
      (* Shift rows down *)
      let mc = e.sheet.max_columns in
      for row = e.sheet.max_rows downto at + 1 do
        for col = 1 to mc do
          let dst = Sheet.idx e.sheet col row in
          let src = Sheet.idx e.sheet col (row - 1) in
          e.sheet.cells.(dst) <- e.sheet.cells.(src);
          let sc = e.sheet.computed.(src) in
          let dc = e.sheet.computed.(dst) in
          dc.value <- sc.value; dc.value_epoch <- sc.value_epoch;
          dc.text <- sc.text; dc.error_message <- sc.error_message;
          dc.spill_role <- sc.spill_role;
          dc.spill_anchor_col <- sc.spill_anchor_col;
          dc.spill_anchor_row <- sc.spill_anchor_row;
          dc.spill_range_start_col <- sc.spill_range_start_col;
          dc.spill_range_start_row <- sc.spill_range_start_row;
          dc.spill_range_end_col <- sc.spill_range_end_col;
          dc.spill_range_end_row <- sc.spill_range_end_row;
          e.sheet.formats.(dst) <- e.sheet.formats.(src);
          let ss = e.sheet.streams.(src) and ds = e.sheet.streams.(dst) in
          ds.active <- ss.active; ds.period <- ss.period;
          ds.elapsed <- ss.elapsed; ds.counter <- ss.counter
        done
      done;
      (* Clear the inserted row *)
      for col = 1 to mc do
        let i = Sheet.idx e.sheet col at in
        Sheet.clear_cell e.sheet i
      done;
      (* Rewrite formulas *)
      let n = Sheet.cell_count e.sheet in
      for i = 0 to n - 1 do
        match e.sheet.cells.(i) with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Insert_row at mc e.sheet.max_rows in
          if f' <> f then e.sheet.cells.(i) <- Sheet.Cell_formula f'
        | _ -> ()
      done;
      (* Rewrite named formulas *)
      e.names <- List.map (fun ne ->
        match ne.ne_input with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Insert_row at mc e.sheet.max_rows in
          if f' <> f then { ne with ne_input = Sheet.Cell_formula f' } else ne
        | _ -> ne
      ) e.names;
      mark_dependency_layout_dirty e;
      mark_stream_layout_dirty e;
      bump_epoch e;
      auto_recalc e;
      Types.Status.ok
  end

let delete_row e at =
  clear_status e;
  if at < 1 || at > e.sheet.max_rows then
    Types.Status.err_out_of_bounds
  else begin
    match check_spill_constraint e Types.Delete_row at with
    | Some (col, row, sc1, sr1, sc2, sr2) ->
      e.last_reject_kind <- 1;
      e.last_reject_context <- {
        reject_kind = 1; op_kind = Types.Delete_row; op_index = at;
        has_cell = true; cell_col = col; cell_row = row;
        has_range = true;
        range_start_col = sc1; range_start_row = sr1;
        range_end_col = sc2; range_end_row = sr2;
      };
      Types.Status.reject_structural
    | None ->
      let mc = e.sheet.max_columns in
      (* Rewrite formulas BEFORE shifting *)
      let n = Sheet.cell_count e.sheet in
      for i = 0 to n - 1 do
        match e.sheet.cells.(i) with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Delete_row at mc e.sheet.max_rows in
          if f' <> f then e.sheet.cells.(i) <- Sheet.Cell_formula f'
        | _ -> ()
      done;
      e.names <- List.map (fun ne ->
        match ne.ne_input with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Delete_row at mc e.sheet.max_rows in
          if f' <> f then { ne with ne_input = Sheet.Cell_formula f' } else ne
        | _ -> ne
      ) e.names;
      mark_dependency_layout_dirty e;
      mark_stream_layout_dirty e;
      (* Shift rows up *)
      for row = at to e.sheet.max_rows - 1 do
        for col = 1 to mc do
          let dst = Sheet.idx e.sheet col row in
          let src = Sheet.idx e.sheet col (row + 1) in
          e.sheet.cells.(dst) <- e.sheet.cells.(src);
          let sc = e.sheet.computed.(src) in
          let dc = e.sheet.computed.(dst) in
          dc.value <- sc.value; dc.value_epoch <- sc.value_epoch;
          dc.text <- sc.text; dc.error_message <- sc.error_message;
          dc.spill_role <- sc.spill_role;
          dc.spill_anchor_col <- sc.spill_anchor_col;
          dc.spill_anchor_row <- sc.spill_anchor_row;
          dc.spill_range_start_col <- sc.spill_range_start_col;
          dc.spill_range_start_row <- sc.spill_range_start_row;
          dc.spill_range_end_col <- sc.spill_range_end_col;
          dc.spill_range_end_row <- sc.spill_range_end_row;
          e.sheet.formats.(dst) <- e.sheet.formats.(src);
          let ss = e.sheet.streams.(src) and ds = e.sheet.streams.(dst) in
          ds.active <- ss.active; ds.period <- ss.period;
          ds.elapsed <- ss.elapsed; ds.counter <- ss.counter
        done
      done;
      (* Clear last row *)
      for col = 1 to mc do
        let i = Sheet.idx e.sheet col e.sheet.max_rows in
        Sheet.clear_cell e.sheet i
      done;
      bump_epoch e;
      auto_recalc e;
      Types.Status.ok
  end

let insert_col e at =
  clear_status e;
  if at < 1 || at > e.sheet.max_columns then
    Types.Status.err_out_of_bounds
  else begin
    match check_spill_constraint e Types.Insert_col at with
    | Some (col, row, sc1, sr1, sc2, sr2) ->
      e.last_reject_kind <- 1;
      e.last_reject_context <- {
        reject_kind = 1; op_kind = Types.Insert_col; op_index = at;
        has_cell = true; cell_col = col; cell_row = row;
        has_range = true;
        range_start_col = sc1; range_start_row = sr1;
        range_end_col = sc2; range_end_row = sr2;
      };
      Types.Status.reject_structural
    | None ->
      let mr = e.sheet.max_rows in
      let mc = e.sheet.max_columns in
      for row = 1 to mr do
        for col = mc downto at + 1 do
          let dst = Sheet.idx e.sheet col row in
          let src = Sheet.idx e.sheet (col - 1) row in
          e.sheet.cells.(dst) <- e.sheet.cells.(src);
          let sc = e.sheet.computed.(src) in
          let dc = e.sheet.computed.(dst) in
          dc.value <- sc.value; dc.value_epoch <- sc.value_epoch;
          dc.text <- sc.text; dc.error_message <- sc.error_message;
          dc.spill_role <- sc.spill_role;
          dc.spill_anchor_col <- sc.spill_anchor_col;
          dc.spill_anchor_row <- sc.spill_anchor_row;
          dc.spill_range_start_col <- sc.spill_range_start_col;
          dc.spill_range_start_row <- sc.spill_range_start_row;
          dc.spill_range_end_col <- sc.spill_range_end_col;
          dc.spill_range_end_row <- sc.spill_range_end_row;
          e.sheet.formats.(dst) <- e.sheet.formats.(src);
          let ss = e.sheet.streams.(src) and ds = e.sheet.streams.(dst) in
          ds.active <- ss.active; ds.period <- ss.period;
          ds.elapsed <- ss.elapsed; ds.counter <- ss.counter
        done;
        let i = Sheet.idx e.sheet at row in
        Sheet.clear_cell e.sheet i
      done;
      (* Rewrite formulas *)
      let n = Sheet.cell_count e.sheet in
      for i = 0 to n - 1 do
        match e.sheet.cells.(i) with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Insert_col at mc mr in
          if f' <> f then e.sheet.cells.(i) <- Sheet.Cell_formula f'
        | _ -> ()
      done;
      e.names <- List.map (fun ne ->
        match ne.ne_input with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Insert_col at mc mr in
          if f' <> f then { ne with ne_input = Sheet.Cell_formula f' } else ne
        | _ -> ne
      ) e.names;
      mark_dependency_layout_dirty e;
      mark_stream_layout_dirty e;
      bump_epoch e;
      auto_recalc e;
      Types.Status.ok
  end

let delete_col e at =
  clear_status e;
  if at < 1 || at > e.sheet.max_columns then
    Types.Status.err_out_of_bounds
  else begin
    match check_spill_constraint e Types.Delete_col at with
    | Some (col, row, sc1, sr1, sc2, sr2) ->
      e.last_reject_kind <- 1;
      e.last_reject_context <- {
        reject_kind = 1; op_kind = Types.Delete_col; op_index = at;
        has_cell = true; cell_col = col; cell_row = row;
        has_range = true;
        range_start_col = sc1; range_start_row = sr1;
        range_end_col = sc2; range_end_row = sr2;
      };
      Types.Status.reject_structural
    | None ->
      let mr = e.sheet.max_rows in
      let mc = e.sheet.max_columns in
      (* Rewrite formulas BEFORE shifting *)
      let n = Sheet.cell_count e.sheet in
      for i = 0 to n - 1 do
        match e.sheet.cells.(i) with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Delete_col at mc mr in
          if f' <> f then e.sheet.cells.(i) <- Sheet.Cell_formula f'
        | _ -> ()
      done;
      e.names <- List.map (fun ne ->
        match ne.ne_input with
        | Sheet.Cell_formula f ->
          let f' = rewrite_formula f Types.Delete_col at mc mr in
          if f' <> f then { ne with ne_input = Sheet.Cell_formula f' } else ne
        | _ -> ne
      ) e.names;
      mark_dependency_layout_dirty e;
      mark_stream_layout_dirty e;
      for row = 1 to mr do
        for col = at to mc - 1 do
          let dst = Sheet.idx e.sheet col row in
          let src = Sheet.idx e.sheet (col + 1) row in
          e.sheet.cells.(dst) <- e.sheet.cells.(src);
          let sc = e.sheet.computed.(src) in
          let dc = e.sheet.computed.(dst) in
          dc.value <- sc.value; dc.value_epoch <- sc.value_epoch;
          dc.text <- sc.text; dc.error_message <- sc.error_message;
          dc.spill_role <- sc.spill_role;
          dc.spill_anchor_col <- sc.spill_anchor_col;
          dc.spill_anchor_row <- sc.spill_anchor_row;
          dc.spill_range_start_col <- sc.spill_range_start_col;
          dc.spill_range_start_row <- sc.spill_range_start_row;
          dc.spill_range_end_col <- sc.spill_range_end_col;
          dc.spill_range_end_row <- sc.spill_range_end_row;
          e.sheet.formats.(dst) <- e.sheet.formats.(src);
          let ss = e.sheet.streams.(src) and ds = e.sheet.streams.(dst) in
          ds.active <- ss.active; ds.period <- ss.period;
          ds.elapsed <- ss.elapsed; ds.counter <- ss.counter
        done;
        let i = Sheet.idx e.sheet mc row in
        Sheet.clear_cell e.sheet i
      done;
      bump_epoch e;
      auto_recalc e;
      Types.Status.ok
  end

(* Iteration config *)
let get_iteration_config e = e.iter_config
let set_iteration_config e cfg =
  clear_status e;
  e.iter_config <- cfg;
  Types.Status.ok

(* Controls *)
let control_define e name def =
  clear_status e;
  let upper = String.uppercase_ascii name in
  let initial = match def.Types.kind with
    | Types.Slider -> def.Types.min
    | Types.Checkbox -> 0.0
    | Types.Button -> 0.0
  in
  let found = ref false in
  e.controls <- List.map (fun ce ->
    if String.uppercase_ascii ce.ce_name = upper then begin
      found := true;
      { ce with ce_def = def; ce_value = initial }
    end else ce
  ) e.controls;
  if not !found then
    e.controls <- e.controls @ [{ ce_name = upper; ce_def = def; ce_value = initial }];
  (* Auto-set named definition *)
  ignore (name_set_number e name initial);
  Types.Status.ok

let control_remove e name =
  clear_status e;
  let upper = String.uppercase_ascii name in
  let before = List.length e.controls in
  e.controls <- List.filter (fun ce -> String.uppercase_ascii ce.ce_name <> upper) e.controls;
  List.length e.controls < before

let control_set_value e name value =
  clear_status e;
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ce -> String.uppercase_ascii ce.ce_name = upper) e.controls with
  | None -> Types.Status.err_invalid_name
  | Some ce ->
    let clamped = match ce.ce_def.kind with
      | Types.Checkbox ->
        if value <> 0.0 && value <> 1.0 then ce.ce_value
        else value
      | Types.Slider -> Float.min (Float.max value ce.ce_def.min) ce.ce_def.max
      | Types.Button -> value
    in
    ce.ce_value <- clamped;
    ignore (name_set_number e name clamped);
    Types.Status.ok

let control_get_value e name =
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ce -> String.uppercase_ascii ce.ce_name = upper) e.controls with
  | Some ce -> Ok ce.ce_value
  | None -> Error Types.Status.err_invalid_name

let control_get_def e name =
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ce -> String.uppercase_ascii ce.ce_name = upper) e.controls with
  | Some ce -> Ok ce.ce_def
  | None -> Error Types.Status.err_invalid_name

let control_list e =
  List.sort (fun a b ->
    String.compare (String.uppercase_ascii a.ce_name) (String.uppercase_ascii b.ce_name)
  ) e.controls

(* Charts *)
let chart_define e name def =
  clear_status e;
  let upper = String.uppercase_ascii name in
  let found = ref false in
  e.charts <- List.map (fun ch ->
    if String.uppercase_ascii ch.ch_name = upper then begin
      found := true;
      { ch with ch_def = def }
    end else ch
  ) e.charts;
  if not !found then
    e.charts <- e.charts @ [{ ch_name = upper; ch_def = def }];
  Types.Status.ok

let chart_remove e name =
  clear_status e;
  let upper = String.uppercase_ascii name in
  let before = List.length e.charts in
  e.charts <- List.filter (fun ch -> String.uppercase_ascii ch.ch_name <> upper) e.charts;
  List.length e.charts < before

let chart_get_output e name =
  let upper = String.uppercase_ascii name in
  match List.find_opt (fun ch -> String.uppercase_ascii ch.ch_name = upper) e.charts with
  | None -> None
  | Some ch ->
    let def = ch.ch_def in
    let r1 = min def.source_start_row def.source_end_row in
    let r2 = max def.source_start_row def.source_end_row in
    let c1 = min def.source_start_col def.source_end_col in
    let c2 = max def.source_start_col def.source_end_col in
    let rows = r2 - r1 + 1 in
    let cols = c2 - c1 + 1 in
    let labels = Array.init rows (fun r -> Printf.sprintf "R%d" (r1 + r)) in
    let series_names = Array.init cols (fun c -> Printf.sprintf "C%d" (c1 + c)) in
    let series_values = Array.init cols (fun c ->
      Array.init rows (fun r ->
        let col = c1 + c and row = r1 + r in
        if Sheet.in_bounds e.sheet col row then
          Types.value_number e.sheet.computed.(Sheet.idx e.sheet col row).value
        else 0.0
      )
    ) in
    Some { co_labels = labels; co_series_names = series_names; co_series_values = series_values }

let chart_list e =
  List.sort (fun a b ->
    String.compare (String.uppercase_ascii a.ch_name) (String.uppercase_ascii b.ch_name)
  ) e.charts

(* UDF *)
let udf_register e name callback volatility =
  clear_status e;
  let upper = String.uppercase_ascii name in
  e.udfs <- List.filter (fun u -> String.uppercase_ascii u.udf_name <> upper) e.udfs;
  e.udfs <- e.udfs @ [{ udf_name = name; udf_callback = callback; udf_volatility = volatility }];
  Types.Status.ok

let udf_unregister e name =
  let upper = String.uppercase_ascii name in
  let before = List.length e.udfs in
  e.udfs <- List.filter (fun u -> String.uppercase_ascii u.udf_name <> upper) e.udfs;
  List.length e.udfs < before

(* Change tracking *)
let change_tracking_enable e =
  clear_status e;
  e.change_tracking <- true;
  e.changes <- [];
  Types.Status.ok

let change_tracking_disable e =
  clear_status e;
  e.change_tracking <- false;
  e.changes <- [];
  Types.Status.ok

let change_tracking_is_enabled e = e.change_tracking

let drain_changes e =
  let changes = List.rev e.changes in
  e.changes <- [];
  changes

(* Error/reject info *)
let last_error_kind e = e.last_error_kind
let last_error_message e = e.last_error_message
let last_reject_kind e = e.last_reject_kind
let last_reject_context e = e.last_reject_context

(* Cell iteration *)
let cell_iterate e =
  let entries = ref [] in
  let n = Sheet.cell_count e.sheet in
  for i = 0 to n - 1 do
    match e.sheet.cells.(i) with
    | Sheet.Cell_empty -> ()
    | _ ->
      let col = (i mod e.sheet.max_columns) + 1 in
      let row = (i / e.sheet.max_columns) + 1 in
      entries := (col, row, Sheet.get_input_type e.sheet i, Sheet.get_input_text e.sheet i) :: !entries
  done;
  List.rev !entries

(* Name iteration *)
let name_iterate e =
  let sorted = List.sort (fun a b ->
    String.compare (String.uppercase_ascii a.ne_name) (String.uppercase_ascii b.ne_name)
  ) e.names in
  List.map (fun ne ->
    let it = match ne.ne_input with
      | Sheet.Cell_empty -> Types.Empty
      | Sheet.Cell_number _ -> Types.Input_number
      | Sheet.Cell_text _ -> Types.Input_text
      | Sheet.Cell_formula _ -> Types.Input_formula
    in
    let text = match ne.ne_input with
      | Sheet.Cell_empty -> ""
      | Sheet.Cell_number n -> Printf.sprintf "%.17g" n
      | Sheet.Cell_text s -> s
      | Sheet.Cell_formula s -> s
    in
    (ne.ne_name, it, text)
  ) sorted

(* Parse cell ref *)
let parse_cell_ref _e ref_str =
  match Address.parse_a1 ref_str 0 with
  | Some (col, row, consumed, _, _) when consumed = String.length ref_str ->
    Ok (col, row)
  | _ -> Error Types.Status.err_invalid_address

(* Engine clear *)
let clear e =
  clear_status e;
  Sheet.clear_all e.sheet;
  e.names <- [];
  e.controls <- [];
  e.charts <- [];
  e.udfs <- [];
  e.changes <- [];
  e.dirty_input_cells_rev <- [];
  Array.fill e.compiled_formula_src 0 (Array.length e.compiled_formula_src) None;
  Array.fill e.compiled_formula_prog 0 (Array.length e.compiled_formula_prog) None;
  mark_dependency_layout_dirty e;
  mark_stream_layout_dirty e;
  bump_epoch e;
  e.stabilized_epoch <- e.committed_epoch;
  Types.Status.ok

(* Format iteration *)
let format_iterate e =
  let entries = ref [] in
  let n = Sheet.cell_count e.sheet in
  for i = 0 to n - 1 do
    let fmt = e.sheet.formats.(i) in
    if fmt <> Types.default_format then begin
      let col = (i mod e.sheet.max_columns) + 1 in
      let row = (i / e.sheet.max_columns) + 1 in
      entries := (col, row, fmt) :: !entries
    end
  done;
  List.rev !entries
