(* sheet.ml - Grid data structure for the spreadsheet *)

type cell_input =
  | Cell_empty
  | Cell_number of float
  | Cell_text of string
  | Cell_formula of string

type cell_computed = {
  mutable value : Types.value;
  mutable value_epoch : int;
  mutable text : string;
  mutable error_message : string;
  mutable spill_role : int;  (* 0=none, 1=anchor, 2=member *)
  mutable spill_anchor_col : int;
  mutable spill_anchor_row : int;
  mutable spill_range_start_col : int;
  mutable spill_range_start_row : int;
  mutable spill_range_end_col : int;
  mutable spill_range_end_row : int;
}

type stream_state = {
  mutable active : bool;
  mutable period : float;
  mutable elapsed : float;
  mutable counter : int;
}

type t = {
  max_columns : int;
  max_rows : int;
  cells : cell_input array;
  computed : cell_computed array;
  formats : Types.cell_format array;
  streams : stream_state array;
}

let cell_count s = s.max_columns * s.max_rows

let idx s col row = (row - 1) * s.max_columns + (col - 1)

let in_bounds s col row =
  col >= 1 && col <= s.max_columns && row >= 1 && row <= s.max_rows

let make_computed () = {
  value = Types.Blank; value_epoch = 0; text = "";
  error_message = "";
  spill_role = 0;
  spill_anchor_col = 0; spill_anchor_row = 0;
  spill_range_start_col = 0; spill_range_start_row = 0;
  spill_range_end_col = 0; spill_range_end_row = 0;
}

let make_stream () = {
  active = false; period = 0.0; elapsed = 0.0; counter = 0;
}

let create ~max_columns ~max_rows =
  let n = max_columns * max_rows in
  {
    max_columns; max_rows;
    cells = Array.make n Cell_empty;
    computed = Array.init n (fun _ -> make_computed ());
    formats = Array.make n Types.default_format;
    streams = Array.init n (fun _ -> make_stream ());
  }

let clear_cell sheet i =
  sheet.cells.(i) <- Cell_empty;
  let c = sheet.computed.(i) in
  c.value <- Types.Blank; c.value_epoch <- 0; c.text <- "";
  c.error_message <- "";
  c.spill_role <- 0;
  c.spill_anchor_col <- 0; c.spill_anchor_row <- 0;
  c.spill_range_start_col <- 0; c.spill_range_start_row <- 0;
  c.spill_range_end_col <- 0; c.spill_range_end_row <- 0;
  sheet.formats.(i) <- Types.default_format;
  let st = sheet.streams.(i) in
  st.active <- false; st.period <- 0.0; st.elapsed <- 0.0; st.counter <- 0

let clear_all sheet =
  let n = cell_count sheet in
  for i = 0 to n - 1 do
    clear_cell sheet i
  done

let get_input_type sheet i =
  match sheet.cells.(i) with
  | Cell_empty -> Types.Empty
  | Cell_number _ -> Types.Input_number
  | Cell_text _ -> Types.Input_text
  | Cell_formula _ -> Types.Input_formula

let get_input_text sheet i =
  match sheet.cells.(i) with
  | Cell_empty -> ""
  | Cell_number n -> Printf.sprintf "%.17g" n
  | Cell_text s -> s
  | Cell_formula s -> s
