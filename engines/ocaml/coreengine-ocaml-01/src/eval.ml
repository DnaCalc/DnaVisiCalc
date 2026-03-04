(* eval.ml - Formula evaluator for DnaVisiCalc OCaml engine.
   Implements text-based evaluation matching the C reference engine's semantics:
   - Left-to-right operator scanning (no precedence beyond operator ordering)
   - Comparison operators evaluated before arithmetic
   - Depth-limited recursion (max 16)
   - All standard spreadsheet functions *)

type eval_ctx = {
  sheet : Sheet.t;
  self_col : int;
  self_row : int;
  committed_epoch : int;
  rand_counter : int ref;
  let_bindings : (string * float) list;
  names : (string * Sheet.cell_input) list;
  udfs : (string * (Types.value array -> Types.value)) list;
}

type eval_result =
  | Num of float
  | Txt of string
  | Bln of bool
  | Blk
  | Err of Types.error_kind * string
  | Arr of eval_result array array  (* rows x cols *)

type cmp_op =
  | Cmp_le
  | Cmp_ge
  | Cmp_ne
  | Cmp_lt
  | Cmp_gt
  | Cmp_eq

type compiled_expr =
  | CNum of float
  | CBool of bool
  | CRef of int * int * bool  (* col, row, spill-ref *)
  | CNeg of compiled_expr
  | CAdd of compiled_expr * compiled_expr
  | CSub of compiled_expr * compiled_expr
  | CMul of compiled_expr * compiled_expr
  | CDiv of compiled_expr * compiled_expr
  | CCompare of cmp_op * compiled_expr * compiled_expr
  | CRound of compiled_expr * compiled_expr

let result_to_value = function
  | Num n -> Types.Number n
  | Txt s -> Types.Text s
  | Bln b -> Types.Bool b
  | Blk -> Types.Blank
  | Err (k, m) -> Types.Error (k, m)
  | Arr _ -> Types.Number 0.0

let value_to_result = function
  | Types.Number n -> Num n
  | Types.Text s -> Txt s
  | Types.Bool b -> Bln b
  | Types.Blank -> Blk
  | Types.Error (k, m) -> Err (k, m)

let result_to_num = function
  | Num n -> Some n
  | Bln true -> Some 1.0
  | Bln false -> Some 0.0
  | Blk -> Some 0.0
  | _ -> None

let is_error = function Err _ -> true | _ -> false

let strip_eq s =
  let s = String.trim s in
  if String.length s > 0 && s.[0] = '=' then String.sub s 1 (String.length s - 1)
  else s

let is_space = function
  | ' ' | '\t' | '\r' | '\n' -> true
  | _ -> false

let skip_ws s pos =
  let len = String.length s in
  let i = ref pos in
  while !i < len && is_space s.[!i] do incr i done;
  !i

(* Golden ratio hash for deterministic pseudo-random *)
let golden_hash seed =
  let h = ref (seed * 2654435761) in
  h := !h lxor (!h lsr 16);
  h := !h * 2246822507;
  h := !h lxor (!h lsr 13);
  h := !h * 3266489909;
  h := !h lxor (!h lsr 16);
  (Float.of_int (abs (!h mod 1000000000))) /. 1000000000.0

(* Parse a number from string *)
let parse_number s pos =
  let len = String.length s in
  let i = ref pos in
  let neg = if !i < len && s.[!i] = '-' then (incr i; true) else false in
  let start = !i in
  while !i < len && ((s.[!i] >= '0' && s.[!i] <= '9') || s.[!i] = '.') do incr i done;
  (* Handle scientific notation *)
  if !i < len && (s.[!i] = 'e' || s.[!i] = 'E') then begin
    incr i;
    if !i < len && (s.[!i] = '+' || s.[!i] = '-') then incr i;
    while !i < len && s.[!i] >= '0' && s.[!i] <= '9' do incr i done
  end;
  if !i > start then
    let ns = String.sub s start (!i - start) in
    match float_of_string_opt ns with
    | Some n -> Some ((if neg then -.n else n), !i)
    | None -> None
  else if neg then None
  else None

let compile_cmp_of_token = function
  | "<=" -> Some Cmp_le
  | ">=" -> Some Cmp_ge
  | "<>" -> Some Cmp_ne
  | "<" -> Some Cmp_lt
  | ">" -> Some Cmp_gt
  | "=" -> Some Cmp_eq
  | _ -> None

let try_compile formula =
  let s = strip_eq formula in
  let len = String.length s in

  let rec parse_expr pos = parse_comparison pos

  and parse_comparison pos =
    match parse_additive pos with
    | None -> None
    | Some (lhs, after_lhs) ->
      let i = skip_ws s after_lhs in
      let token, consumed =
        if i + 1 < len && s.[i] = '<' && s.[i + 1] = '=' then ("<=", 2)
        else if i + 1 < len && s.[i] = '>' && s.[i + 1] = '=' then (">=", 2)
        else if i + 1 < len && s.[i] = '<' && s.[i + 1] = '>' then ("<>", 2)
        else if i < len && (s.[i] = '<' || s.[i] = '>' || s.[i] = '=') then (String.make 1 s.[i], 1)
        else ("", 0)
      in
      if consumed = 0 then Some (lhs, after_lhs)
      else
        match compile_cmp_of_token token, parse_additive (i + consumed) with
        | Some op, Some (rhs, after_rhs) -> Some (CCompare (op, lhs, rhs), after_rhs)
        | _ -> None

  and parse_additive pos =
    match parse_multiplicative pos with
    | None -> None
    | Some (lhs0, after_lhs0) ->
      let rec loop lhs after_lhs =
        let i = skip_ws s after_lhs in
        if i < len && (s.[i] = '+' || s.[i] = '-') then
          match parse_multiplicative (i + 1) with
          | Some (rhs, after_rhs) ->
            let lhs' = if s.[i] = '+' then CAdd (lhs, rhs) else CSub (lhs, rhs) in
            loop lhs' after_rhs
          | None -> None
        else
          Some (lhs, after_lhs)
      in
      loop lhs0 after_lhs0

  and parse_multiplicative pos =
    match parse_unary pos with
    | None -> None
    | Some (lhs0, after_lhs0) ->
      let rec loop lhs after_lhs =
        let i = skip_ws s after_lhs in
        if i < len && (s.[i] = '*' || s.[i] = '/') then
          match parse_unary (i + 1) with
          | Some (rhs, after_rhs) ->
            let lhs' = if s.[i] = '*' then CMul (lhs, rhs) else CDiv (lhs, rhs) in
            loop lhs' after_rhs
          | None -> None
        else
          Some (lhs, after_lhs)
      in
      loop lhs0 after_lhs0

  and parse_unary pos =
    let i = skip_ws s pos in
    if i < len && s.[i] = '-' then
      match parse_unary (i + 1) with
      | Some (expr, after_expr) -> Some (CNeg expr, after_expr)
      | None -> None
    else
      parse_primary i

  and parse_primary pos =
    let i = skip_ws s pos in
    if i >= len then None
    else if s.[i] = '(' then
      match parse_expr (i + 1) with
      | Some (inner, after_inner) ->
        let j = skip_ws s after_inner in
        if j < len && s.[j] = ')' then Some (inner, j + 1) else None
      | None -> None
    else
      match parse_number s i with
      | Some (n, consumed) when consumed > i -> Some (CNum n, consumed)
      | _ ->
        (* Cell refs are valid primary terms and should win over plain identifiers. *)
        (match Address.parse_a1 s i with
         | Some (col, row, consumed, _, _) when consumed > 0 ->
           let j = i + consumed in
           if j < len && s.[j] = '#' then Some (CRef (col, row, true), j + 1)
           else Some (CRef (col, row, false), j)
         | _ ->
           let j = ref i in
           while !j < len &&
                 (((s.[!j] >= 'A' && s.[!j] <= 'Z') || (s.[!j] >= 'a' && s.[!j] <= 'z')) ||
                  (s.[!j] >= '0' && s.[!j] <= '9') || s.[!j] = '_' || s.[!j] = '.') do
             incr j
           done;
           if !j = i then None
           else
             let ident = String.uppercase_ascii (String.sub s i (!j - i)) in
             let k = skip_ws s !j in
             if ident = "TRUE" && k = !j then Some (CBool true, !j)
             else if ident = "FALSE" && k = !j then Some (CBool false, !j)
             else if k < len && s.[k] = '(' then
               let rec parse_args pos_acc acc =
                 let p = skip_ws s pos_acc in
                 if p < len && s.[p] = ')' then Some (List.rev acc, p + 1)
                 else
                   match parse_expr p with
                   | None -> None
                   | Some (arg_expr, after_arg) ->
                     let q = skip_ws s after_arg in
                     if q < len && s.[q] = ',' then parse_args (q + 1) (arg_expr :: acc)
                     else if q < len && s.[q] = ')' then Some (List.rev (arg_expr :: acc), q + 1)
                     else None
               in
               match ident, parse_args (k + 1) [] with
               | "ROUND", Some ([x; d], after_call) -> Some (CRound (x, d), after_call)
               | _ -> None
             else
               None)
  in

  match parse_expr 0 with
  | Some (expr, after_expr) ->
    if skip_ws s after_expr = len then Some expr else None
  | None -> None

let eval_cmp op a b =
  match op with
  | Cmp_le -> a <= b
  | Cmp_ge -> a >= b
  | Cmp_ne -> a <> b
  | Cmp_lt -> a < b
  | Cmp_gt -> a > b
  | Cmp_eq -> a = b

let eval_compiled ctx compiled depth =
  let rec eval expr level =
    if level > 16 then Err (Types.Err_value, "recursion depth exceeded")
    else
      match expr with
      | CNum n -> Num n
      | CBool b -> Bln b
      | CRef (col, row, spill) ->
        if not (Sheet.in_bounds ctx.sheet col row) then
          Err (Types.Err_ref, "#REF!")
        else
          let i = Sheet.idx ctx.sheet col row in
          let c = ctx.sheet.computed.(i) in
          if spill then
            if c.spill_role = 1 then Num (Types.value_number c.value)
            else value_to_result c.value
          else
            value_to_result c.value
      | CNeg inner ->
        (match eval inner (level + 1) with
         | Num n -> Num (-.n)
         | Bln true -> Num (-1.0)
         | Bln false -> Num 0.0
         | Blk -> Num 0.0
         | other -> other)
      | CAdd (l, r) ->
        let lv = eval l (level + 1) in
        let rv = eval r (level + 1) in
        (match result_to_num lv, result_to_num rv with
         | Some a, Some b -> Num (a +. b)
         | _ -> if is_error lv then lv else if is_error rv then rv else Err (Types.Err_value, "#VALUE!"))
      | CSub (l, r) ->
        let lv = eval l (level + 1) in
        let rv = eval r (level + 1) in
        (match result_to_num lv, result_to_num rv with
         | Some a, Some b -> Num (a -. b)
         | _ -> if is_error lv then lv else if is_error rv then rv else Err (Types.Err_value, "#VALUE!"))
      | CMul (l, r) ->
        let lv = eval l (level + 1) in
        let rv = eval r (level + 1) in
        (match result_to_num lv, result_to_num rv with
         | Some a, Some b -> Num (a *. b)
         | _ -> if is_error lv then lv else if is_error rv then rv else Err (Types.Err_value, "#VALUE!"))
      | CDiv (l, r) ->
        let lv = eval l (level + 1) in
        let rv = eval r (level + 1) in
        (match result_to_num lv, result_to_num rv with
         | Some _, Some 0.0 -> Err (Types.Div_zero, "#DIV/0!")
         | Some a, Some b -> Num (a /. b)
         | _ -> if is_error lv then lv else if is_error rv then rv else Err (Types.Err_value, "#VALUE!"))
      | CCompare (op, l, r) ->
        let lv = eval l (level + 1) in
        let rv = eval r (level + 1) in
        (match result_to_num lv, result_to_num rv with
         | Some a, Some b -> Bln (eval_cmp op a b)
         | _ -> if is_error lv then lv else if is_error rv then rv else Err (Types.Err_value, "#VALUE!"))
      | CRound (x, d) ->
        let xv = eval x (level + 1) in
        let dv = eval d (level + 1) in
        (match result_to_num xv, result_to_num dv with
         | Some n, Some digits ->
           let mult = 10.0 ** (floor digits) in
           Num (Float.round (n *. mult) /. mult)
         | _ -> if is_error xv then xv else if is_error dv then dv else Err (Types.Err_value, "#VALUE!"))
  in
  eval compiled depth

(* Find matching closing paren *)
let find_close_paren s pos =
  let len = String.length s in
  let depth = ref 1 in
  let i = ref (pos + 1) in
  while !i < len && !depth > 0 do
    if s.[!i] = '(' then incr depth
    else if s.[!i] = ')' then decr depth
    else if s.[!i] = '"' then begin
      incr i;
      while !i < len && s.[!i] <> '"' do incr i done
    end;
    if !depth > 0 then incr i
  done;
  if !depth = 0 then Some !i else None

(* Split arguments at top-level commas *)
let split_args s =
  let len = String.length s in
  if len = 0 then []
  else begin
    let args = ref [] in
    let depth = ref 0 in
    let start = ref 0 in
    let in_str = ref false in
    for i = 0 to len - 1 do
      if !in_str then begin
        if s.[i] = '"' then in_str := false
      end else begin
        if s.[i] = '"' then in_str := true
        else if s.[i] = '(' then incr depth
        else if s.[i] = ')' then decr depth
        else if s.[i] = ',' && !depth = 0 then begin
          args := String.sub s !start (i - !start) :: !args;
          start := i + 1
        end
      end
    done;
    args := String.sub s !start (len - !start) :: !args;
    List.rev !args
  end

(* Parse range reference like A1:B5 or A1...B5 *)
let parse_range s =
  let s = String.trim s in
  let try_parse sep sep_len =
    match Address.parse_a1 s 0 with
    | Some (c1, r1, consumed1, _, _) ->
      let after = consumed1 in
      if after + sep_len <= String.length s &&
         String.sub s after sep_len = sep then
        match Address.parse_a1 s (after + sep_len) with
        | Some (c2, r2, consumed2, _, _) when after + sep_len + consumed2 = String.length s ->
          Some (c1, r1, c2, r2)
        | _ -> None
      else None
    | None -> None
  in
  match try_parse ":" 1 with
  | Some _ as r -> r
  | None -> try_parse "..." 3

(* Evaluate range values as flat list of results *)
let eval_range_values ctx c1 r1 c2 r2 =
  let results = ref [] in
  let sc1 = min c1 c2 and sc2 = max c1 c2 in
  let sr1 = min r1 r2 and sr2 = max r1 r2 in
  for r = sr1 to sr2 do
    for c = sc1 to sc2 do
      if Sheet.in_bounds ctx.sheet c r then begin
        let i = Sheet.idx ctx.sheet c r in
        results := value_to_result ctx.sheet.computed.(i).value :: !results
      end
    done
  done;
  List.rev !results

(* Expand an argument that may be a range into a list of values *)
let expand_arg ctx arg =
  let arg = String.trim arg in
  match parse_range arg with
  | Some (c1, r1, c2, r2) -> eval_range_values ctx c1 r1 c2 r2
  | None ->
    (* Check for spill reference: ADDR# *)
    let len = String.length arg in
    if len > 1 && arg.[len-1] = '#' then begin
      let ref_part = String.sub arg 0 (len - 1) in
      match Address.parse_a1 ref_part 0 with
      | Some (col, row, consumed, _, _) when consumed = String.length ref_part ->
        if Sheet.in_bounds ctx.sheet col row then begin
          let i = Sheet.idx ctx.sheet col row in
          let c = ctx.sheet.computed.(i) in
          if c.spill_role = 1 then  (* anchor *)
            eval_range_values ctx
              c.spill_range_start_col c.spill_range_start_row
              c.spill_range_end_col c.spill_range_end_row
          else [value_to_result ctx.sheet.computed.(i).value]
        end else [Err (Types.Err_ref, "#REF!")]
      | _ -> [Err (Types.Err_ref, "#REF!")]
    end else
      [] (* caller will eval as expression *)

(* Main expression evaluator *)
let rec eval_expr ctx expr depth =
  if depth > 16 then Err (Types.Err_value, "recursion depth exceeded")
  else
    let expr = String.trim expr in
    let expr = if String.length expr > 0 && expr.[0] = '=' then
      String.sub expr 1 (String.length expr - 1) |> String.trim
    else expr in
    if String.length expr = 0 then Blk
    else eval_comparison ctx expr depth

and eval_comparison ctx expr depth =
  (* Find comparison operators at top level *)
  let len = String.length expr in
  let pdepth = ref 0 in
  let in_str = ref false in
  let op_pos = ref (-1) in
  let op_len = ref 0 in
  let i = ref 0 in
  while !i < len && !op_pos = -1 do
    if !in_str then begin
      if expr.[!i] = '"' then in_str := false;
      incr i
    end else if expr.[!i] = '"' then begin
      in_str := true; incr i
    end else if expr.[!i] = '(' then begin
      incr pdepth; incr i
    end else if expr.[!i] = ')' then begin
      decr pdepth; incr i
    end else if !pdepth = 0 then begin
      if !i + 1 < len && expr.[!i] = '<' && expr.[!i+1] = '=' then
        (op_pos := !i; op_len := 2)
      else if !i + 1 < len && expr.[!i] = '>' && expr.[!i+1] = '=' then
        (op_pos := !i; op_len := 2)
      else if !i + 1 < len && expr.[!i] = '<' && expr.[!i+1] = '>' then
        (op_pos := !i; op_len := 2)
      else if expr.[!i] = '<' then
        (op_pos := !i; op_len := 1)
      else if expr.[!i] = '>' then
        (op_pos := !i; op_len := 1)
      else if expr.[!i] = '=' then
        (op_pos := !i; op_len := 1)
      else incr i
    end else incr i
  done;
  if !op_pos > 0 then begin
    let lhs = String.sub expr 0 !op_pos in
    let rhs = String.sub expr (!op_pos + !op_len) (len - !op_pos - !op_len) in
    let op = String.sub expr !op_pos !op_len in
    let lv = eval_additive ctx lhs depth in
    let rv = eval_additive ctx rhs depth in
    match result_to_num lv, result_to_num rv with
    | Some a, Some b ->
      let result = match op with
        | "<=" -> a <= b | ">=" -> a >= b | "<>" -> a <> b
        | "<" -> a < b | ">" -> a > b | "=" -> a = b
        | _ -> false
      in
      Bln result
    | _ ->
      if is_error lv then lv
      else if is_error rv then rv
      else Err (Types.Err_value, "#VALUE!")
  end else
    eval_additive ctx expr depth

and eval_additive ctx expr depth =
  (* Scan for + and - at top level, left to right *)
  let len = String.length expr in
  let pdepth = ref 0 in
  let in_str = ref false in
  let last_op = ref (-1) in
  let i = ref 0 in
  while !i < len do
    if !in_str then begin
      if expr.[!i] = '"' then in_str := false;
      incr i
    end else if expr.[!i] = '"' then begin
      in_str := true; incr i
    end else if expr.[!i] = '(' then begin
      incr pdepth; incr i
    end else if expr.[!i] = ')' then begin
      decr pdepth; incr i
    end else if !pdepth = 0 && !i > 0 && (expr.[!i] = '+' || expr.[!i] = '-') then begin
      (* Check it's not a sign after operator or start *)
      let prev = expr.[!i - 1] in
      if prev <> '(' && prev <> ',' && prev <> '+' && prev <> '-' && prev <> '*' && prev <> '/' && prev <> 'E' && prev <> 'e' then
        last_op := !i;
      incr i
    end else
      incr i
  done;
  if !last_op > 0 then begin
    let lhs = String.sub expr 0 !last_op in
    let rhs = String.sub expr (!last_op + 1) (len - !last_op - 1) in
    let op = expr.[!last_op] in
    let lv = eval_additive ctx lhs depth in
    let rv = eval_multiplicative ctx rhs depth in
    match result_to_num lv, result_to_num rv with
    | Some a, Some b ->
      if op = '+' then Num (a +. b) else Num (a -. b)
    | _ ->
      if is_error lv then lv
      else if is_error rv then rv
      else Err (Types.Err_value, "#VALUE!")
  end else
    eval_multiplicative ctx expr depth

and eval_multiplicative ctx expr depth =
  let len = String.length expr in
  let pdepth = ref 0 in
  let in_str = ref false in
  let last_op = ref (-1) in
  let i = ref 0 in
  while !i < len do
    if !in_str then begin
      if expr.[!i] = '"' then in_str := false;
      incr i
    end else if expr.[!i] = '"' then begin
      in_str := true; incr i
    end else if expr.[!i] = '(' then begin
      incr pdepth; incr i
    end else if expr.[!i] = ')' then begin
      decr pdepth; incr i
    end else if !pdepth = 0 && (expr.[!i] = '*' || expr.[!i] = '/') then begin
      last_op := !i;
      incr i
    end else
      incr i
  done;
  if !last_op > 0 then begin
    let lhs = String.sub expr 0 !last_op in
    let rhs = String.sub expr (!last_op + 1) (len - !last_op - 1) in
    let op = expr.[!last_op] in
    let lv = eval_multiplicative ctx lhs depth in
    let rv = eval_unary ctx rhs depth in
    match result_to_num lv, result_to_num rv with
    | Some a, Some b ->
      if op = '*' then Num (a *. b)
      else if b = 0.0 then Err (Types.Div_zero, "#DIV/0!")
      else Num (a /. b)
    | _ ->
      if is_error lv then lv
      else if is_error rv then rv
      else Err (Types.Err_value, "#VALUE!")
  end else
    eval_unary ctx expr depth

and eval_unary ctx expr depth =
  let expr = String.trim expr in
  let len = String.length expr in
  if len > 0 && expr.[0] = '-' then begin
    let rest = String.sub expr 1 (len - 1) in
    match eval_primary ctx rest depth with
    | Num n -> Num (-.n)
    | Bln true -> Num (-1.0)
    | Bln false -> Num 0.0
    | Blk -> Num 0.0
    | e -> e
  end else
    eval_primary ctx expr depth

and eval_primary ctx expr depth =
  let expr = String.trim expr in
  let len = String.length expr in
  if len = 0 then Blk
  (* Parenthesized expression *)
  else if expr.[0] = '(' then begin
    match find_close_paren expr 0 with
    | Some close ->
      let inner = String.sub expr 1 (close - 1) in
      eval_expr ctx inner (depth + 1)
    | None -> Err (Types.Err_value, "unmatched paren")
  end
  (* String literal *)
  else if expr.[0] = '"' then begin
    let i = ref 1 in
    while !i < len && expr.[!i] <> '"' do incr i done;
    Txt (String.sub expr 1 (!i - 1))
  end
  (* #REF! error literal *)
  else if len >= 5 && String.uppercase_ascii (String.sub expr 0 5) = "#REF!" then
    Err (Types.Err_ref, "#REF!")
  (* TRUE/FALSE literals *)
  else if String.uppercase_ascii expr = "TRUE" then Bln true
  else if String.uppercase_ascii expr = "FALSE" then Bln false
  (* Function call *)
  else if try
    let paren_pos = String.index expr '(' in
    let name_part = String.sub expr 0 paren_pos in
    let name_trimmed = String.trim name_part in
    String.length name_trimmed > 0 &&
    (let c = name_trimmed.[0] in
     (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'))
  with Not_found -> false
  then
    eval_function_call ctx expr depth
  (* Number literal *)
  else begin
    match parse_number expr 0 with
    | Some (n, consumed) when consumed = len -> Num n
    | _ ->
      (* Cell reference *)
      match Address.parse_a1 expr 0 with
      | Some (col, row, consumed, _, _) ->
        if consumed < len && expr.[consumed] = '#' then begin
          (* Spill reference *)
          if Sheet.in_bounds ctx.sheet col row then begin
            let i = Sheet.idx ctx.sheet col row in
            let c = ctx.sheet.computed.(i) in
            if c.spill_role = 1 then
              Num (Types.value_number c.value)
            else
              value_to_result c.value
          end else Err (Types.Err_ref, "#REF!")
        end else if consumed = len then begin
          if Sheet.in_bounds ctx.sheet col row then begin
            let i = Sheet.idx ctx.sheet col row in
            value_to_result ctx.sheet.computed.(i).value
          end else Err (Types.Err_ref, "#REF!")
        end else begin
          (* Could be a range like A1:B5 used as single value *)
          match parse_range expr with
          | Some (c1, r1, _, _) ->
            if Sheet.in_bounds ctx.sheet c1 r1 then begin
              let i = Sheet.idx ctx.sheet c1 r1 in
              value_to_result ctx.sheet.computed.(i).value
            end else Err (Types.Err_ref, "#REF!")
          | None ->
            (* Try named definition *)
            eval_name ctx expr depth
        end
      | None ->
        eval_name ctx expr depth
  end

and eval_name ctx name depth =
  let name = String.trim name in
  let upper = String.uppercase_ascii name in
  (* Check let bindings first *)
  match List.assoc_opt upper (List.map (fun (k,v) -> (String.uppercase_ascii k, v)) ctx.let_bindings) with
  | Some v -> Num v
  | None ->
    (* Check named definitions *)
    match List.find_opt (fun (n, _) -> String.uppercase_ascii n = upper) ctx.names with
    | Some (_, Sheet.Cell_number n) -> Num n
    | Some (_, Sheet.Cell_text s) -> Txt s
    | Some (_, Sheet.Cell_formula f) -> eval_expr ctx f (depth + 1)
    | Some (_, Sheet.Cell_empty) -> Blk
    | None -> Err (Types.Err_name, Printf.sprintf "#NAME? %s" name)

and eval_function_call ctx expr depth =
  let paren_pos = String.index expr '(' in
  let name = String.trim (String.sub expr 0 paren_pos) in
  let upper = String.uppercase_ascii name in
  (* Find matching close paren *)
  let rest_start = paren_pos in
  match find_close_paren expr rest_start with
  | None -> Err (Types.Err_value, "unmatched paren")
  | Some close_pos ->
    let args_str = String.sub expr (paren_pos + 1) (close_pos - paren_pos - 1) in
    (* Check for LAMBDA invocation: result)(args) *)
    let after_close = close_pos + 1 in
    let remaining = if after_close < String.length expr then
      String.sub expr after_close (String.length expr - after_close)
    else "" in
    if upper = "LAMBDA" then begin
      let args = split_args args_str in
      match args with
      | [param; body] ->
        let param = String.trim param in
        if String.length remaining > 0 && remaining.[0] = '(' then begin
          match find_close_paren remaining 0 with
          | Some cp ->
            let call_arg = String.sub remaining 1 (cp - 1) in
            let arg_val = eval_expr ctx call_arg (depth + 1) in
            (match result_to_num arg_val with
             | Some n ->
               let ctx' = { ctx with let_bindings = (param, n) :: ctx.let_bindings } in
               eval_expr ctx' body (depth + 1)
             | None ->
               if is_error arg_val then arg_val
               else Err (Types.Err_value, "#VALUE!"))
          | None -> Err (Types.Err_value, "unmatched paren in LAMBDA call")
        end else
          (* Return lambda as a concept - for MAP usage we handle specially *)
          Err (Types.Err_value, "LAMBDA without invocation")
      | _ -> Err (Types.Err_value, "LAMBDA requires (param, body)")
    end else
      eval_builtin ctx upper args_str depth

and eval_builtin ctx fname args_str depth =
  let args = split_args args_str in
  let eval_arg s = eval_expr ctx (String.trim s) (depth + 1) in
  let eval_arg_num s =
    match eval_arg s with
    | Num n -> Some n | Bln true -> Some 1.0 | Bln false -> Some 0.0
    | Blk -> Some 0.0 | _ -> None
  in
  match fname with
  (* Aggregate functions *)
  | "SUM" ->
    let total = ref 0.0 in
    List.iter (fun arg ->
      let expanded = expand_arg ctx arg in
      if expanded <> [] then
        List.iter (fun v -> match result_to_num v with Some n -> total := !total +. n | None -> ()) expanded
      else
        (match eval_arg_num arg with Some n -> total := !total +. n | None -> ())
    ) args;
    Num !total
  | "MIN" ->
    let vals = ref [] in
    List.iter (fun arg ->
      let expanded = expand_arg ctx arg in
      if expanded <> [] then
        List.iter (fun v -> match result_to_num v with Some n -> vals := n :: !vals | None -> ()) expanded
      else
        (match eval_arg_num arg with Some n -> vals := n :: !vals | None -> ())
    ) args;
    (match !vals with [] -> Num 0.0 | vs -> Num (List.fold_left min infinity vs))
  | "MAX" ->
    let vals = ref [] in
    List.iter (fun arg ->
      let expanded = expand_arg ctx arg in
      if expanded <> [] then
        List.iter (fun v -> match result_to_num v with Some n -> vals := n :: !vals | None -> ()) expanded
      else
        (match eval_arg_num arg with Some n -> vals := n :: !vals | None -> ())
    ) args;
    (match !vals with [] -> Num 0.0 | vs -> Num (List.fold_left max neg_infinity vs))
  | "AVERAGE" ->
    let total = ref 0.0 in
    let count = ref 0 in
    List.iter (fun arg ->
      let expanded = expand_arg ctx arg in
      if expanded <> [] then
        List.iter (fun v -> match result_to_num v with Some n -> total := !total +. n; incr count | None -> ()) expanded
      else
        (match eval_arg_num arg with Some n -> total := !total +. n; incr count | None -> ())
    ) args;
    if !count = 0 then Err (Types.Div_zero, "#DIV/0!")
    else Num (!total /. float_of_int !count)
  | "COUNT" ->
    let count = ref 0 in
    List.iter (fun arg ->
      let expanded = expand_arg ctx arg in
      if expanded <> [] then
        List.iter (fun v -> match v with Num _ | Bln _ -> incr count | _ -> ()) expanded
      else
        (match eval_arg arg with Num _ | Bln _ -> incr count | _ -> ())
    ) args;
    Num (float_of_int !count)

  (* Conditional *)
  | "IF" ->
    (match args with
     | cond :: true_br :: rest ->
       let cv = eval_arg cond in
       let is_true = match result_to_num cv with Some n -> n <> 0.0 | None -> false in
       if is_error cv then cv
       else if is_true then eval_arg true_br
       else (match rest with fb :: _ -> eval_arg fb | [] -> Bln false)
     | _ -> Err (Types.Err_value, "IF requires 2-3 args"))
  | "IFERROR" ->
    (match args with
     | [val_arg; fallback] ->
       let v = eval_arg val_arg in
       if is_error v then eval_arg fallback else v
     | _ -> Err (Types.Err_value, "IFERROR requires 2 args"))
  | "IFNA" ->
    (match args with
     | [val_arg; fallback] ->
       let v = eval_arg val_arg in
       (match v with Err (Types.Err_na, _) -> eval_arg fallback | _ -> v)
     | _ -> Err (Types.Err_value, "IFNA requires 2 args"))

  (* Error generators *)
  | "NA" -> Err (Types.Err_na, "#N/A")
  | "ERROR" ->
    (match args with
     | [msg] ->
       let m = eval_arg msg in
       let s = match m with Txt s -> s | _ -> "error" in
       Err (Types.Err_value, s)
     | _ -> Err (Types.Err_value, "#VALUE!"))

  (* Logical *)
  | "AND" ->
    let result = ref true in
    let has_error = ref None in
    List.iter (fun arg ->
      if !has_error = None then begin
        let expanded = expand_arg ctx arg in
        let vals = if expanded <> [] then expanded else [eval_arg arg] in
        List.iter (fun v ->
          match result_to_num v with
          | Some n -> if n = 0.0 then result := false
          | None -> if is_error v then has_error := Some v
        ) vals
      end
    ) args;
    (match !has_error with Some e -> e | None -> Bln !result)
  | "OR" ->
    let result = ref false in
    let has_error = ref None in
    List.iter (fun arg ->
      if !has_error = None then begin
        let expanded = expand_arg ctx arg in
        let vals = if expanded <> [] then expanded else [eval_arg arg] in
        List.iter (fun v ->
          match result_to_num v with
          | Some n -> if n <> 0.0 then result := true
          | None -> if is_error v then has_error := Some v
        ) vals
      end
    ) args;
    (match !has_error with Some e -> e | None -> Bln !result)
  | "NOT" ->
    (match args with
     | [a] ->
       let v = eval_arg a in
       (match result_to_num v with
        | Some n -> Bln (n = 0.0)
        | None -> if is_error v then v else Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "NOT requires 1 arg"))

  (* Type predicates *)
  | "ISERROR" ->
    (match args with [a] -> Bln (is_error (eval_arg a)) | _ -> Err (Types.Err_value, ""))
  | "ISNA" ->
    (match args with
     | [a] -> (match eval_arg a with Err (Types.Err_na, _) -> Bln true | _ -> Bln false)
     | _ -> Err (Types.Err_value, ""))
  | "ISBLANK" ->
    (match args with [a] -> Bln (eval_arg a = Blk) | _ -> Err (Types.Err_value, ""))
  | "ISTEXT" ->
    (match args with [a] -> Bln (match eval_arg a with Txt _ -> true | _ -> false) | _ -> Err (Types.Err_value, ""))
  | "ISNUMBER" ->
    (match args with [a] -> Bln (match eval_arg a with Num _ -> true | _ -> false) | _ -> Err (Types.Err_value, ""))
  | "ISLOGICAL" ->
    (match args with [a] -> Bln (match eval_arg a with Bln _ -> true | _ -> false) | _ -> Err (Types.Err_value, ""))
  | "ERROR.TYPE" ->
    (match args with
     | [a] ->
       (match eval_arg a with
        | Err (k, _) ->
          let code = match k with
            | Types.Err_null -> 1 | Types.Div_zero -> 2 | Types.Err_value -> 3
            | Types.Err_ref -> 4 | Types.Err_name -> 5 | Types.Err_num -> 6
            | Types.Err_na -> 7 | _ -> 8
          in Num (float_of_int code)
        | _ -> Err (Types.Err_na, "#N/A"))
     | _ -> Err (Types.Err_value, ""))

  (* Math *)
  | "ABS" ->
    (match args with [a] -> (match eval_arg_num a with Some n -> Num (abs_float n) | None -> Err (Types.Err_value, "#VALUE!")) | _ -> Err (Types.Err_value, ""))
  | "INT" ->
    (match args with [a] -> (match eval_arg_num a with Some n -> Num (floor n) | None -> Err (Types.Err_value, "#VALUE!")) | _ -> Err (Types.Err_value, ""))
  | "ROUND" ->
    (match args with
     | [a; d] ->
       (match eval_arg_num a, eval_arg_num d with
        | Some n, Some digits ->
          let mult = 10.0 ** (floor digits) in
          Num (Float.round (n *. mult) /. mult)
        | _ -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, ""))
  | "SIGN" ->
    (match args with
     | [a] -> (match eval_arg_num a with
       | Some n -> Num (if n > 0.0 then 1.0 else if n < 0.0 then -1.0 else 0.0)
       | None -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, ""))
  | "SQRT" ->
    (match args with
     | [a] -> (match eval_arg_num a with
       | Some n -> if n < 0.0 then Err (Types.Err_num, "#NUM!") else Num (sqrt n)
       | None -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, ""))
  | "EXP" ->
    (match args with [a] -> (match eval_arg_num a with Some n -> Num (exp n) | None -> Err (Types.Err_value, "")) | _ -> Err (Types.Err_value, ""))
  | "LN" ->
    (match args with
     | [a] -> (match eval_arg_num a with
       | Some n -> if n <= 0.0 then Err (Types.Err_num, "#NUM!") else Num (log n)
       | None -> Err (Types.Err_value, ""))
     | _ -> Err (Types.Err_value, ""))
  | "LOG10" ->
    (match args with
     | [a] -> (match eval_arg_num a with
       | Some n -> if n <= 0.0 then Err (Types.Err_num, "#NUM!") else Num (log10 n)
       | None -> Err (Types.Err_value, ""))
     | _ -> Err (Types.Err_value, ""))
  | "SIN" -> (match args with [a] -> (match eval_arg_num a with Some n -> Num (sin n) | None -> Err (Types.Err_value, "")) | _ -> Err (Types.Err_value, ""))
  | "COS" -> (match args with [a] -> (match eval_arg_num a with Some n -> Num (cos n) | None -> Err (Types.Err_value, "")) | _ -> Err (Types.Err_value, ""))
  | "TAN" -> (match args with [a] -> (match eval_arg_num a with Some n -> Num (tan n) | None -> Err (Types.Err_value, "")) | _ -> Err (Types.Err_value, ""))
  | "ATN" -> (match args with [a] -> (match eval_arg_num a with Some n -> Num (atan n) | None -> Err (Types.Err_value, "")) | _ -> Err (Types.Err_value, ""))
  | "PI" -> Num Float.pi

  (* Financial *)
  | "NPV" ->
    (match args with
     | rate_s :: cashflows when List.length cashflows > 0 ->
       (match eval_arg_num rate_s with
        | Some rate ->
          let total = ref 0.0 in
          let period = ref 1 in
          List.iter (fun cf ->
            match eval_arg_num cf with
            | Some v ->
              total := !total +. v /. ((1.0 +. rate) ** float_of_int !period);
              incr period
            | None -> ()
          ) cashflows;
          Num !total
        | None -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "NPV requires rate and cashflows"))
  | "PV" ->
    (match args with
     | [rate_s; nper_s; pmt_s; fv_s; type_s] ->
       (match eval_arg_num rate_s, eval_arg_num nper_s, eval_arg_num pmt_s,
              eval_arg_num fv_s, eval_arg_num type_s with
        | Some rate, Some nper, Some pmt, Some fv, Some typ ->
          if rate = 0.0 then
            Num (-. pmt *. nper -. fv)
          else begin
            let pvif = (1.0 +. rate) ** nper in
            let pv_annuity = pmt *. (1.0 +. rate *. typ) *. (pvif -. 1.0) /. rate in
            Num (-.(pv_annuity +. fv) /. pvif)
          end
        | _ -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "PV requires 5 args"))
  | "FV" ->
    (match args with
     | [rate_s; nper_s; pmt_s; pv_s; type_s] ->
       (match eval_arg_num rate_s, eval_arg_num nper_s, eval_arg_num pmt_s,
              eval_arg_num pv_s, eval_arg_num type_s with
        | Some rate, Some nper, Some pmt, Some pv, Some typ ->
          if rate = 0.0 then
            Num (-. pmt *. nper -. pv)
          else begin
            let pvif = (1.0 +. rate) ** nper in
            let fv_annuity = pmt *. (1.0 +. rate *. typ) *. (pvif -. 1.0) /. rate in
            Num (-.pv *. pvif -. fv_annuity)
          end
        | _ -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "FV requires 5 args"))
  | "PMT" ->
    (match args with
     | [rate_s; nper_s; pv_s; fv_s; type_s] ->
       (match eval_arg_num rate_s, eval_arg_num nper_s, eval_arg_num pv_s,
              eval_arg_num fv_s, eval_arg_num type_s with
        | Some rate, Some nper, Some pv, Some fv, Some typ ->
          if rate = 0.0 then
            Num (-.(pv +. fv) /. nper)
          else begin
            let pvif = (1.0 +. rate) ** nper in
            Num (-.(pv *. pvif +. fv) *. rate /. ((1.0 +. rate *. typ) *. (pvif -. 1.0)))
          end
        | _ -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "PMT requires 5 args"))

  (* Lookup *)
  | "LOOKUP" ->
    (match args with
     | [needle_s; lookup_s; result_s] ->
       (match eval_arg_num needle_s with
        | Some needle ->
          let lookup_r = parse_range (String.trim lookup_s) in
          let result_r = parse_range (String.trim result_s) in
          (match lookup_r, result_r with
           | Some (lc1, lr1, lc2, lr2), Some (rc1, rr1, rc2, rr2) ->
             let lookup_vals = eval_range_values ctx lc1 lr1 lc2 lr2 in
             let result_vals = eval_range_values ctx rc1 rr1 rc2 rr2 in
             let best_idx = ref (-1) in
             let best_val = ref neg_infinity in
             List.iteri (fun i v ->
               match result_to_num v with
               | Some n when n <= needle && n > !best_val ->
                 best_val := n; best_idx := i
               | _ -> ()
             ) lookup_vals;
             if !best_idx >= 0 && !best_idx < List.length result_vals then
               List.nth result_vals !best_idx
             else Err (Types.Err_na, "#N/A")
           | _ -> Err (Types.Err_value, "#VALUE!"))
        | None -> Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "LOOKUP requires 3 args"))

  (* String *)
  | "CONCAT" ->
    let buf = Buffer.create 64 in
    List.iter (fun arg ->
      let v = eval_arg arg in
      match v with
      | Num n ->
        let s = Printf.sprintf "%.17g" n in
        (* Clean up trailing zeros for integer values *)
        let s = if Float.is_integer n && n >= -1e15 && n <= 1e15 then
          string_of_int (Float.to_int n)
        else s in
        Buffer.add_string buf s
      | Txt s -> Buffer.add_string buf s
      | Bln true -> Buffer.add_string buf "TRUE"
      | Bln false -> Buffer.add_string buf "FALSE"
      | Blk -> ()
      | Err _ -> ()
      | Arr _ -> ()
    ) args;
    Txt (Buffer.contents buf)
  | "LEN" ->
    (match args with
     | [a] ->
       let v = eval_arg a in
       let s = match v with
         | Txt s -> s | Num n -> Printf.sprintf "%.17g" n
         | Bln true -> "TRUE" | Bln false -> "FALSE" | _ -> ""
       in
       Num (float_of_int (String.length s))
     | _ -> Err (Types.Err_value, ""))

  (* Cell info *)
  | "ROW" ->
    (match args with
     | [] -> Num (float_of_int ctx.self_row)
     | [a] ->
       let a = String.trim a in
       (match Address.parse_a1 a 0 with
        | Some (_, r, consumed, _, _) when consumed = String.length a -> Num (float_of_int r)
        | _ ->
          match parse_range a with
          | Some (_, r, _, _) -> Num (float_of_int r)
          | None -> Num (float_of_int ctx.self_row))
     | _ -> Err (Types.Err_value, ""))
  | "COLUMN" ->
    (match args with
     | [] -> Num (float_of_int ctx.self_col)
     | [a] ->
       let a = String.trim a in
       (match Address.parse_a1 a 0 with
        | Some (c, _, consumed, _, _) when consumed = String.length a -> Num (float_of_int c)
        | _ ->
          match parse_range a with
          | Some (c, _, _, _) -> Num (float_of_int c)
          | None -> Num (float_of_int ctx.self_col))
     | _ -> Err (Types.Err_value, ""))

  (* LET *)
  | "LET" ->
    (match args with
     | [name_arg; val_arg; body_arg] ->
       let name = String.trim name_arg in
       let v = eval_arg val_arg in
       (match result_to_num v with
        | Some n ->
          let ctx' = { ctx with let_bindings = (name, n) :: ctx.let_bindings } in
          eval_expr ctx' (String.trim body_arg) (depth + 1)
        | None ->
          if is_error v then v
          else Err (Types.Err_value, "#VALUE!"))
     | _ -> Err (Types.Err_value, "LET requires 3 args"))

  (* INDIRECT *)
  | "INDIRECT" ->
    (match args with
     | [ref_arg] ->
       let v = eval_arg ref_arg in
       (match v with
        | Txt s ->
          (match Address.parse_a1 s 0 with
           | Some (c, r, consumed, _, _) when consumed = String.length s ->
             if Sheet.in_bounds ctx.sheet c r then begin
               let i = Sheet.idx ctx.sheet c r in
               value_to_result ctx.sheet.computed.(i).value
             end else Err (Types.Err_ref, "#REF!")
           | _ -> Err (Types.Err_ref, "#REF!"))
        | _ -> Err (Types.Err_ref, "#REF!"))
     | [ref_arg; mode_arg] ->
       let v = eval_arg ref_arg in
       let mode_v = eval_arg mode_arg in
       let is_a1 = match result_to_num mode_v with
         | Some n -> n <> 0.0 | None -> true
       in
       (match v with
        | Txt s ->
          if is_a1 then begin
            match Address.parse_a1 s 0 with
            | Some (c, r, consumed, _, _) when consumed = String.length s ->
              if Sheet.in_bounds ctx.sheet c r then
                value_to_result ctx.sheet.computed.(Sheet.idx ctx.sheet c r).value
              else Err (Types.Err_ref, "#REF!")
            | _ -> Err (Types.Err_ref, "#REF!")
          end else begin
            match Address.parse_r1c1 s 0 with
            | Some (c, r, consumed) when consumed = String.length s ->
              if Sheet.in_bounds ctx.sheet c r then
                value_to_result ctx.sheet.computed.(Sheet.idx ctx.sheet c r).value
              else Err (Types.Err_ref, "#REF!")
            | _ -> Err (Types.Err_ref, "#REF!")
          end
        | _ -> Err (Types.Err_ref, "#REF!"))
     | _ -> Err (Types.Err_value, "INDIRECT requires 1-2 args"))

  (* OFFSET *)
  | "OFFSET" ->
    (match args with
     | [base_arg; row_off_arg; col_off_arg] ->
       let base = String.trim base_arg in
       (match Address.parse_a1 base 0 with
        | Some (bc, br, _, _, _) ->
          (match eval_arg_num row_off_arg, eval_arg_num col_off_arg with
           | Some ro, Some co ->
             let nr = br + int_of_float ro in
             let nc = bc + int_of_float co in
             if Sheet.in_bounds ctx.sheet nc nr then
               value_to_result ctx.sheet.computed.(Sheet.idx ctx.sheet nc nr).value
             else Err (Types.Err_ref, "#REF!")
           | _ -> Err (Types.Err_value, "#VALUE!"))
        | None -> Err (Types.Err_ref, "#REF!"))
     | _ -> Err (Types.Err_value, "OFFSET requires 3 args"))

  (* Volatile *)
  | "NOW" -> Num (float_of_int ctx.committed_epoch)
  | "RAND" ->
    let seed = ctx.committed_epoch * 1000000 + ctx.self_row * 1000 + ctx.self_col + !(ctx.rand_counter) in
    ctx.rand_counter := !(ctx.rand_counter) + 1;
    Num (golden_hash seed)

  (* Dynamic array - SEQUENCE *)
  | "SEQUENCE" ->
    let rows = match args with a :: _ -> (match eval_arg_num a with Some n -> int_of_float n | None -> 1) | [] -> 1 in
    let cols = match args with _ :: a :: _ -> (match eval_arg_num a with Some n -> int_of_float n | None -> 1) | _ -> 1 in
    let start = match args with _ :: _ :: a :: _ -> (match eval_arg_num a with Some n -> n | None -> 1.0) | _ -> 1.0 in
    let step = match args with _ :: _ :: _ :: a :: _ -> (match eval_arg_num a with Some n -> n | None -> 1.0) | _ -> 1.0 in
    if rows <= 1 && cols <= 1 then Num start
    else begin
      let arr = Array.init rows (fun r ->
        Array.init cols (fun c ->
          Num (start +. float_of_int (r * cols + c) *. step)
        )
      ) in
      Arr arr
    end

  (* RANDARRAY *)
  | "RANDARRAY" ->
    let rows = match args with a :: _ -> (match eval_arg_num a with Some n -> int_of_float n | None -> 1) | [] -> 1 in
    let cols = match args with _ :: a :: _ -> (match eval_arg_num a with Some n -> int_of_float n | None -> 1) | _ -> 1 in
    let rmin = match args with _ :: _ :: a :: _ -> (match eval_arg_num a with Some n -> n | None -> 0.0) | _ -> 0.0 in
    let rmax = match args with _ :: _ :: _ :: a :: _ -> (match eval_arg_num a with Some n -> n | None -> 1.0) | _ -> 1.0 in
    let whole = match args with _ :: _ :: _ :: _ :: a :: _ -> (match eval_arg_num a with Some n -> n <> 0.0 | None -> false) | _ -> false in
    let arr = Array.init rows (fun r ->
      Array.init cols (fun c ->
        let seed = ctx.committed_epoch * 1000000 + (ctx.self_row + r) * 1000 + (ctx.self_col + c) + !(ctx.rand_counter) in
        ctx.rand_counter := !(ctx.rand_counter) + 1;
        let rv = golden_hash seed in
        let v = rmin +. rv *. (rmax -. rmin) in
        let v = if whole then floor (v +. 0.5) else v in
        let v = if whole then Float.min (Float.max v rmin) rmax else v in
        Num v
      )
    ) in
    if rows = 1 && cols = 1 then arr.(0).(0)
    else Arr arr

  (* MAP *)
  | "MAP" ->
    (match args with
     | [range_arg; lambda_arg] ->
       let range_arg = String.trim range_arg in
       let lambda_arg = String.trim lambda_arg in
       (* Parse range *)
       (match parse_range range_arg with
        | Some (c1, r1, c2, r2) ->
          let sc1 = min c1 c2 and sc2 = max c1 c2 in
          let sr1 = min r1 r2 and sr2 = max r1 r2 in
          let rows = sr2 - sr1 + 1 in
          let cols = sc2 - sc1 + 1 in
          (* Parse LAMBDA *)
          let lambda_upper = String.uppercase_ascii lambda_arg in
          if String.length lambda_upper > 7 && String.sub lambda_upper 0 7 = "LAMBDA(" then begin
            match find_close_paren lambda_arg 6 with
            | Some close ->
              let inner = String.sub lambda_arg 7 (close - 7) in
              let largs = split_args inner in
              (match largs with
               | [param; body] ->
                 let param = String.trim param in
                 let body = String.trim body in
                 let arr = Array.init rows (fun r ->
                   Array.init cols (fun c ->
                     let cr = sr1 + r and cc = sc1 + c in
                     if Sheet.in_bounds ctx.sheet cc cr then begin
                       let i = Sheet.idx ctx.sheet cc cr in
                       let cell_v = ctx.sheet.computed.(i).value in
                       match cell_v with
                       | Types.Number n ->
                         let ctx' = { ctx with let_bindings = (param, n) :: ctx.let_bindings } in
                         eval_expr ctx' body (depth + 1)
                       | _ -> Blk
                     end else Blk
                   )
                 ) in
                 if rows = 1 && cols = 1 then arr.(0).(0)
                 else Arr arr
               | _ -> Err (Types.Err_value, "MAP LAMBDA needs param and body"))
            | None -> Err (Types.Err_value, "unmatched paren")
          end else
            Err (Types.Err_value, "MAP second arg must be LAMBDA")
        | None -> Err (Types.Err_value, "MAP first arg must be range"))
     | _ -> Err (Types.Err_value, "MAP requires 2 args"))

  (* STREAM *)
  | "STREAM" ->
    (* Stream evaluation returns the current counter value.
       The actual streaming state is managed by the engine. *)
    let _period = match args with
      | [a] -> (match eval_arg_num a with Some n -> n | None -> 1.0)
      | _ -> 1.0
    in
    (* Return 0 - the engine will set up streaming state *)
    Num 0.0

  (* UDF lookup *)
  | _ ->
    (match List.find_opt (fun (n, _) -> String.uppercase_ascii n = fname) ctx.udfs with
     | Some (_, callback) ->
       let arg_vals = List.map (fun a ->
         result_to_value (eval_arg a)
       ) args in
       callback (Array.of_list arg_vals)
       |> value_to_result
     | None ->
       (* Try as named reference *)
       eval_name ctx fname depth)
