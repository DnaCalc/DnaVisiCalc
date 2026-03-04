#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../src/dvc_engine.h"

#define CHECK(expr) do { if (!(expr)) { fprintf(stderr, "check failed: %s (%s:%d)\n", #expr, __FILE__, __LINE__); return 1; } } while (0)
#define CHECK_OK(expr) CHECK((expr) == DVC_OK)

static int nearly_equal(double a, double b) {
  double diff = fabs(a - b);
  return diff <= 1e-9;
}

static int read_cell_number(DvcEngine *e, uint16_t col, uint16_t row, double *out) {
  DvcCellAddr addr = {col, row};
  DvcCellState st;
  if (dvc_cell_get_state(e, addr, &st) != DVC_OK) return 0;
  if (st.value.type != DVC_VALUE_NUMBER) return 0;
  *out = st.value.number;
  return 1;
}

static int read_cell_bool(DvcEngine *e, uint16_t col, uint16_t row, int *out) {
  DvcCellAddr addr = {col, row};
  DvcCellState st;
  if (dvc_cell_get_state(e, addr, &st) != DVC_OK) return 0;
  if (st.value.type != DVC_VALUE_BOOL) return 0;
  *out = st.value.bool_val ? 1 : 0;
  return 1;
}

static int read_cell_error_kind(DvcEngine *e, uint16_t col, uint16_t row, DvcCellErrorKind *out) {
  DvcCellAddr addr = {col, row};
  DvcCellState st;
  if (dvc_cell_get_state(e, addr, &st) != DVC_OK) return 0;
  if (st.value.type != DVC_VALUE_ERROR) return 0;
  *out = st.value.error_kind;
  return 1;
}

static int read_cell_text(DvcEngine *e, uint16_t col, uint16_t row, char *buf, uint32_t cap) {
  uint32_t out_len = 0;
  if (dvc_cell_get_text(e, (DvcCellAddr){col, row}, NULL, 0, &out_len) != DVC_OK) return 0;
  if (out_len + 1 > cap) return 0;
  if (dvc_cell_get_text(e, (DvcCellAddr){col, row}, buf, cap, &out_len) != DVC_OK) return 0;
  buf[out_len] = '\0';
  return 1;
}

static int read_cell_input_text(DvcEngine *e, uint16_t col, uint16_t row, char *buf, uint32_t cap) {
  DvcCellAddr addr = {col, row};
  uint32_t len = 0;
  if (dvc_cell_get_input_text(e, addr, NULL, 0, &len) != DVC_OK) return 0;
  if (len + 1 > cap) return 0;
  if (dvc_cell_get_input_text(e, addr, buf, cap, &len) != DVC_OK) return 0;
  buf[len] = '\0';
  return 1;
}

static int read_name_input_text(DvcEngine *e, const char *name, char *buf, uint32_t cap) {
  uint32_t len = 0;
  if (dvc_name_get_input_text(e, name, (uint32_t)strlen(name), NULL, 0, &len) != DVC_OK) return 0;
  if (len + 1 > cap) return 0;
  if (dvc_name_get_input_text(e, name, (uint32_t)strlen(name), buf, cap, &len) != DVC_OK) return 0;
  buf[len] = '\0';
  return 1;
}

static int test_slice_a_function_semantics(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 4.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "LET(x,A1+1,x*2)", (uint32_t)strlen("LET(x,A1+1,x*2)")));
  double v = 0.0;
  CHECK(read_cell_number(e, 2, 1, &v));
  CHECK(nearly_equal(v, 10.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 2}, "LAMBDA(x,x+3)(A1)", (uint32_t)strlen("LAMBDA(x,x+3)(A1)")));
  CHECK(read_cell_number(e, 2, 2, &v));
  CHECK(nearly_equal(v, 7.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 1}, "SEQUENCE(2,2,1,1)", (uint32_t)strlen("SEQUENCE(2,2,1,1)")));
  CHECK(read_cell_number(e, 3, 1, &v) && nearly_equal(v, 1.0));
  CHECK(read_cell_number(e, 4, 1, &v) && nearly_equal(v, 2.0));
  CHECK(read_cell_number(e, 3, 2, &v) && nearly_equal(v, 3.0));
  CHECK(read_cell_number(e, 4, 2, &v) && nearly_equal(v, 4.0));
  DvcSpillRole role = DVC_SPILL_NONE;
  CHECK_OK(dvc_cell_spill_role(e, (DvcCellAddr){3, 1}, &role));
  CHECK(role == DVC_SPILL_ANCHOR);
  CHECK_OK(dvc_cell_spill_role(e, (DvcCellAddr){4, 2}, &role));
  CHECK(role == DVC_SPILL_MEMBER);

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 1}, "MAP(C1:D2,LAMBDA(v,v*10))", (uint32_t)strlen("MAP(C1:D2,LAMBDA(v,v*10))")));
  CHECK(read_cell_number(e, 6, 1, &v) && nearly_equal(v, 10.0));
  CHECK(read_cell_number(e, 7, 2, &v) && nearly_equal(v, 40.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 1}, "INDIRECT(\"A1\")", (uint32_t)strlen("INDIRECT(\"A1\")")));
  CHECK(read_cell_number(e, 8, 1, &v) && nearly_equal(v, 4.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 2}, "INDIRECT(\"R1C1\",FALSE)", (uint32_t)strlen("INDIRECT(\"R1C1\",FALSE)")));
  CHECK(read_cell_number(e, 8, 2, &v) && nearly_equal(v, 4.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 3}, "OFFSET(A1,1,1)", (uint32_t)strlen("OFFSET(A1,1,1)")));
  CHECK(read_cell_number(e, 8, 3, &v) && nearly_equal(v, 7.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){10, 1}, "RANDARRAY(1,1)", (uint32_t)strlen("RANDARRAY(1,1)")));
  CHECK(read_cell_number(e, 10, 1, &v));
  CHECK(v >= 0.0 && v <= 1.0);
  double prev = v;
  CHECK_OK(dvc_invalidate_volatile(e));
  CHECK(read_cell_number(e, 10, 1, &v));
  CHECK(!nearly_equal(prev, v));
  CHECK(v >= 0.0 && v <= 1.0);

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){10, 2}, "RANDARRAY(1,1,5,9,1)", (uint32_t)strlen("RANDARRAY(1,1,5,9,1)")));
  CHECK(read_cell_number(e, 10, 2, &v));
  CHECK(v >= 5.0 && v <= 9.0);
  CHECK(nearly_equal(v, floor(v)));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 2}, 2.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 3}, 8.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 10}, "SUM(A1:A3)", (uint32_t)strlen("SUM(A1:A3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 10}, "MIN(A1:A3)", (uint32_t)strlen("MIN(A1:A3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 10}, "MAX(A1:A3)", (uint32_t)strlen("MAX(A1:A3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 10}, "AVERAGE(A1:A3)", (uint32_t)strlen("AVERAGE(A1:A3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 10}, "COUNT(A1:A3)", (uint32_t)strlen("COUNT(A1:A3)")));
  CHECK(read_cell_number(e, 2, 10, &v) && nearly_equal(v, 14.0));
  CHECK(read_cell_number(e, 3, 10, &v) && nearly_equal(v, 2.0));
  CHECK(read_cell_number(e, 4, 10, &v) && nearly_equal(v, 8.0));
  CHECK(read_cell_number(e, 5, 10, &v) && nearly_equal(v, 14.0 / 3.0));
  CHECK(read_cell_number(e, 6, 10, &v) && nearly_equal(v, 3.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 11}, "IF(A1>3,10,20)", (uint32_t)strlen("IF(A1>3,10,20)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 11}, "IFERROR(ERROR(\"x\"),7)", (uint32_t)strlen("IFERROR(ERROR(\"x\"),7)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 11}, "IFNA(NA(),9)", (uint32_t)strlen("IFNA(NA(),9)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 11}, "NA()", (uint32_t)strlen("NA()")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 11}, "ERROR(\"x\")", (uint32_t)strlen("ERROR(\"x\")")));
  CHECK(read_cell_number(e, 2, 11, &v) && nearly_equal(v, 10.0));
  CHECK(read_cell_number(e, 3, 11, &v) && nearly_equal(v, 7.0));
  CHECK(read_cell_number(e, 4, 11, &v) && nearly_equal(v, 9.0));
  DvcCellErrorKind ek = DVC_CELL_ERR_NULL;
  CHECK(read_cell_error_kind(e, 5, 11, &ek) && ek == DVC_CELL_ERR_NA);
  CHECK(read_cell_error_kind(e, 6, 11, &ek) && ek == DVC_CELL_ERR_VALUE);

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 12}, "AND(A1>0,A2>0)", (uint32_t)strlen("AND(A1>0,A2>0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 12}, "OR(A1<0,A2>0)", (uint32_t)strlen("OR(A1<0,A2>0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 12}, "NOT(A1>0)", (uint32_t)strlen("NOT(A1>0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 12}, "ISERROR(E11)", (uint32_t)strlen("ISERROR(E11)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 12}, "ISNA(E11)", (uint32_t)strlen("ISNA(E11)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){7, 12}, "ISBLANK(Z100)", (uint32_t)strlen("ISBLANK(Z100)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 12}, "ISTEXT(\"abc\")", (uint32_t)strlen("ISTEXT(\"abc\")")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){9, 12}, "ISNUMBER(A1)", (uint32_t)strlen("ISNUMBER(A1)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){10, 12}, "ISLOGICAL(B12)", (uint32_t)strlen("ISLOGICAL(B12)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){11, 12}, "ERROR.TYPE(E11)", (uint32_t)strlen("ERROR.TYPE(E11)")));
  int bv = 0;
  CHECK(read_cell_bool(e, 2, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 3, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 4, 12, &bv) && bv == 0);
  CHECK(read_cell_bool(e, 5, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 6, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 7, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 8, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 9, 12, &bv) && bv == 1);
  CHECK(read_cell_bool(e, 10, 12, &bv) && bv == 1);
  CHECK(read_cell_number(e, 11, 12, &v) && nearly_equal(v, 7.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 13}, "ABS(-3)", (uint32_t)strlen("ABS(-3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 13}, "INT(3.9)", (uint32_t)strlen("INT(3.9)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 13}, "ROUND(3.14159,2)", (uint32_t)strlen("ROUND(3.14159,2)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 13}, "SIGN(-5)", (uint32_t)strlen("SIGN(-5)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 13}, "SQRT(9)", (uint32_t)strlen("SQRT(9)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){7, 13}, "EXP(1)", (uint32_t)strlen("EXP(1)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 13}, "LN(2.718281828459045)", (uint32_t)strlen("LN(2.718281828459045)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){9, 13}, "LOG10(100)", (uint32_t)strlen("LOG10(100)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){10, 13}, "SIN(0)", (uint32_t)strlen("SIN(0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){11, 13}, "COS(0)", (uint32_t)strlen("COS(0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){12, 13}, "TAN(0)", (uint32_t)strlen("TAN(0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){13, 13}, "ATN(1)", (uint32_t)strlen("ATN(1)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){14, 13}, "PI()", (uint32_t)strlen("PI()")));
  CHECK(read_cell_number(e, 2, 13, &v) && nearly_equal(v, 3.0));
  CHECK(read_cell_number(e, 3, 13, &v) && nearly_equal(v, 3.0));
  CHECK(read_cell_number(e, 4, 13, &v) && nearly_equal(v, 3.14));
  CHECK(read_cell_number(e, 5, 13, &v) && nearly_equal(v, -1.0));
  CHECK(read_cell_number(e, 6, 13, &v) && nearly_equal(v, 3.0));
  CHECK(read_cell_number(e, 7, 13, &v) && nearly_equal(v, 2.718281828459045));
  CHECK(read_cell_number(e, 8, 13, &v) && nearly_equal(v, 1.0));
  CHECK(read_cell_number(e, 9, 13, &v) && nearly_equal(v, 2.0));
  CHECK(read_cell_number(e, 10, 13, &v) && nearly_equal(v, 0.0));
  CHECK(read_cell_number(e, 11, 13, &v) && nearly_equal(v, 1.0));
  CHECK(read_cell_number(e, 12, 13, &v) && nearly_equal(v, 0.0));
  CHECK(read_cell_number(e, 13, 13, &v) && nearly_equal(v, 0.7853981633974483));
  CHECK(read_cell_number(e, 14, 13, &v) && nearly_equal(v, 3.141592653589793));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 20}, 1.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 21}, 2.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 22}, 3.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 20}, 10.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 21}, 20.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 22}, 30.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 14}, "NPV(0.1,100,110)", (uint32_t)strlen("NPV(0.1,100,110)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 14}, "PV(0,10,-5,0,0)", (uint32_t)strlen("PV(0,10,-5,0,0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 14}, "FV(0,10,-5,0,0)", (uint32_t)strlen("FV(0,10,-5,0,0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 14}, "PMT(0,10,50,0,0)", (uint32_t)strlen("PMT(0,10,50,0,0)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 14}, "LOOKUP(2.5,A20:A22,B20:B22)", (uint32_t)strlen("LOOKUP(2.5,A20:A22,B20:B22)")));
  CHECK(read_cell_number(e, 2, 14, &v) && nearly_equal(v, 181.8181818181818));
  CHECK(read_cell_number(e, 3, 14, &v) && nearly_equal(v, 50.0));
  CHECK(read_cell_number(e, 4, 14, &v) && nearly_equal(v, 50.0));
  CHECK(read_cell_number(e, 5, 14, &v) && nearly_equal(v, -5.0));
  CHECK(read_cell_number(e, 6, 14, &v) && nearly_equal(v, 20.0));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 15}, "CONCAT(\"A\",\"B\",A1)", (uint32_t)strlen("CONCAT(\"A\",\"B\",A1)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 15}, "LEN(\"Hello\")", (uint32_t)strlen("LEN(\"Hello\")")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 15}, "ROW(A3)", (uint32_t)strlen("ROW(A3)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 15}, "COLUMN(B2)", (uint32_t)strlen("COLUMN(B2)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 15}, "ROW()", (uint32_t)strlen("ROW()")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){7, 15}, "COLUMN()", (uint32_t)strlen("COLUMN()")));
  char tbuf[64];
  CHECK(read_cell_text(e, 2, 15, tbuf, sizeof(tbuf)));
  CHECK(strcmp(tbuf, "AB4") == 0);
  CHECK(read_cell_number(e, 3, 15, &v) && nearly_equal(v, 5.0));
  CHECK(read_cell_number(e, 4, 15, &v) && nearly_equal(v, 3.0));
  CHECK(read_cell_number(e, 5, 15, &v) && nearly_equal(v, 2.0));
  CHECK(read_cell_number(e, 6, 15, &v) && nearly_equal(v, 15.0));
  CHECK(read_cell_number(e, 7, 15, &v) && nearly_equal(v, 7.0));

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_slice_b_rewrite_and_reject(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 1}, "$A1+A$1+$A$1+A1", (uint32_t)strlen("$A1+A$1+$A$1+A1")));
  CHECK_OK(dvc_name_set_formula(e, "NREF", 4, "A1:B2", (uint32_t)strlen("A1:B2")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){7, 5}, "SEQUENCE(2,1,1,1)", (uint32_t)strlen("SEQUENCE(2,1,1,1)")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){8, 5}, "G5#", (uint32_t)strlen("G5#")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){9, 4}, "SUM($A1,B$2,C3:D4,G5#)", (uint32_t)strlen("SUM($A1,B$2,C3:D4,G5#)")));

  CHECK_OK(dvc_insert_row(e, 1));

  char text[128];
  CHECK(read_cell_input_text(e, 5, 2, text, sizeof(text)));
  CHECK(strcmp(text, "$A2+A$1+$A$1+A2") == 0);
  CHECK(read_name_input_text(e, "NREF", text, sizeof(text)));
  CHECK(strcmp(text, "A2:B3") == 0);
  CHECK(read_cell_input_text(e, 8, 6, text, sizeof(text)));
  CHECK(strcmp(text, "G6#") == 0);
  CHECK(read_cell_input_text(e, 9, 5, text, sizeof(text)));
  CHECK(strcmp(text, "SUM($A2,B$2,C4:D5,G6#)") == 0);

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 4}, "A1+1", (uint32_t)strlen("A1+1")));
  CHECK_OK(dvc_delete_row(e, 1));
  CHECK(read_cell_input_text(e, 4, 3, text, sizeof(text)));
  CHECK(strcmp(text, "#REF!+1") == 0);

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "SEQUENCE(2,2,1,1)", (uint32_t)strlen("SEQUENCE(2,2,1,1)")));
  uint64_t before_epoch = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &before_epoch));
  DvcStatus s = dvc_delete_row(e, 1);
  CHECK(s == DVC_REJECT_STRUCTURAL_CONSTRAINT);
  uint64_t after_epoch = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &after_epoch));
  CHECK(before_epoch == after_epoch);
  DvcRejectKind reject_kind = DVC_REJECT_KIND_NONE;
  CHECK_OK(dvc_last_reject_kind(e, &reject_kind));
  CHECK(reject_kind == DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT);
  DvcLastRejectContext reject_ctx;
  CHECK_OK(dvc_last_reject_context(e, &reject_ctx));
  CHECK(reject_ctx.reject_kind == DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT);
  CHECK(reject_ctx.op_kind == DVC_STRUCT_OP_DELETE_ROW);
  CHECK(reject_ctx.op_index == 1);
  CHECK(reject_ctx.has_cell == 1);
  CHECK(reject_ctx.has_range == 1);
  CHECK(reject_ctx.range.start.col == 1 && reject_ctx.range.start.row == 1);
  CHECK(reject_ctx.range.end.col == 2 && reject_ctx.range.end.row == 2);

  CHECK_OK(dvc_engine_destroy(e));

  DvcEngine *e2 = NULL;
  CHECK_OK(dvc_engine_create(&e2));
  CHECK_OK(dvc_cell_set_formula(e2, (DvcCellAddr){3, 3}, "SUM($A1,B$2,C3:D4)", (uint32_t)strlen("SUM($A1,B$2,C3:D4)")));
  CHECK_OK(dvc_name_set_formula(e2, "NCOL", 4, "B2:C3", (uint32_t)strlen("B2:C3")));
  CHECK_OK(dvc_insert_col(e2, 2));
  CHECK(read_cell_input_text(e2, 4, 3, text, sizeof(text)));
  CHECK(strcmp(text, "SUM($A1,C$2,D3:E4)") == 0);
  CHECK(read_name_input_text(e2, "NCOL", text, sizeof(text)));
  CHECK(strcmp(text, "C2:D3") == 0);

  CHECK_OK(dvc_cell_set_formula(e2, (DvcCellAddr){3, 4}, "B1+1", (uint32_t)strlen("B1+1")));
  CHECK_OK(dvc_delete_col(e2, 2));
  CHECK(read_cell_input_text(e2, 2, 4, text, sizeof(text)));
  CHECK(strcmp(text, "#REF!+1") == 0);
  CHECK_OK(dvc_engine_destroy(e2));
  return 0;
}

static int test_slice_c_cycle_diagnostic(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));
  CHECK_OK(dvc_change_tracking_enable(e));

  DvcIterationConfig cfg;
  CHECK_OK(dvc_engine_get_iteration_config(e, &cfg));
  cfg.enabled = 0;
  CHECK_OK(dvc_engine_set_iteration_config(e, &cfg));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "B1", (uint32_t)strlen("B1")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1", (uint32_t)strlen("A1")));
  CHECK_OK(dvc_recalculate(e));

  DvcCellState st;
  CHECK_OK(dvc_cell_get_state(e, (DvcCellAddr){1, 1}, &st));
  CHECK(st.value.type == DVC_VALUE_NUMBER || st.value.type == DVC_VALUE_BOOL);

  DvcChangeIterator *it = NULL;
  CHECK_OK(dvc_change_iterate(e, &it));
  CHECK(it != NULL);
  int found_cycle_diag = 0;
  int32_t done = 0;
  while (!done) {
    DvcChangeType t = DVC_CHANGE_CELL_VALUE;
    uint64_t epoch = 0;
    CHECK_OK(dvc_change_iterator_next(it, &t, &epoch, &done));
    if (done) break;
    if (t == DVC_CHANGE_DIAGNOSTIC) {
      DvcDiagnosticCode code = -1;
      uint32_t msg_len = 0;
      char msg[128];
      CHECK_OK(dvc_change_get_diagnostic(it, &code, NULL, 0, &msg_len));
      CHECK(msg_len < sizeof(msg));
      CHECK_OK(dvc_change_get_diagnostic(it, &code, msg, sizeof(msg), &msg_len));
      msg[msg_len] = '\0';
      if (code == DVC_DIAG_CIRCULAR_REFERENCE_DETECTED) {
        found_cycle_diag = 1;
      }
    }
  }
  CHECK(found_cycle_diag);

  CHECK_OK(dvc_change_iterator_destroy(it));
  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_slice_d_cycle_mode_depth(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));
  CHECK_OK(dvc_change_tracking_enable(e));
  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));

  DvcIterationConfig cfg;
  CHECK_OK(dvc_engine_get_iteration_config(e, &cfg));
  cfg.enabled = 0;
  CHECK_OK(dvc_engine_set_iteration_config(e, &cfg));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 0.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 1}, 0.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){3, 1}, 0.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "B1+1", (uint32_t)strlen("B1+1")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "C1+1", (uint32_t)strlen("C1+1")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 1}, "A1+1", (uint32_t)strlen("A1+1")));
  CHECK_OK(dvc_recalculate(e));

  double v = 0.0;
  CHECK(read_cell_number(e, 1, 1, &v) && nearly_equal(v, 1.0));
  CHECK(read_cell_number(e, 2, 1, &v) && nearly_equal(v, 1.0));
  CHECK(read_cell_number(e, 3, 1, &v) && nearly_equal(v, 1.0));

  DvcChangeIterator *it = NULL;
  CHECK_OK(dvc_change_iterate(e, &it));
  int found_cycle_diag = 0;
  int32_t done = 0;
  while (!done) {
    DvcChangeType t = DVC_CHANGE_CELL_VALUE;
    uint64_t epoch = 0;
    CHECK_OK(dvc_change_iterator_next(it, &t, &epoch, &done));
    if (done) break;
    if (t == DVC_CHANGE_DIAGNOSTIC) {
      DvcDiagnosticCode code = -1;
      uint32_t msg_len = 0;
      CHECK_OK(dvc_change_get_diagnostic(it, &code, NULL, 0, &msg_len));
      if (code == DVC_DIAG_CIRCULAR_REFERENCE_DETECTED) found_cycle_diag = 1;
    }
  }
  CHECK(found_cycle_diag == 1);
  CHECK_OK(dvc_change_iterator_destroy(it));

  CHECK_OK(dvc_engine_get_iteration_config(e, &cfg));
  cfg.enabled = 1;
  cfg.max_iterations = 64;
  cfg.convergence_tolerance = 1e-9;
  CHECK_OK(dvc_engine_set_iteration_config(e, &cfg));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 0.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 1}, 0.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "B1/2+5", (uint32_t)strlen("B1/2+5")));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1/2+5", (uint32_t)strlen("A1/2+5")));
  CHECK_OK(dvc_recalculate(e));

  CHECK(read_cell_number(e, 1, 1, &v) && fabs(v - 10.0) < 1e-4);
  CHECK(read_cell_number(e, 2, 1, &v) && fabs(v - 10.0) < 1e-4);

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

int main(void) {
  if (test_slice_a_function_semantics() != 0) return 1;
  if (test_slice_b_rewrite_and_reject() != 0) return 1;
  if (test_slice_c_cycle_diagnostic() != 0) return 1;
  if (test_slice_d_cycle_mode_depth() != 0) return 1;
  printf("api_closure: ok\n");
  return 0;
}
