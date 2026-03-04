(* types.ml - Core types for DnaVisiCalc OCaml engine *)

module Status = struct
  let ok = 0
  let reject_structural = 1
  let reject_policy = 2
  let err_null_pointer = -1
  let err_out_of_bounds = -2
  let err_invalid_address = -3
  let err_parse = -4
  let err_dependency = -5
  let err_invalid_name = -6
  let err_out_of_memory = -7
  let err_invalid_argument = -8
end

type error_kind =
  | Div_zero | Err_value | Err_name | Unknown_name | Err_ref
  | Err_spill | Err_cycle | Err_na | Err_null | Err_num

let error_kind_to_int = function
  | Div_zero -> 0 | Err_value -> 1 | Err_name -> 2 | Unknown_name -> 3
  | Err_ref -> 4 | Err_spill -> 5 | Err_cycle -> 6 | Err_na -> 7
  | Err_null -> 8 | Err_num -> 9

let error_kind_of_int = function
  | 0 -> Div_zero | 1 -> Err_value | 2 -> Err_name | 3 -> Unknown_name
  | 4 -> Err_ref | 5 -> Err_spill | 6 -> Err_cycle | 7 -> Err_na
  | 8 -> Err_null | _ -> Err_num

type value =
  | Number of float
  | Text of string
  | Bool of bool
  | Blank
  | Error of error_kind * string

let value_type_int = function
  | Number _ -> 0 | Text _ -> 1 | Bool _ -> 2 | Blank -> 3 | Error _ -> 4

let value_number = function Number n -> n | Bool true -> 1.0 | Bool false -> 0.0 | _ -> 0.0
let value_bool_val = function Bool b -> if b then 1 else 0 | _ -> 0
let value_error_kind = function Error (k, _) -> error_kind_to_int k | _ -> 0

module Recalc_mode = struct
  let automatic = 0
  let manual = 1
end

type input_type = Empty | Input_number | Input_text | Input_formula

let input_type_to_int = function
  | Empty -> 0 | Input_number -> 1 | Input_text -> 2 | Input_formula -> 3

module Spill_role = struct
  let none = 0
  let anchor = 1
  let member_ = 2
end

type cell_state = {
  value : value;
  value_epoch : int;
  stale : bool;
}

type cell_format = {
  has_decimals : bool;
  decimals : int;
  bold : bool;
  italic : bool;
  fg : int;
  bg : int;
}

let default_format = {
  has_decimals = false; decimals = 0;
  bold = false; italic = false;
  fg = -1; bg = -1;
}

type control_kind = Slider | Checkbox | Button

let control_kind_to_int = function Slider -> 0 | Checkbox -> 1 | Button -> 2
let control_kind_of_int = function 0 -> Slider | 1 -> Checkbox | _ -> Button

type control_def = {
  kind : control_kind;
  min : float;
  max : float;
  step : float;
}

type chart_def = {
  source_start_col : int;
  source_start_row : int;
  source_end_col : int;
  source_end_row : int;
}

type volatility = Standard | Volatile | Externally_invalidated

let volatility_of_int = function 0 -> Standard | 1 -> Volatile | _ -> Externally_invalidated

type change_type =
  | Change_cell_value
  | Change_name_value
  | Change_chart_output
  | Change_spill_region
  | Change_cell_format
  | Change_diagnostic

let change_type_to_int = function
  | Change_cell_value -> 0 | Change_name_value -> 1 | Change_chart_output -> 2
  | Change_spill_region -> 3 | Change_cell_format -> 4 | Change_diagnostic -> 5

type diagnostic_code = Circular_reference_detected

type iteration_config = {
  enabled : bool;
  max_iterations : int;
  convergence_tolerance : float;
}

type structural_op = None_op | Insert_row | Delete_row | Insert_col | Delete_col

let structural_op_to_int = function
  | None_op -> 0 | Insert_row -> 1 | Delete_row -> 2 | Insert_col -> 3 | Delete_col -> 4

type reject_context = {
  reject_kind : int;
  op_kind : structural_op;
  op_index : int;
  has_cell : bool;
  cell_col : int;
  cell_row : int;
  has_range : bool;
  range_start_col : int;
  range_start_row : int;
  range_end_col : int;
  range_end_row : int;
}

let empty_reject_context = {
  reject_kind = 0; op_kind = None_op; op_index = 0;
  has_cell = false; cell_col = 0; cell_row = 0;
  has_range = false;
  range_start_col = 0; range_start_row = 0;
  range_end_col = 0; range_end_row = 0;
}
