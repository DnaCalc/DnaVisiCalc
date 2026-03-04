let expect cond msg =
  if not cond then failwith msg

let number_of_state = function
  | Coreengine_ocaml01.Types.Number n -> n
  | _ -> failwith "expected numeric value"

let () =
  let engine =
    match Coreengine_ocaml01.Engine.create_default () with
    | Ok e -> e
    | Error status -> failwith (Printf.sprintf "engine create failed: %d" status)
  in

  let a1 = { Coreengine_ocaml01.Address.col = 1; Coreengine_ocaml01.Address.row = 1 } in
  let b1 = { Coreengine_ocaml01.Address.col = 2; Coreengine_ocaml01.Address.row = 1 } in
  let c1 = { Coreengine_ocaml01.Address.col = 3; Coreengine_ocaml01.Address.row = 1 } in
  let c2 = { Coreengine_ocaml01.Address.col = 3; Coreengine_ocaml01.Address.row = 2 } in
  let d1 = { Coreengine_ocaml01.Address.col = 4; Coreengine_ocaml01.Address.row = 1 } in

  expect (Coreengine_ocaml01.Engine.set_number engine a1 10.0 = Coreengine_ocaml01.Types.Status.ok) "set number failed";
  expect (Coreengine_ocaml01.Engine.set_formula engine b1 "A1+5" = Coreengine_ocaml01.Types.Status.ok) "set formula failed";

  let b1_state =
    match Coreengine_ocaml01.Engine.get_state engine b1 with
    | Ok state -> state
    | Error status -> failwith (Printf.sprintf "get state failed: %d" status)
  in
  let b1_value = number_of_state b1_state.Coreengine_ocaml01.Types.value in
  expect (abs_float (b1_value -. 15.0) < 1e-9) "A1+5 should equal 15";

  expect
    (Coreengine_ocaml01.Engine.set_recalc_mode engine Coreengine_ocaml01.Types.Recalc_mode.manual
     = Coreengine_ocaml01.Types.Status.ok)
    "set manual recalc mode failed";
  expect
    (Coreengine_ocaml01.Engine.set_number engine a1 20.0 = Coreengine_ocaml01.Types.Status.ok)
    "manual set number failed";
  let b1_stale_manual =
    match Coreengine_ocaml01.Engine.get_state engine b1 with
    | Ok state -> state
    | Error status -> failwith (Printf.sprintf "get state (manual) failed: %d" status)
  in
  expect b1_stale_manual.Coreengine_ocaml01.Types.stale "B1 should be stale in manual mode before recalc";
  expect
    (Coreengine_ocaml01.Engine.recalculate engine = Coreengine_ocaml01.Types.Status.ok)
    "manual recalculate failed";
  let b1_after_manual =
    match Coreengine_ocaml01.Engine.get_state engine b1 with
    | Ok state -> state
    | Error status -> failwith (Printf.sprintf "get state (after manual recalc) failed: %d" status)
  in
  let b1_after_value = number_of_state b1_after_manual.Coreengine_ocaml01.Types.value in
  expect (not b1_after_manual.Coreengine_ocaml01.Types.stale) "B1 should be stable after manual recalc";
  expect (abs_float (b1_after_value -. 25.0) < 1e-9) "A1+5 should equal 25 after manual recalc";
  expect
    (Coreengine_ocaml01.Engine.set_recalc_mode engine Coreengine_ocaml01.Types.Recalc_mode.automatic
     = Coreengine_ocaml01.Types.Status.ok)
    "set automatic recalc mode failed";

  expect
    (Coreengine_ocaml01.Engine.set_formula engine c1 "SEQUENCE(2,2,1,1)" = Coreengine_ocaml01.Types.Status.ok)
    "set SEQUENCE formula failed";
  let role_c1 =
    match Coreengine_ocaml01.Engine.spill_role engine c1 with
    | Ok role -> role
    | Error status -> failwith (Printf.sprintf "spill role failed: %d" status)
  in
  let role_c2 =
    match Coreengine_ocaml01.Engine.spill_role engine c2 with
    | Ok role -> role
    | Error status -> failwith (Printf.sprintf "spill role failed: %d" status)
  in
  expect (role_c1 = Coreengine_ocaml01.Types.Spill_role.anchor) "C1 should be spill anchor";
  expect (role_c2 = Coreengine_ocaml01.Types.Spill_role.member_) "C2 should be spill member";

  expect (Coreengine_ocaml01.Engine.set_formula engine d1 "STREAM(1)" = Coreengine_ocaml01.Types.Status.ok) "set STREAM formula failed";
  let tick_status, advanced = Coreengine_ocaml01.Engine.tick_streams engine 1.2 in
  expect (tick_status = Coreengine_ocaml01.Types.Status.ok) "tick streams failed";
  expect advanced "stream should advance after 1.2s on period 1";

  let d1_state =
    match Coreengine_ocaml01.Engine.get_state engine d1 with
    | Ok state -> state
    | Error status -> failwith (Printf.sprintf "get stream state failed: %d" status)
  in
  let d1_value = number_of_state d1_state.Coreengine_ocaml01.Types.value in
  expect (abs_float (d1_value -. 1.0) < 1e-9) "STREAM counter should be 1 after first tick"
