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

static int read_cell_number(const DvcEngine *e, uint16_t col, uint16_t row, double *out) {
  DvcCellState st;
  if (dvc_cell_get_state(e, (DvcCellAddr){col, row}, &st) != DVC_OK) return 0;
  if (st.value.type != DVC_VALUE_NUMBER) return 0;
  *out = st.value.number;
  return 1;
}

static int read_cell_input_text(const DvcEngine *e, uint16_t col, uint16_t row, char *buf, uint32_t cap) {
  uint32_t out_len = 0;
  if (dvc_cell_get_input_text(e, (DvcCellAddr){col, row}, NULL, 0, &out_len) != DVC_OK) return 0;
  if (out_len + 1 > cap) return 0;
  if (dvc_cell_get_input_text(e, (DvcCellAddr){col, row}, buf, cap, &out_len) != DVC_OK) return 0;
  buf[out_len] = '\0';
  return 1;
}

static int assert_epoch_order(const DvcEngine *e) {
  uint64_t committed = 0;
  uint64_t stabilized = 0;
  if (dvc_engine_committed_epoch(e, &committed) != DVC_OK) return 0;
  if (dvc_engine_stabilized_epoch(e, &stabilized) != DVC_OK) return 0;
  return stabilized <= committed;
}

static int assert_epoch_monotonic_step(const DvcEngine *e, uint64_t *prev_committed, uint64_t *prev_stabilized) {
  uint64_t committed = 0;
  uint64_t stabilized = 0;
  if (dvc_engine_committed_epoch(e, &committed) != DVC_OK) return 0;
  if (dvc_engine_stabilized_epoch(e, &stabilized) != DVC_OK) return 0;
  if (committed < *prev_committed || stabilized < *prev_stabilized) return 0;
  *prev_committed = committed;
  *prev_stabilized = stabilized;
  return 1;
}

static int capture_state_signature(const DvcEngine *e, DvcCellAddr addr, char *out, size_t cap) {
  DvcCellState st;
  if (dvc_cell_get_state(e, addr, &st) != DVC_OK) return 0;
  int n = snprintf(
      out,
      cap,
      "%u,%u|t=%d|n=%.17g|b=%d|e=%d|ve=%llu|st=%d",
      (unsigned)addr.col,
      (unsigned)addr.row,
      (int)st.value.type,
      st.value.number,
      (int)st.value.bool_val,
      (int)st.value.error_kind,
      (unsigned long long)st.value_epoch,
      (int)st.stale);
  return n > 0 && (size_t)n < cap;
}

static int run_replay_signature(char *out, size_t cap) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 10.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1+5", 4));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){3, 1}, "RANDARRAY(1,1,0,1,0)", 20));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){4, 1}, "STREAM(0.5)", 11));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){5, 1}, "LET(x,B1,x*2)", 13));

  int32_t any_advanced = 0;
  CHECK_OK(dvc_tick_streams(e, 0.6, &any_advanced));
  CHECK(any_advanced == 1);
  CHECK_OK(dvc_invalidate_volatile(e));

  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){6, 1}, "B1+C1", 5));
  CHECK_OK(dvc_recalculate(e));

  uint64_t committed = 0;
  uint64_t stabilized = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &committed));
  CHECK_OK(dvc_engine_stabilized_epoch(e, &stabilized));

  char s1[160], s2[160], s3[160], s4[160], s5[160], s6[160];
  CHECK(capture_state_signature(e, (DvcCellAddr){1, 1}, s1, sizeof(s1)));
  CHECK(capture_state_signature(e, (DvcCellAddr){2, 1}, s2, sizeof(s2)));
  CHECK(capture_state_signature(e, (DvcCellAddr){3, 1}, s3, sizeof(s3)));
  CHECK(capture_state_signature(e, (DvcCellAddr){4, 1}, s4, sizeof(s4)));
  CHECK(capture_state_signature(e, (DvcCellAddr){5, 1}, s5, sizeof(s5)));
  CHECK(capture_state_signature(e, (DvcCellAddr){6, 1}, s6, sizeof(s6)));

  int n = snprintf(
      out,
      cap,
      "epoch=%llu/%llu|%s|%s|%s|%s|%s|%s",
      (unsigned long long)committed,
      (unsigned long long)stabilized,
      s1, s2, s3, s4, s5, s6);
  CHECK(n > 0 && (size_t)n < cap);

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_epoch_001(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 1.0));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1+1", 4));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_recalculate(e));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_insert_row(e, 2));
  CHECK(assert_epoch_order(e));
  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_epoch_002(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  uint64_t prev_c = 0, prev_s = 0;
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));

  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 1.0));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_cell_set_text(e, (DvcCellAddr){1, 2}, "x", 1));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1+1", 4));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_recalculate(e));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_insert_col(e, 2));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_delete_col(e, 2));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_insert_row(e, 2));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_delete_row(e, 2));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));

  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_AUTOMATIC));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){3, 1}, 3.0));
  CHECK(assert_epoch_monotonic_step(e, &prev_c, &prev_s));

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_cell_001(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));
  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 5.0));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1+1", 4));

  DvcCellState st;
  uint64_t committed = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &committed));
  CHECK_OK(dvc_cell_get_state(e, (DvcCellAddr){2, 1}, &st));
  CHECK(st.stale == ((st.value_epoch < committed) ? 1 : 0));
  CHECK(st.stale == 1);

  CHECK_OK(dvc_recalculate(e));
  CHECK_OK(dvc_engine_committed_epoch(e, &committed));
  CHECK_OK(dvc_cell_get_state(e, (DvcCellAddr){2, 1}, &st));
  CHECK(st.stale == ((st.value_epoch < committed) ? 1 : 0));
  CHECK(st.stale == 0);

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_det_001(void) {
  char sig1[1400];
  char sig2[1400];
  CHECK(run_replay_signature(sig1, sizeof(sig1)) == 0);
  CHECK(run_replay_signature(sig2, sizeof(sig2)) == 0);
  CHECK(strcmp(sig1, sig2) == 0);
  return 0;
}

static int test_ct_str_001(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "SEQUENCE(2,2,1,1)", 17));
  double a1_before = 0.0;
  double b2_before = 0.0;
  CHECK(read_cell_number(e, 1, 1, &a1_before));
  CHECK(read_cell_number(e, 2, 2, &b2_before));

  char formula_before[64];
  CHECK(read_cell_input_text(e, 1, 1, formula_before, sizeof(formula_before)));

  uint64_t before_epoch = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &before_epoch));

  DvcStatus s = dvc_delete_row(e, 1);
  CHECK(s == DVC_REJECT_STRUCTURAL_CONSTRAINT);

  uint64_t after_epoch = 0;
  CHECK_OK(dvc_engine_committed_epoch(e, &after_epoch));
  CHECK(after_epoch == before_epoch);

  double a1_after = 0.0;
  double b2_after = 0.0;
  CHECK(read_cell_number(e, 1, 1, &a1_after));
  CHECK(read_cell_number(e, 2, 2, &b2_after));
  CHECK(nearly_equal(a1_before, a1_after));
  CHECK(nearly_equal(b2_before, b2_after));

  char formula_after[64];
  CHECK(read_cell_input_text(e, 1, 1, formula_after, sizeof(formula_after)));
  CHECK(strcmp(formula_before, formula_after) == 0);

  DvcRejectKind rk = DVC_REJECT_KIND_NONE;
  CHECK_OK(dvc_last_reject_kind(e, &rk));
  CHECK(rk == DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT);
  DvcLastRejectContext ctx;
  CHECK_OK(dvc_last_reject_context(e, &ctx));
  CHECK(ctx.reject_kind == DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT);
  CHECK(ctx.op_kind == DVC_STRUCT_OP_DELETE_ROW);
  CHECK(ctx.op_index == 1);
  CHECK(ctx.has_cell == 1);
  CHECK(ctx.has_range == 1);
  CHECK(ctx.range.start.col == 1 && ctx.range.start.row == 1);
  CHECK(ctx.range.end.col == 2 && ctx.range.end.row == 2);

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_cycle_001(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));
  CHECK_OK(dvc_change_tracking_enable(e));
  CHECK_OK(dvc_engine_set_recalc_mode(e, DVC_RECALC_MANUAL));

  DvcIterationConfig cfg;
  CHECK_OK(dvc_engine_get_iteration_config(e, &cfg));
  cfg.enabled = 0;
  CHECK_OK(dvc_engine_set_iteration_config(e, &cfg));

  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){1, 1}, 11.0));
  CHECK_OK(dvc_cell_set_number(e, (DvcCellAddr){2, 1}, 22.0));
  CHECK_OK(dvc_recalculate(e));

  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){1, 1}, "B1", 2));
  CHECK_OK(dvc_cell_set_formula(e, (DvcCellAddr){2, 1}, "A1", 2));
  CHECK_OK(dvc_recalculate(e));

  double a1 = 0.0;
  double b1 = 0.0;
  CHECK(read_cell_number(e, 1, 1, &a1));
  CHECK(read_cell_number(e, 2, 1, &b1));
  CHECK(nearly_equal(a1, 22.0));
  CHECK(nearly_equal(b1, 11.0));

  DvcChangeIterator *it = NULL;
  CHECK_OK(dvc_change_iterate(e, &it));
  CHECK(it != NULL);
  int found_diag = 0;
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
      if (code == DVC_DIAG_CIRCULAR_REFERENCE_DETECTED) found_diag = 1;
    }
  }
  CHECK(found_diag == 1);
  CHECK_OK(dvc_change_iterator_destroy(it));

  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_entities_001(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  DvcControlDef slider = {
      .kind = DVC_CONTROL_SLIDER,
      .min = 0.0,
      .max = 10.0,
      .step = 1.0,
  };
  DvcControlDef button = {
      .kind = DVC_CONTROL_BUTTON,
      .min = 0.0,
      .max = 0.0,
      .step = 0.0,
  };

  CHECK_OK(dvc_control_define(e, "speed", 5, &slider));
  CHECK_OK(dvc_control_define(e, "apply", 5, &button));

  DvcControlIterator *it = NULL;
  CHECK_OK(dvc_control_iterate(e, &it));
  CHECK(it != NULL);

  int32_t done = 0;
  int count = 0;
  while (!done) {
    uint32_t name_len = 0;
    DvcControlDef got_def;
    double got_value = 0.0;
    CHECK_OK(dvc_control_iterator_next(it, NULL, 0, &name_len, &got_def, &got_value, &done));
    if (done) break;
    CHECK(name_len < 64);
    char name_buf[64];
    CHECK_OK(dvc_control_iterator_next(it, name_buf, name_len, &name_len, &got_def, &got_value, &done));
    CHECK(done == 0);
    count++;
  }

  CHECK(count == 2);
  CHECK_OK(dvc_control_iterator_destroy(it));
  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

static int test_ct_entities_002(void) {
  DvcEngine *e = NULL;
  CHECK_OK(dvc_engine_create(&e));

  DvcChartDef chart = {
      .source_range = {
          .start = {1, 1},
          .end = {2, 2},
      },
  };
  CHECK_OK(dvc_chart_define(e, "main_chart", 10, &chart));

  DvcChartIterator *it = NULL;
  CHECK_OK(dvc_chart_iterate(e, &it));
  CHECK(it != NULL);

  int32_t done = 0;
  int count = 0;
  while (!done) {
    uint32_t name_len = 0;
    DvcChartDef got_def;
    CHECK_OK(dvc_chart_iterator_next(it, NULL, 0, &name_len, &got_def, &done));
    if (done) break;
    CHECK(name_len < 64);
    char name_buf[64];
    CHECK_OK(dvc_chart_iterator_next(it, name_buf, name_len, &name_len, &got_def, &done));
    CHECK(done == 0);
    count++;
  }

  CHECK(count == 1);
  CHECK_OK(dvc_chart_iterator_destroy(it));
  CHECK_OK(dvc_engine_destroy(e));
  return 0;
}

int main(void) {
  if (test_ct_epoch_001() != 0) return 1;
  printf("CT-EPOCH-001: pass\n");

  if (test_ct_epoch_002() != 0) return 1;
  printf("CT-EPOCH-002: pass\n");

  if (test_ct_cell_001() != 0) return 1;
  printf("CT-CELL-001: pass\n");

  if (test_ct_det_001() != 0) return 1;
  printf("CT-DET-001: pass\n");

  if (test_ct_str_001() != 0) return 1;
  printf("CT-STR-001: pass\n");

  if (test_ct_cycle_001() != 0) return 1;
  printf("CT-CYCLE-001: pass\n");

  if (test_ct_entities_001() != 0) return 1;
  printf("CT-ENTITIES-001: pass\n");

  if (test_ct_entities_002() != 0) return 1;
  printf("CT-ENTITIES-002: pass\n");

  printf("api_conformance_ct: ok\n");
  return 0;
}
