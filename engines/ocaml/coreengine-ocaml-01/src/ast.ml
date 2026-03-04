(* ast.ml - Abstract syntax tree for formulas - not used for tree-based eval,
   but provides reserved-name list and helpers for the text-based evaluator *)

let builtin_functions = [
  "TRUE"; "FALSE"; "SUM"; "MIN"; "MAX"; "AVERAGE"; "COUNT";
  "IF"; "IFERROR"; "IFNA"; "NA"; "ERROR"; "AND"; "OR"; "NOT";
  "ISERROR"; "ISNA"; "ISBLANK"; "ISTEXT"; "ISNUMBER"; "ISLOGICAL";
  "ERROR.TYPE"; "ABS"; "INT"; "ROUND"; "SIGN"; "SQRT"; "EXP";
  "LN"; "LOG10"; "SIN"; "COS"; "TAN"; "ATN"; "PI";
  "NPV"; "PV"; "FV"; "PMT"; "LOOKUP"; "CONCAT"; "LEN";
  "SEQUENCE"; "RANDARRAY"; "LET"; "LAMBDA"; "MAP";
  "INDIRECT"; "OFFSET"; "ROW"; "COLUMN"; "NOW"; "RAND"; "STREAM";
]

let is_builtin name =
  List.exists (fun b -> String.uppercase_ascii name = b) builtin_functions

let is_valid_name s =
  let len = String.length s in
  if len = 0 then false
  else
    let c0 = s.[0] in
    let ok_start = (c0 >= 'A' && c0 <= 'Z') || (c0 >= 'a' && c0 <= 'z') || c0 = '_' in
    if not ok_start then false
    else begin
      let ok = ref true in
      for i = 1 to len - 1 do
        let c = s.[i] in
        if not ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
                (c >= '0' && c <= '9') || c = '_' || c = '.') then
          ok := false
      done;
      !ok
    end
