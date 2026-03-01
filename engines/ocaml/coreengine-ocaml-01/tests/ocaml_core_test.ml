let () =
  let r = Coreengine_ocaml01.Ocaml_core.create () in
  let v1 = Coreengine_ocaml01.Ocaml_core.next_unit r in
  let v2 = Coreengine_ocaml01.Ocaml_core.next_unit r in
  if v1 = v2 then failwith "deterministic stream did not advance"
