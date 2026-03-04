(* incremental_runtime.ml - Dependency tracking and incremental recalculation
   using Jane Street Incremental concepts.

   Architecture: We maintain a dependency graph over cells. When a cell is
   dirtied, we propagate staleness through reverse dependencies. Recalculation
   evaluates only dirty cells in topological order, with cycle detection and
   optional iterative convergence for circular references. *)

(* Dependency graph: for each cell index, the set of cells it depends on
   (forward) and cells that depend on it (reverse). *)
type dep_graph = {
  mutable forward : int list array;   (* cell -> deps *)
  mutable reverse : int list array;   (* cell -> dependents *)
  mutable n : int;
}

module Incr = Incremental.Make ()

let default_max_height_allowed = 8192

let ensure_height_budget () =
  let current = Incr.State.max_height_allowed Incr.State.t in
  if current < default_max_height_allowed then
    Incr.State.set_max_height_allowed Incr.State.t default_max_height_allowed

type runtime = {
  mutable vars : int Incr.Var.t array;
  mutable nodes : int Incr.t array;
  mutable observers : int Incr.Observer.t option array;
  mutable changed_flag : bool array;
  mutable changed_formula_rev : int list;
  mutable global_tick : int Incr.Var.t;
  mutable stamp : int;
}

let create_graph n = {
  forward = Array.make n [];
  reverse = Array.make n [];
  n;
}

let create_runtime n =
  ensure_height_budget ();
  let vars = Array.init n (fun _ -> Incr.Var.create 0) in
  {
    vars;
    nodes = Array.map Incr.Var.watch vars;
    observers = Array.make n None;
    changed_flag = Array.make n false;
    changed_formula_rev = [];
    global_tick = Incr.Var.create 0;
    stamp = 0;
  }

let rebuild_deps graph (sheet : Sheet.t) =
  let n = graph.n in
  for i = 0 to n - 1 do
    graph.forward.(i) <- [];
    graph.reverse.(i) <- []
  done;
  for i = 0 to n - 1 do
    match sheet.cells.(i) with
    | Sheet.Cell_formula formula ->
      let deps = Parser.collect_cell_deps formula sheet.max_columns sheet.max_rows in
      let dep_indices_rev = ref [] in
      List.iter (fun (c, r) ->
        if Sheet.in_bounds sheet c r then begin
          let di = Sheet.idx sheet c r in
          dep_indices_rev := di :: !dep_indices_rev;
          graph.reverse.(di) <- i :: graph.reverse.(di)
        end
      ) deps;
      graph.forward.(i) <- List.rev !dep_indices_rev
    | _ -> ()
  done

let dep_signature_node nodes tick_watch deps =
  let dep_count = List.length deps in
  let inputs = Array.make (dep_count + 1) tick_watch in
  let rec fill pos = function
    | [] -> ()
    | di :: tl ->
      inputs.(pos) <- nodes.(di);
      fill (pos + 1) tl
  in
  fill 1 deps;
  match Incr.reduce_balanced inputs ~f:(fun x -> x) ~reduce:( + ) with
  | Some node -> node
  | None -> tick_watch

let rebuild_runtime rt graph (sheet : Sheet.t) =
  ensure_height_budget ();
  let n = graph.n in
  Array.iter (function
    | Some obs -> Incr.Observer.disallow_future_use obs
    | None -> ()
  ) rt.observers;
  rt.vars <- Array.init n (fun _ -> Incr.Var.create 0);
  rt.nodes <- Array.map Incr.Var.watch rt.vars;
  rt.observers <- Array.make n None;
  rt.changed_flag <- Array.make n false;
  rt.changed_formula_rev <- [];
  rt.global_tick <- Incr.Var.create 0;
  rt.stamp <- 0;

  let tick_watch = Incr.Var.watch rt.global_tick in
  for i = 0 to n - 1 do
    match sheet.cells.(i) with
    | Sheet.Cell_formula _ ->
      let node = dep_signature_node rt.nodes tick_watch graph.forward.(i) in
      rt.nodes.(i) <- node
    | _ -> ()
  done;

  for i = 0 to n - 1 do
    match sheet.cells.(i) with
    | Sheet.Cell_formula _ ->
      let obs = Incr.observe rt.nodes.(i) in
      Incr.Observer.on_update_exn obs ~f:(fun update ->
        match update with
        | Incr.Observer.Update.Initialized _
        | Incr.Observer.Update.Changed _
        | Incr.Observer.Update.Invalidated ->
          if not rt.changed_flag.(i) then begin
            rt.changed_flag.(i) <- true;
            rt.changed_formula_rev <- i :: rt.changed_formula_rev
          end
      );
      rt.observers.(i) <- Some obs
    | _ -> ()
  done;
  Incr.stabilize ();
  List.iter (fun i -> rt.changed_flag.(i) <- false) rt.changed_formula_rev;
  rt.changed_formula_rev <- []

let touch_cell rt i =
  if i >= 0 && i < Array.length rt.vars then begin
    rt.stamp <- rt.stamp + 1;
    Incr.Var.set rt.vars.(i) rt.stamp
  end

let touch_global rt =
  rt.stamp <- rt.stamp + 1;
  Incr.Var.set rt.global_tick rt.stamp

let stabilize_and_take_changed_formulas rt =
  rt.changed_formula_rev <- [];
  Incr.stabilize ();
  let changed = List.rev rt.changed_formula_rev in
  List.iter (fun i -> rt.changed_flag.(i) <- false) changed;
  rt.changed_formula_rev <- [];
  changed

(* Detect cycles using DFS coloring. Returns array of booleans marking cycle members. *)
let detect_cycles graph =
  let n = graph.n in
  let color = Array.make n 0 in  (* 0=white, 1=gray, 2=black *)
  let in_cycle = Array.make n false in
  let rec dfs node =
    color.(node) <- 1;
    List.iter (fun dep ->
      if dep >= 0 && dep < n then begin
        if color.(dep) = 1 then begin
          in_cycle.(dep) <- true;
          in_cycle.(node) <- true
        end else if color.(dep) = 0 then begin
          dfs dep;
          if in_cycle.(dep) then in_cycle.(node) <- true
        end
      end
    ) graph.forward.(node);
    color.(node) <- 2
  in
  for i = 0 to n - 1 do
    if color.(i) = 0 then dfs i
  done;
  in_cycle

(* Topological sort for evaluation order *)
let topo_sort graph =
  let n = graph.n in
  let in_degree = Array.make n 0 in
  for i = 0 to n - 1 do
    List.iter (fun dep ->
      if dep >= 0 && dep < n then
        in_degree.(i) <- in_degree.(i) + 1
    ) graph.forward.(i)
  done;
  let queue = Queue.create () in
  for i = 0 to n - 1 do
    if in_degree.(i) = 0 then Queue.push i queue
  done;
  let order = ref [] in
  while not (Queue.is_empty queue) do
    let node = Queue.pop queue in
    order := node :: !order;
    List.iter (fun dep_of ->
      if dep_of >= 0 && dep_of < n then begin
        in_degree.(dep_of) <- in_degree.(dep_of) - 1;
        if in_degree.(dep_of) = 0 then Queue.push dep_of queue
      end
    ) graph.reverse.(node)
  done;
  List.rev !order
