#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

#include "../src/dvc_engine.h"

#define LOAD_FN(handle, name, type) \
  type name = (type)GetProcAddress(handle, #name); \
  if (!(name)) { fprintf(stderr, "missing export: %s\n", #name); return 2; }

typedef DvcStatus (*fn_create)(DvcEngine **);
typedef DvcStatus (*fn_destroy)(DvcEngine *);
typedef DvcStatus (*fn_set_number)(DvcEngine *, DvcCellAddr, double);
typedef DvcStatus (*fn_set_formula)(DvcEngine *, DvcCellAddr, const char *, uint32_t);
typedef DvcStatus (*fn_get_state)(const DvcEngine *, DvcCellAddr, DvcCellState *);
typedef DvcStatus (*fn_spill_role)(const DvcEngine *, DvcCellAddr, DvcSpillRole *);
typedef DvcStatus (*fn_tick_streams)(DvcEngine *, double, int32_t *);
typedef DvcStatus (*fn_insert_row)(DvcEngine *, uint16_t);
typedef DvcStatus (*fn_control_define)(DvcEngine *, const char *, uint32_t, const DvcControlDef *);
typedef DvcStatus (*fn_control_set_value)(DvcEngine *, const char *, uint32_t, double);
typedef DvcStatus (*fn_control_get_value)(const DvcEngine *, const char *, uint32_t, double *, int32_t *);
typedef DvcStatus (*fn_chart_define)(DvcEngine *, const char *, uint32_t, const DvcChartDef *);
typedef DvcStatus (*fn_chart_get_output)(const DvcEngine *, const char *, uint32_t, DvcChartOutput **, int32_t *);
typedef DvcStatus (*fn_chart_series_count)(const DvcChartOutput *, uint32_t *);
typedef DvcStatus (*fn_udf_register)(DvcEngine *, const char *, uint32_t, DvcUdfCallback, void *, DvcVolatility);
typedef DvcStatus (*fn_udf_unregister)(DvcEngine *, const char *, uint32_t, int32_t *);
typedef DvcStatus (*fn_invalidate_udf)(DvcEngine *, const char *, uint32_t);
typedef DvcStatus (*fn_change_enable)(DvcEngine *);
typedef DvcStatus (*fn_change_iterate)(DvcEngine *, DvcChangeIterator **);
typedef DvcStatus (*fn_change_next)(DvcChangeIterator *, DvcChangeType *, uint64_t *, int32_t *);
typedef DvcStatus (*fn_change_destroy)(DvcChangeIterator *);
typedef DvcStatus (*fn_parse_ref)(const DvcEngine *, const char *, uint32_t, DvcCellAddr *);

typedef uint32_t (*fn_api_version)(void);

static DvcStatus udf_passthrough(
    void *user_data,
    const DvcCellValue *args,
    uint32_t arg_count,
    DvcCellValue *out) {
  (void)user_data;
  if (!out) return DVC_ERR_NULL_POINTER;
  if (arg_count > 0 && args) {
    *out = args[0];
  } else {
    out->type = DVC_VALUE_NUMBER;
    out->number = 0.0;
    out->bool_val = 0;
    out->error_kind = DVC_CELL_ERR_NULL;
  }
  return DVC_OK;
}

int main(void) {
  HMODULE dll = LoadLibraryA("dvc_coreengine_c01.dll");
  if (!dll) {
    fprintf(stderr, "failed to load dll\n");
    return 1;
  }

  LOAD_FN(dll, dvc_engine_create, fn_create);
  LOAD_FN(dll, dvc_engine_destroy, fn_destroy);
  LOAD_FN(dll, dvc_cell_set_number, fn_set_number);
  LOAD_FN(dll, dvc_cell_set_formula, fn_set_formula);
  LOAD_FN(dll, dvc_cell_get_state, fn_get_state);
  LOAD_FN(dll, dvc_cell_spill_role, fn_spill_role);
  LOAD_FN(dll, dvc_tick_streams, fn_tick_streams);
  LOAD_FN(dll, dvc_insert_row, fn_insert_row);
  LOAD_FN(dll, dvc_control_define, fn_control_define);
  LOAD_FN(dll, dvc_control_set_value, fn_control_set_value);
  LOAD_FN(dll, dvc_control_get_value, fn_control_get_value);
  LOAD_FN(dll, dvc_chart_define, fn_chart_define);
  LOAD_FN(dll, dvc_chart_get_output, fn_chart_get_output);
  LOAD_FN(dll, dvc_chart_output_series_count, fn_chart_series_count);
  LOAD_FN(dll, dvc_udf_register, fn_udf_register);
  LOAD_FN(dll, dvc_udf_unregister, fn_udf_unregister);
  LOAD_FN(dll, dvc_invalidate_udf, fn_invalidate_udf);
  LOAD_FN(dll, dvc_change_tracking_enable, fn_change_enable);
  LOAD_FN(dll, dvc_change_iterate, fn_change_iterate);
  LOAD_FN(dll, dvc_change_iterator_next, fn_change_next);
  LOAD_FN(dll, dvc_change_iterator_destroy, fn_change_destroy);
  LOAD_FN(dll, dvc_parse_cell_ref, fn_parse_ref);
  LOAD_FN(dll, dvc_api_version, fn_api_version);

  if (dvc_api_version() == 0) {
    fprintf(stderr, "api version invalid\n");
    return 3;
  }

  DvcEngine *e = NULL;
  if (dvc_engine_create(&e) != DVC_OK || !e) {
    fprintf(stderr, "create failed\n");
    return 4;
  }

  DvcCellAddr a1 = {1,1};
  DvcCellAddr b1 = {2,1};
  DvcCellAddr c1 = {3,1};
  DvcCellAddr c2 = {3,2};
  DvcCellState st;

  if (dvc_cell_set_number(e, a1, 10.0) != DVC_OK) return 5;
  if (dvc_cell_set_formula(e, b1, "A1+5", 4) != DVC_OK) return 6;
  if (dvc_cell_get_state(e, b1, &st) != DVC_OK) return 7;
  if (st.value.type != DVC_VALUE_NUMBER || st.value.number < 14.999 || st.value.number > 15.001) {
    fprintf(stderr, "formula eval mismatch: %.4f\n", st.value.number);
    return 8;
  }

  if (dvc_cell_set_formula(e, c1, "SEQUENCE(2,2,1,1)", 17) != DVC_OK) return 9;
  DvcSpillRole role = DVC_SPILL_NONE;
  if (dvc_cell_spill_role(e, c1, &role) != DVC_OK || role != DVC_SPILL_ANCHOR) return 10;
  if (dvc_cell_spill_role(e, c2, &role) != DVC_OK || role != DVC_SPILL_MEMBER) return 11;

  DvcCellAddr d1 = {4,1};
  if (dvc_cell_set_formula(e, d1, "STREAM(1)", 9) != DVC_OK) return 12;
  int32_t any_adv = 0;
  if (dvc_tick_streams(e, 1.2, &any_adv) != DVC_OK || any_adv != 1) return 13;

  DvcControlDef cd;
  cd.kind = DVC_CONTROL_SLIDER;
  cd.min = 0.0;
  cd.max = 10.0;
  cd.step = 1.0;
  if (dvc_control_define(e, "RATE", 4, &cd) != DVC_OK) return 14;
  if (dvc_control_set_value(e, "RATE", 4, 7.0) != DVC_OK) return 15;
  double cv = 0.0;
  int32_t found = 0;
  if (dvc_control_get_value(e, "RATE", 4, &cv, &found) != DVC_OK || !found || cv < 6.999 || cv > 7.001) return 16;

  DvcChartDef chart_def;
  chart_def.source_range.start = a1;
  chart_def.source_range.end = b1;
  const char *chart_name = "CHART_ONE";
  uint32_t chart_name_len = (uint32_t)strlen(chart_name);
  if (dvc_chart_define(e, chart_name, chart_name_len, &chart_def) != DVC_OK) return 161;
  DvcChartOutput *chart_out = NULL;
  if (dvc_chart_get_output(e, chart_name, chart_name_len, &chart_out, &found) != DVC_OK || !found || !chart_out) return 162;
  uint32_t series_count = 0;
  if (dvc_chart_output_series_count(chart_out, &series_count) != DVC_OK || series_count == 0) return 163;

  if (dvc_insert_row(e, 1) != DVC_OK) return 164;
  if (dvc_cell_get_state(e, a1, &st) != DVC_OK) return 165;
  if (st.value.type != DVC_VALUE_BLANK) return 166;

  if (dvc_udf_register(e, "MYUDF", 5, NULL, NULL, DVC_VOLATILITY_EXTERNALLY_INVALIDATED) != DVC_ERR_NULL_POINTER) return 167;

  if (dvc_change_tracking_enable(e) != DVC_OK) return 17;
  if (dvc_cell_set_number(e, a1, 11.0) != DVC_OK) return 18;
  DvcChangeIterator *it = NULL;
  if (dvc_change_iterate(e, &it) != DVC_OK || !it) return 19;
  DvcChangeType ct;
  uint64_t epoch;
  int32_t done = 0;
  if (dvc_change_iterator_next(it, &ct, &epoch, &done) != DVC_OK || done != 0) return 20;
  if (dvc_change_iterator_destroy(it) != DVC_OK) return 21;

  DvcCellAddr bk254;
  if (dvc_parse_cell_ref(e, "BK254", 5, &bk254) != DVC_OK) return 22;
  if (bk254.col != 63 || bk254.row != 254) return 23;

  int32_t udf_found = 0;
  DvcStatus sreg = dvc_udf_register(e, "MYUDF", 5, udf_passthrough, NULL, DVC_VOLATILITY_EXTERNALLY_INVALIDATED);
  if (sreg != DVC_OK) return 231;
  if (dvc_invalidate_udf(e, "MYUDF", 5) != DVC_OK) return 232;
  if (dvc_udf_unregister(e, "MYUDF", 5, &udf_found) != DVC_OK || !udf_found) return 233;

  if (dvc_engine_destroy(e) != DVC_OK) return 24;
  FreeLibrary(dll);
  printf("api_smoke: ok\n");
  return 0;
}
