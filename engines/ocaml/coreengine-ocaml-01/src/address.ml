(* address.ml - Cell address parsing and manipulation *)

type t = { col : int; row : int }

let make ~col ~row = { col; row }

let col_to_letters c =
  let buf = Buffer.create 3 in
  let rec go n =
    if n <= 0 then ()
    else begin
      go ((n - 1) / 26);
      Buffer.add_char buf (Char.chr (Char.code 'A' + ((n - 1) mod 26)))
    end
  in
  go c;
  Buffer.contents buf

let letters_to_col s start len =
  let result = ref 0 in
  for i = start to start + len - 1 do
    let c = Char.code (Char.uppercase_ascii s.[i]) - Char.code 'A' + 1 in
    result := !result * 26 + c
  done;
  !result

let parse_a1 s pos =
  let len = String.length s in
  if pos >= len then None
  else
    let i = ref pos in
    let col_abs = if !i < len && s.[!i] = '$' then (incr i; true) else false in
    let col_start = !i in
    while !i < len && (let c = s.[!i] in (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) do
      incr i
    done;
    let col_len = !i - col_start in
    if col_len = 0 || col_len > 3 then None
    else
      let row_abs = if !i < len && s.[!i] = '$' then (incr i; true) else false in
      let row_start = !i in
      if row_start >= len || s.[row_start] < '1' || s.[row_start] > '9' then None
      else begin
        while !i < len && s.[!i] >= '0' && s.[!i] <= '9' do
          incr i
        done;
        let row_str = String.sub s row_start (!i - row_start) in
        let row = int_of_string row_str in
        if row > 65535 then None
        else
          let col = letters_to_col s col_start col_len in
          Some (col, row, !i - pos, col_abs, row_abs)
      end

let parse_r1c1 s pos =
  let len = String.length s in
  if pos >= len then None
  else
    let i = ref pos in
    if !i < len && (s.[!i] = 'R' || s.[!i] = 'r') then begin
      incr i;
      let row_start = !i in
      while !i < len && s.[!i] >= '0' && s.[!i] <= '9' do incr i done;
      if !i = row_start then None
      else
        let row = int_of_string (String.sub s row_start (!i - row_start)) in
        if !i < len && (s.[!i] = 'C' || s.[!i] = 'c') then begin
          incr i;
          let col_start = !i in
          while !i < len && s.[!i] >= '0' && s.[!i] <= '9' do incr i done;
          if !i = col_start then None
          else
            let col = int_of_string (String.sub s col_start (!i - col_start)) in
            Some (col, row, !i - pos)
        end else None
    end else None

let to_a1 ?(col_abs=false) ?(row_abs=false) addr =
  let cb = if col_abs then "$" else "" in
  let rb = if row_abs then "$" else "" in
  Printf.sprintf "%s%s%s%d" cb (col_to_letters addr.col) rb addr.row

let is_cell_like s =
  let upper = String.uppercase_ascii s in
  match parse_a1 upper 0 with
  | Some (_, _, consumed, _, _) -> consumed = String.length upper
  | None -> false

let palette_names = [|
  "Mist"; "Sage"; "Fern"; "Moss"; "Olive"; "Seafoam"; "Lagoon"; "Teal";
  "Sky"; "Cloud"; "Sand"; "Clay"; "Peach"; "Rose"; "Lavender"; "Slate"
|]
