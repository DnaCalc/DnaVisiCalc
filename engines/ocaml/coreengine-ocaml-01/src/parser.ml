(* parser.ml - Formula text scanning and reference extraction.
   The engine uses a text-based evaluator (matching the C reference),
   so this module provides helpers for reference extraction, rewriting,
   and function-name extraction from formula text. *)

let is_alpha c = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
let is_digit c = c >= '0' && c <= '9'
let is_alnum c = is_alpha c || is_digit c

(* Skip over a quoted string starting at pos (pos points at opening '"') *)
let skip_string s pos =
  let len = String.length s in
  let i = ref (pos + 1) in
  while !i < len && s.[!i] <> '"' do incr i done;
  if !i < len then !i + 1 else !i

(* Check if position is at a boundary (not preceded by alnum) *)
let at_boundary s pos =
  pos = 0 || not (is_alnum s.[pos - 1])

(* Extract function name at pos: sequence of alpha/digit/dot/underscore *)
let extract_ident s pos =
  let len = String.length s in
  let i = ref pos in
  while !i < len && (is_alnum s.[!i] || s.[!i] = '_' || s.[!i] = '.') do incr i done;
  String.sub s pos (!i - pos)

(* Collect all cell reference dependencies from a formula.
   Returns list of (col, row) pairs. *)
let collect_cell_deps formula max_col max_row =
  let deps = Hashtbl.create 16 in
  let len = String.length formula in
  let i = ref 0 in
  while !i < len do
    if formula.[!i] = '"' then
      i := skip_string formula !i
    else if at_boundary formula !i then begin
      match Address.parse_a1 formula !i with
      | Some (col, row, consumed, _, _) when col >= 1 && col <= max_col && row >= 1 && row <= max_row ->
        (* Check for range *)
        let after = !i + consumed in
        if after < len && formula.[after] = '#' then begin
          (* Spill ref - just add the anchor *)
          Hashtbl.replace deps (col, row) ();
          i := after + 1
        end else if after + 1 < len && formula.[after] = ':' then begin
          match Address.parse_a1 formula (after + 1) with
          | Some (c2, r2, consumed2, _, _) ->
            let c1 = min col c2 and c2 = max col c2 in
            let r1 = min row r2 and r2 = max row r2 in
            for c = c1 to c2 do
              for r = r1 to r2 do
                Hashtbl.replace deps (c, r) ()
              done
            done;
            i := after + 1 + consumed2
          | None ->
            Hashtbl.replace deps (col, row) ();
            i := after
        end else if after + 3 < len && formula.[after] = '.' && formula.[after+1] = '.' && formula.[after+2] = '.' then begin
          match Address.parse_a1 formula (after + 3) with
          | Some (c2, r2, consumed2, _, _) ->
            let c1 = min col c2 and c2 = max col c2 in
            let r1 = min row r2 and r2 = max row r2 in
            for c = c1 to c2 do
              for r = r1 to r2 do
                Hashtbl.replace deps (c, r) ()
              done
            done;
            i := after + 3 + consumed2
          | None ->
            Hashtbl.replace deps (col, row) ();
            i := after
        end else begin
          Hashtbl.replace deps (col, row) ();
          i := after
        end
      | _ -> incr i
    end else
      incr i
  done;
  Hashtbl.fold (fun (c, r) () acc -> (c, r) :: acc) deps []

(* Check if formula contains a specific function name (case-insensitive) *)
let formula_has_function formula fname =
  let upper = String.uppercase_ascii formula in
  let fupper = String.uppercase_ascii fname in
  let flen = String.length fupper in
  let ulen = String.length upper in
  let found = ref false in
  let i = ref 0 in
  while !i <= ulen - flen && not !found do
    if upper.[!i] = '"' then
      i := skip_string upper !i
    else if at_boundary upper !i &&
            String.sub upper !i flen = fupper &&
            (!i + flen >= ulen || upper.[!i + flen] = '(' || not (is_alnum upper.[!i + flen])) then
      found := true
    else
      incr i
  done;
  !found
