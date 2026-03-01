type t = { mutable seed : int64 }

let create () = { seed = 0x1234_5678L }

let next_u64 t =
  let a = 6364136223846793005L in
  let c = 1442695040888963407L in
  t.seed <- Int64.add (Int64.mul t.seed a) c;
  t.seed

let next_unit t =
  let v = next_u64 t in
  Int64.to_float (Int64.logand v 0x000f_ffff_ffff_ffffL) /. Int64.to_float 0x0010_0000_0000_0000L

let deterministic_perturb prev =
  let x = prev *. 1.0000001 +. 0.000001 in
  if x > 1.0 then x -. floor x else x
