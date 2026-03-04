/* dvc_engine.c - Thin C FFI bridge for OCaml coreengine.
   All business logic lives in OCaml modules. This file only handles:
   - DLL export signatures matching dvc_engine.h
   - Type marshaling between C structs and OCaml values
   - Calling OCaml via caml_named_value + caml_callback */

#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <stdarg.h>
#include <caml/mlvalues.h>
#include <caml/callback.h>
#include <caml/memory.h>
#include <caml/alloc.h>
#include <caml/misc.h>

#ifdef _WIN32
#include <windows.h>
#endif

#define DVC_EXPORTS
#include "dvc_engine.h"

#ifdef _WIN32
static char_os dvc_ocaml_argv0[] = L"dvc_ocaml_engine";
#else
static char_os dvc_ocaml_argv0[] = "dvc_ocaml_engine";
#endif

static int trace_enabled_cached = -1;

static int trace_enabled(void) {
  if (trace_enabled_cached >= 0) return trace_enabled_cached;
  /* Trace is opt-in to avoid persistent log churn and perf noise. */
  {
    const char *env = getenv("DVC_OCAML_TRACE");
    trace_enabled_cached = (env != NULL && env[0] != '\0' && env[0] != '0') ? 1 : 0;
  }
  return trace_enabled_cached;
}

static void trace_log(const char *fmt, ...) {
  FILE *fp;
  va_list ap;
  if (!trace_enabled()) return;
  fp = fopen("trace.log", "a");
  if (!fp) return;
  va_start(ap, fmt);
  vfprintf(fp, fmt, ap);
  va_end(ap);
  fputc('\n', fp);
  fflush(fp);
  fclose(fp);
}

/* OCaml runtime embedding lock.
   We run all C->OCaml entrypoints under one process-wide mutex to keep
   host-side behavior deterministic and avoid runtime hook ordering issues
   seen with Windows DLL startup combinations. */

#ifdef _WIN32
static INIT_ONCE ocaml_init_once = INIT_ONCE_STATIC_INIT;
static CRITICAL_SECTION ocaml_big_lock;

static BOOL CALLBACK do_ocaml_init(PINIT_ONCE once, PVOID param, PVOID *ctx) {
  (void)once; (void)param; (void)ctx;
  trace_log("do_ocaml_init: begin");
  InitializeCriticalSection(&ocaml_big_lock);
  trace_log("do_ocaml_init: after InitializeCriticalSection");
  char_os *argv[] = { dvc_ocaml_argv0, NULL };
  trace_log("do_ocaml_init: before caml_startup");
  caml_startup(argv);
  trace_log("do_ocaml_init: after caml_startup");
  return TRUE;
}

static void ensure_ocaml_init(void) {
  InitOnceExecuteOnce(&ocaml_init_once, do_ocaml_init, NULL, NULL);
}

static void _ocaml_unlock(int *acquired) {
  if (*acquired) LeaveCriticalSection(&ocaml_big_lock);
}

/* Serialize all OCaml calls in-process. */
#define OCAML_LOCK() \
  int _ocaml_acq __attribute__((cleanup(_ocaml_unlock))) = 0; \
  ensure_ocaml_init(); \
  EnterCriticalSection(&ocaml_big_lock); \
  _ocaml_acq = 1

#else
static int ocaml_initialized = 0;

static void ensure_ocaml_init(void) {
  if (!ocaml_initialized) {
    char_os *argv[] = { dvc_ocaml_argv0, NULL };
    caml_startup(argv);
    ocaml_initialized = 1;
  }
}

#define OCAML_LOCK() ensure_ocaml_init()
#endif

static const value *get_cb(const char *name) {
  return caml_named_value(name);
}

struct DvcEngine { int handle; };
struct DvcCellIterator { int handle; int peeked; char peek_name[256]; uint32_t peek_name_len; char peek_text[4096]; uint32_t peek_text_len; int32_t peek_done; };
struct DvcNameIterator { int handle; int peeked; char peek_name[256]; uint32_t peek_name_len; int32_t peek_done; };
struct DvcFormatIterator { int handle; int peeked; int32_t peek_done; };
struct DvcControlIterator { int handle; int peeked; char peek_name[256]; uint32_t peek_name_len; DvcControlDef peek_def; double peek_value; int32_t peek_done; };
struct DvcChartIterator { int handle; int peeked; char peek_name[256]; uint32_t peek_name_len; DvcChartDef peek_def; int32_t peek_done; };
struct DvcChangeIterator { int handle; };
struct DvcChartOutput { int handle; };

static DvcStatus copy_str(value str, char *buf, uint32_t buf_len, uint32_t *out_len) {
  uint32_t slen = (uint32_t)caml_string_length(str);
  if (out_len) *out_len = slen;
  if (buf && buf_len > 0) {
    uint32_t n = slen < buf_len ? slen : buf_len;
    memcpy(buf, String_val(str), n);
  }
  return DVC_OK;
}

DVC_API DvcStatus dvc_engine_create(DvcEngine **out) {
  if (!out) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_engine_create: enter");
  OCAML_LOCK();
  const value *cb = get_cb("dvc_engine_create");
  if (!cb) return DVC_ERR_OUT_OF_MEMORY;
  value r = caml_callback(*cb, Val_unit);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) return s;
  DvcEngine *e = (DvcEngine *)malloc(sizeof(DvcEngine));
  if (!e) return DVC_ERR_OUT_OF_MEMORY;
  e->handle = Int_val(Field(r, 1));
  *out = e;
  trace_log("dvc_engine_create: ok handle=%d", e->handle);
  return DVC_OK;
}

DVC_API DvcStatus dvc_engine_create_with_bounds(DvcSheetBounds bounds, DvcEngine **out) {
  if (!out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_engine_create_with_bounds");
  if (!cb) return DVC_ERR_OUT_OF_MEMORY;
  value r = caml_callback2(*cb, Val_int(bounds.max_columns), Val_int(bounds.max_rows));
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) return s;
  DvcEngine *e = (DvcEngine *)malloc(sizeof(DvcEngine));
  if (!e) return DVC_ERR_OUT_OF_MEMORY;
  e->handle = Int_val(Field(r, 1));
  *out = e;
  return DVC_OK;
}

DVC_API DvcStatus dvc_engine_destroy(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_engine_destroy: enter handle=%d", engine->handle);
  OCAML_LOCK();
  const value *cb = get_cb("dvc_engine_destroy");
  if (cb) caml_callback(*cb, Val_int(engine->handle));
  free(engine);
  trace_log("dvc_engine_destroy: done");
  return DVC_OK;
}

DVC_API DvcStatus dvc_engine_clear(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_engine_clear");
  return Int_val(caml_callback(*cb, Val_int(engine->handle)));
}

DVC_API DvcStatus dvc_engine_bounds(const DvcEngine *engine, DvcSheetBounds *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_engine_bounds");
  value r = caml_callback(*cb, Val_int(engine->handle));
  if (Int_val(Field(r, 0)) != DVC_OK) return Int_val(Field(r, 0));
  out->max_columns = (uint16_t)Int_val(Field(r, 1));
  out->max_rows = (uint16_t)Int_val(Field(r, 2));
  return DVC_OK;
}

DVC_API DvcStatus dvc_engine_get_recalc_mode(const DvcEngine *engine, DvcRecalcMode *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_get_recalc_mode");
  value r = caml_callback(*cb, Val_int(engine->handle));
  *out = Int_val(Field(r, 1));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_engine_set_recalc_mode(DvcEngine *engine, DvcRecalcMode mode) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_engine_set_recalc_mode: enter handle=%d mode=%d", engine->handle, (int)mode);
  OCAML_LOCK();
  const value *cb = get_cb("dvc_set_recalc_mode");
  {
    DvcStatus st = Int_val(caml_callback2(*cb, Val_int(engine->handle), Val_int(mode)));
    trace_log("dvc_engine_set_recalc_mode: status=%d", (int)st);
    return st;
  }
}

DVC_API DvcStatus dvc_engine_committed_epoch(const DvcEngine *engine, uint64_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_engine_committed_epoch: enter handle=%d", engine->handle);
  OCAML_LOCK();
  const value *cb = get_cb("dvc_committed_epoch");
  value r = caml_callback(*cb, Val_int(engine->handle));
  *out = (uint64_t)Int_val(Field(r, 1));
  trace_log("dvc_engine_committed_epoch: status=%d epoch=%llu",
            Int_val(Field(r, 0)), (unsigned long long)(*out));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_engine_stabilized_epoch(const DvcEngine *engine, uint64_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_stabilized_epoch");
  value r = caml_callback(*cb, Val_int(engine->handle));
  *out = (uint64_t)Int_val(Field(r, 1));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_engine_is_stable(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_is_stable");
  value r = caml_callback(*cb, Val_int(engine->handle));
  *out = Int_val(Field(r, 1));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double val) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_cell_set_number: enter handle=%d c=%u r=%u v=%.17g",
            engine->handle, (unsigned)addr.col, (unsigned)addr.row, val);
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_set_number");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row), caml_copy_double(val) };
  {
    DvcStatus st = Int_val(caml_callbackN(*cb, 4, a));
    trace_log("dvc_cell_set_number: status=%d", (int)st);
    return st;
  }
}

DVC_API DvcStatus dvc_cell_set_text(DvcEngine *engine, DvcCellAddr addr, const char *text, uint32_t text_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!text && text_len > 0) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vt, r);
  const value *cb = get_cb("dvc_cell_set_text");
  vt = caml_alloc_string(text_len);
  if (text_len > 0) memcpy(Bytes_val(vt), text, text_len);
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row), vt };
  r = caml_callbackN(*cb, 4, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_cell_set_formula(DvcEngine *engine, DvcCellAddr addr, const char *formula, uint32_t formula_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!formula && formula_len > 0) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_cell_set_formula: enter handle=%d c=%u r=%u len=%u",
            engine->handle, (unsigned)addr.col, (unsigned)addr.row, (unsigned)formula_len);
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vf, r);
  const value *cb = get_cb("dvc_cell_set_formula");
  vf = caml_alloc_string(formula_len);
  if (formula_len > 0) memcpy(Bytes_val(vf), formula, formula_len);
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row), vf };
  r = caml_callbackN(*cb, 4, a);
  trace_log("dvc_cell_set_formula: status=%d", Int_val(r));
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_cell_clear(DvcEngine *engine, DvcCellAddr addr) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_clear");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  return Int_val(caml_callbackN(*cb, 3, a));
}

DVC_API DvcStatus dvc_cell_get_state(const DvcEngine *engine, DvcCellAddr addr, DvcCellState *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  trace_log("dvc_cell_get_state: enter handle=%d c=%u r=%u",
            engine->handle, (unsigned)addr.col, (unsigned)addr.row);
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_get_state");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*cb, 3, a);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) return s;
  out->value.type = Int_val(Field(r, 1));
  out->value.number = Double_val(Field(r, 2));
  out->value.bool_val = Int_val(Field(r, 3));
  out->value.error_kind = Int_val(Field(r, 4));
  out->value_epoch = (uint64_t)Int_val(Field(r, 5));
  out->stale = Int_val(Field(r, 6));
  trace_log("dvc_cell_get_state: ok type=%d stale=%d epoch=%llu",
            (int)out->value.type, (int)out->stale, (unsigned long long)out->value_epoch);
  return DVC_OK;
}

DVC_API DvcStatus dvc_cell_get_text(const DvcEngine *engine, DvcCellAddr addr, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_get_text");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*cb, 3, a);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_cell_get_input_type(const DvcEngine *engine, DvcCellAddr addr, DvcInputType *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_get_input_type");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*cb, 3, a);
  *out = Int_val(Field(r, 1));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_cell_get_input_text(const DvcEngine *engine, DvcCellAddr addr, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  const value *cb = get_cb("dvc_cell_get_input_text");
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*cb, 3, a);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

/* A1 variants */
static DvcStatus parse_a1(const DvcEngine *engine, const char *ref, uint32_t len, DvcCellAddr *out) {
  if (!engine || !ref || !out) return DVC_ERR_NULL_POINTER;
  trace_log("parse_a1: enter handle=%d len=%u", engine->handle, (unsigned)len);
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vr, r, tmp);
  const value *cb = get_cb("dvc_parse_cell_ref");
  vr = caml_alloc_string(len);
  memcpy(Bytes_val(vr), ref, len);
  value a[] = { Val_int(engine->handle), vr };
  r = caml_callbackN(*cb, 2, a);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) CAMLreturnT(DvcStatus, s);
  out->col = (uint16_t)Int_val(Field(r, 1));
  out->row = (uint16_t)Int_val(Field(r, 2));
  trace_log("parse_a1: ok c=%u r=%u", (unsigned)out->col, (unsigned)out->row);
  CAMLreturnT(DvcStatus, DVC_OK);
}

DVC_API DvcStatus dvc_cell_set_number_a1(DvcEngine *e, const char *r, uint32_t rl, double v) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_set_number(e,a,v);
}
DVC_API DvcStatus dvc_cell_set_text_a1(DvcEngine *e, const char *r, uint32_t rl, const char *t, uint32_t tl) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_set_text(e,a,t,tl);
}
DVC_API DvcStatus dvc_cell_set_formula_a1(DvcEngine *e, const char *r, uint32_t rl, const char *f, uint32_t fl) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_set_formula(e,a,f,fl);
}
DVC_API DvcStatus dvc_cell_clear_a1(DvcEngine *e, const char *r, uint32_t rl) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_clear(e,a);
}
DVC_API DvcStatus dvc_cell_get_state_a1(const DvcEngine *e, const char *r, uint32_t rl, DvcCellState *o) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_get_state(e,a,o);
}
DVC_API DvcStatus dvc_cell_get_text_a1(const DvcEngine *e, const char *r, uint32_t rl, char *b, uint32_t bl, uint32_t *ol) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_get_text(e,a,b,bl,ol);
}
DVC_API DvcStatus dvc_cell_get_input_type_a1(const DvcEngine *e, const char *r, uint32_t rl, DvcInputType *o) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_get_input_type(e,a,o);
}
DVC_API DvcStatus dvc_cell_get_input_text_a1(const DvcEngine *e, const char *r, uint32_t rl, char *b, uint32_t bl, uint32_t *ol) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_get_input_text(e,a,b,bl,ol);
}
DVC_API DvcStatus dvc_cell_get_format_a1(const DvcEngine *e, const char *r, uint32_t rl, DvcCellFormat *o) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_get_format(e,a,o);
}
DVC_API DvcStatus dvc_cell_set_format_a1(DvcEngine *e, const char *r, uint32_t rl, const DvcCellFormat *f) {
  OCAML_LOCK();
  DvcCellAddr a; DvcStatus s=parse_a1(e,r,rl,&a); return s?s:dvc_cell_set_format(e,a,f);
}

/* Names */
DVC_API DvcStatus dvc_name_set_number(DvcEngine *engine, const char *name, uint32_t name_len, double val) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  const value *cb = get_cb("dvc_name_set_number");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn, caml_copy_double(val) };
  r = caml_callbackN(*cb, 3, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_name_set_text(DvcEngine *engine, const char *name, uint32_t name_len, const char *text, uint32_t text_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, vt, r);
  const value *cb = get_cb("dvc_name_set_text");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  vt = caml_alloc_string(text_len); if (text_len > 0 && text) memcpy(Bytes_val(vt), text, text_len);
  value a[] = { Val_int(engine->handle), vn, vt };
  r = caml_callbackN(*cb, 3, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_name_set_formula(DvcEngine *engine, const char *name, uint32_t name_len, const char *formula, uint32_t formula_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, vf, r);
  const value *cb = get_cb("dvc_name_set_formula");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  vf = caml_alloc_string(formula_len); if (formula_len > 0 && formula) memcpy(Bytes_val(vf), formula, formula_len);
  value a[] = { Val_int(engine->handle), vn, vf };
  r = caml_callbackN(*cb, 3, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_name_clear(DvcEngine *engine, const char *name, uint32_t name_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  const value *cb = get_cb("dvc_name_clear");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*cb, 2, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_name_get_input_type(const DvcEngine *engine, const char *name, uint32_t name_len, DvcInputType *out) {
  if (!engine || !name || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  const value *cb = get_cb("dvc_name_get_input_type");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*cb, 2, a);
  *out = Int_val(Field(r, 1));
  CAMLreturnT(DvcStatus, Int_val(Field(r, 0)));
}

DVC_API DvcStatus dvc_name_get_input_text(const DvcEngine *engine, const char *name, uint32_t name_len, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  const value *cb = get_cb("dvc_name_get_input_text");
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*cb, 2, a);
  int s = Int_val(Field(r, 0));
  if (s != DVC_OK) CAMLreturnT(DvcStatus, s);
  CAMLreturnT(DvcStatus, copy_str(Field(r, 1), buf, buf_len, out_len));
}

DVC_API DvcStatus dvc_recalculate(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback(*get_cb("dvc_recalculate"), Val_int(engine->handle)));
}

DVC_API DvcStatus dvc_has_volatile_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_has_volatile"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_has_externally_invalidated_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_has_ext_invalidated"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_invalidate_volatile(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback(*get_cb("dvc_invalidate_volatile"), Val_int(engine->handle)));
}

DVC_API DvcStatus dvc_has_stream_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_has_streams"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_tick_streams(DvcEngine *engine, double elapsed_secs, int32_t *any_advanced) {
  if (!engine || !any_advanced) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), caml_copy_double(elapsed_secs) };
  value r = caml_callbackN(*get_cb("dvc_tick_streams"), 2, a);
  *any_advanced = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_invalidate_udf(DvcEngine *engine, const char *name, uint32_t name_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_invalidate_udf"), 2, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_cell_get_format(const DvcEngine *engine, DvcCellAddr addr, DvcCellFormat *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*get_cb("dvc_cell_get_format"), 3, a);
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  out->has_decimals = Bool_val(Field(r, 1)); out->decimals = Int_val(Field(r, 2));
  out->bold = Bool_val(Field(r, 3)); out->italic = Bool_val(Field(r, 4));
  out->fg = Int_val(Field(r, 5)); out->bg = Int_val(Field(r, 6));
  return DVC_OK;
}

DVC_API DvcStatus dvc_cell_set_format(DvcEngine *engine, DvcCellAddr addr, const DvcCellFormat *fmt) {
  if (!engine || !fmt) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row),
    Val_bool(fmt->has_decimals), Val_int(fmt->decimals), Val_bool(fmt->bold), Val_bool(fmt->italic),
    Val_int(fmt->fg), Val_int(fmt->bg) };
  return Int_val(caml_callbackN(*get_cb("dvc_cell_set_format"), 9, a));
}

DVC_API DvcStatus dvc_cell_spill_role(const DvcEngine *engine, DvcCellAddr addr, DvcSpillRole *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*get_cb("dvc_spill_role"), 3, a);
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  *out = Int_val(Field(r, 1)); return DVC_OK;
}

DVC_API DvcStatus dvc_cell_spill_anchor(const DvcEngine *engine, DvcCellAddr addr, DvcCellAddr *out, int32_t *found) {
  if (!engine || !out || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*get_cb("dvc_spill_anchor"), 3, a);
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  *found = Int_val(Field(r, 1)); out->col = Int_val(Field(r, 2)); out->row = Int_val(Field(r, 3));
  return DVC_OK;
}

DVC_API DvcStatus dvc_cell_spill_range(const DvcEngine *engine, DvcCellAddr addr, DvcCellRange *out, int32_t *found) {
  if (!engine || !out || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*get_cb("dvc_spill_range"), 3, a);
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  *found = Int_val(Field(r, 1));
  out->start.col = Int_val(Field(r, 2)); out->start.row = Int_val(Field(r, 3));
  out->end.col = Int_val(Field(r, 4)); out->end.row = Int_val(Field(r, 5));
  return DVC_OK;
}

/* Iterators */
DVC_API DvcStatus dvc_cell_iterate(const DvcEngine *engine, DvcCellIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_cell_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcCellIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_cell_iterator_next(DvcCellIterator *iter, DvcCellAddr *addr, DvcInputType *input_type, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_cell_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  if (addr) { addr->col = Int_val(Field(r, 1)); addr->row = Int_val(Field(r, 2)); }
  if (input_type) *input_type = Int_val(Field(r, 3));
  *done = Int_val(Field(r, 4)); return DVC_OK;
}

DVC_API DvcStatus dvc_cell_iterator_get_text(const DvcCellIterator *iter, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_cell_iter_get_text"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_cell_iterator_destroy(DvcCellIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_cell_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

DVC_API DvcStatus dvc_name_iterate(const DvcEngine *engine, DvcNameIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_name_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcNameIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_name_iterator_next(DvcNameIterator *iter, char *name_buf, uint32_t name_buf_len, uint32_t *name_len, DvcInputType *input_type, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_name_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  copy_str(Field(r, 1), name_buf, name_buf_len, name_len);
  if (input_type) *input_type = Int_val(Field(r, 2));
  *done = Int_val(Field(r, 3)); return DVC_OK;
}

DVC_API DvcStatus dvc_name_iterator_get_text(const DvcNameIterator *iter, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_name_iter_get_text"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_name_iterator_destroy(DvcNameIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_name_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

DVC_API DvcStatus dvc_format_iterate(const DvcEngine *engine, DvcFormatIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_format_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcFormatIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_format_iterator_next(DvcFormatIterator *iter, DvcCellAddr *addr, DvcCellFormat *format, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_format_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  if (addr) { addr->col = Int_val(Field(r, 1)); addr->row = Int_val(Field(r, 2)); }
  if (format) {
    format->has_decimals = Bool_val(Field(r, 3)); format->decimals = Int_val(Field(r, 4));
    format->bold = Bool_val(Field(r, 5)); format->italic = Bool_val(Field(r, 6));
    format->fg = Int_val(Field(r, 7)); format->bg = Int_val(Field(r, 8));
  }
  *done = Int_val(Field(r, 9)); return DVC_OK;
}

DVC_API DvcStatus dvc_format_iterator_destroy(DvcFormatIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_format_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

/* Structural */
DVC_API DvcStatus dvc_insert_row(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback2(*get_cb("dvc_insert_row"), Val_int(engine->handle), Val_int(at)));
}

DVC_API DvcStatus dvc_delete_row(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback2(*get_cb("dvc_delete_row"), Val_int(engine->handle), Val_int(at)));
}

DVC_API DvcStatus dvc_insert_col(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback2(*get_cb("dvc_insert_col"), Val_int(engine->handle), Val_int(at)));
}

DVC_API DvcStatus dvc_delete_col(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback2(*get_cb("dvc_delete_col"), Val_int(engine->handle), Val_int(at)));
}

/* Iteration config */
DVC_API DvcStatus dvc_engine_get_iteration_config(const DvcEngine *engine, DvcIterationConfig *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_get_iter_config"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  out->enabled = Bool_val(Field(r, 1)); out->max_iterations = Int_val(Field(r, 2));
  out->convergence_tolerance = Double_val(Field(r, 3)); return DVC_OK;
}

DVC_API DvcStatus dvc_engine_set_iteration_config(DvcEngine *engine, const DvcIterationConfig *config) {
  if (!engine || !config) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_bool(config->enabled), Val_int(config->max_iterations), caml_copy_double(config->convergence_tolerance) };
  return Int_val(caml_callbackN(*get_cb("dvc_set_iter_config"), 4, a));
}

/* Controls */
DVC_API DvcStatus dvc_control_define(DvcEngine *engine, const char *name, uint32_t name_len, const DvcControlDef *def) {
  if (!engine || !name || !def) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn, Val_int(def->kind), caml_copy_double(def->min), caml_copy_double(def->max), caml_copy_double(def->step) };
  r = caml_callbackN(*get_cb("dvc_control_define"), 6, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_control_remove(DvcEngine *engine, const char *name, uint32_t name_len, int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_control_remove"), 2, a);
  *found = Int_val(Field(r, 1));
  CAMLreturnT(DvcStatus, Int_val(Field(r, 0)));
}

DVC_API DvcStatus dvc_control_set_value(DvcEngine *engine, const char *name, uint32_t name_len, double val) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn, caml_copy_double(val) };
  r = caml_callbackN(*get_cb("dvc_control_set_value"), 3, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_control_get_value(const DvcEngine *engine, const char *name, uint32_t name_len, double *out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_control_get_value"), 2, a);
  *out = Double_val(Field(r, 1));
  *found = Int_val(Field(r, 2));
  CAMLreturnT(DvcStatus, Int_val(Field(r, 0)));
}

DVC_API DvcStatus dvc_control_get_def(const DvcEngine *engine, const char *name, uint32_t name_len, DvcControlDef *out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_control_get_def"), 2, a);
  out->kind = Int_val(Field(r, 1)); out->min = Double_val(Field(r, 2));
  out->max = Double_val(Field(r, 3)); out->step = Double_val(Field(r, 4));
  *found = Int_val(Field(r, 5));
  CAMLreturnT(DvcStatus, Int_val(Field(r, 0)));
}

DVC_API DvcStatus dvc_control_iterate(const DvcEngine *engine, DvcControlIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_control_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcControlIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_control_iterator_next(DvcControlIterator *iter, char *name_buf, uint32_t name_buf_len, uint32_t *name_len, DvcControlDef *def, double *val, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  if (iter->peeked && name_buf) {
    /* Return cached peek data and advance */
    if (name_len) *name_len = iter->peek_name_len;
    uint32_t n = iter->peek_name_len < name_buf_len ? iter->peek_name_len : name_buf_len;
    memcpy(name_buf, iter->peek_name, n);
    if (def) *def = iter->peek_def;
    if (val) *val = iter->peek_value;
    *done = 0;
    iter->peeked = 0;
    return DVC_OK;
  }
  value r = caml_callback(*get_cb("dvc_control_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  int32_t d = Int_val(Field(r, 7));
  *done = d;
  if (d) return DVC_OK;
  /* Extract data */
  value name_v = Field(r, 1);
  uint32_t slen = (uint32_t)caml_string_length(name_v);
  DvcControlDef got_def;
  got_def.kind = Int_val(Field(r, 2));
  got_def.min = Double_val(Field(r, 3));
  got_def.max = Double_val(Field(r, 4));
  got_def.step = Double_val(Field(r, 5));
  double got_val = Double_val(Field(r, 6));
  if (name_len) *name_len = slen;
  if (def) *def = got_def;
  if (val) *val = got_val;
  if (name_buf) {
    uint32_t cn = slen < name_buf_len ? slen : name_buf_len;
    memcpy(name_buf, String_val(name_v), cn);
  } else {
    /* Peek mode: cache the result */
    iter->peeked = 1;
    iter->peek_name_len = slen < 256 ? slen : 255;
    memcpy(iter->peek_name, String_val(name_v), iter->peek_name_len);
    iter->peek_def = got_def;
    iter->peek_value = got_val;
  }
  return DVC_OK;
}

DVC_API DvcStatus dvc_control_iterator_destroy(DvcControlIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_control_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

/* Charts */
DVC_API DvcStatus dvc_chart_define(DvcEngine *engine, const char *name, uint32_t name_len, const DvcChartDef *def) {
  if (!engine || !name || !def) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal2(vn, r);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn, Val_int(def->source_range.start.col), Val_int(def->source_range.start.row), Val_int(def->source_range.end.col), Val_int(def->source_range.end.row) };
  r = caml_callbackN(*get_cb("dvc_chart_define"), 6, a);
  CAMLreturnT(DvcStatus, Int_val(r));
}

DVC_API DvcStatus dvc_chart_remove(DvcEngine *engine, const char *name, uint32_t name_len, int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_chart_remove"), 2, a);
  *found = Int_val(Field(r, 1));
  CAMLreturnT(DvcStatus, Int_val(Field(r, 0)));
}

DVC_API DvcStatus dvc_chart_get_output(const DvcEngine *engine, const char *name, uint32_t name_len, DvcChartOutput **out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  CAMLparam0();
  CAMLlocal3(vn, r, tmp);
  vn = caml_alloc_string(name_len); memcpy(Bytes_val(vn), name, name_len);
  value a[] = { Val_int(engine->handle), vn };
  r = caml_callbackN(*get_cb("dvc_chart_get_output"), 2, a);
  int s = Int_val(Field(r, 0)); *found = Int_val(Field(r, 1));
  if (!*found) { *out = NULL; CAMLreturnT(DvcStatus, s); }
  DvcChartOutput *co = calloc(1, sizeof(*co));
  if (!co) CAMLreturnT(DvcStatus, DVC_ERR_OUT_OF_MEMORY);
  co->handle = Int_val(Field(r, 2));
  *out = co;
  CAMLreturnT(DvcStatus, s);
}

DVC_API DvcStatus dvc_chart_output_series_count(const DvcChartOutput *output, uint32_t *out) {
  if (!output || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_chart_series_count"), Val_int(output->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_chart_output_label_count(const DvcChartOutput *output, uint32_t *out) {
  if (!output || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_chart_label_count"), Val_int(output->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_chart_output_label(const DvcChartOutput *output, uint32_t index, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!output) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback2(*get_cb("dvc_chart_label"), Val_int(output->handle), Val_int(index));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_chart_output_series_name(const DvcChartOutput *output, uint32_t series_index, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!output) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback2(*get_cb("dvc_chart_series_name"), Val_int(output->handle), Val_int(series_index));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_chart_output_series_values(const DvcChartOutput *output, uint32_t series_index, double *buf, uint32_t buf_len, uint32_t *out_count) {
  if (!output) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback2(*get_cb("dvc_chart_series_values"), Val_int(output->handle), Val_int(series_index));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  value arr = Field(r, 1);
  uint32_t len = Wosize_val(arr) / Double_wosize;
  if (out_count) *out_count = len;
  if (buf && buf_len > 0) { uint32_t n = len < buf_len ? len : buf_len; for (uint32_t i = 0; i < n; i++) buf[i] = Double_flat_field(arr, i); }
  return DVC_OK;
}

DVC_API DvcStatus dvc_chart_iterate(const DvcEngine *engine, DvcChartIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_chart_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcChartIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_chart_iterator_next(DvcChartIterator *iter, char *name_buf, uint32_t name_buf_len, uint32_t *name_len, DvcChartDef *def, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  if (iter->peeked && name_buf) {
    if (name_len) *name_len = iter->peek_name_len;
    uint32_t n = iter->peek_name_len < name_buf_len ? iter->peek_name_len : name_buf_len;
    memcpy(name_buf, iter->peek_name, n);
    if (def) *def = iter->peek_def;
    *done = 0;
    iter->peeked = 0;
    return DVC_OK;
  }
  value r = caml_callback(*get_cb("dvc_chart_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  int32_t d = Int_val(Field(r, 6));
  *done = d;
  if (d) return DVC_OK;
  value name_v = Field(r, 1);
  uint32_t slen = (uint32_t)caml_string_length(name_v);
  DvcChartDef got_def;
  got_def.source_range.start.col = Int_val(Field(r, 2));
  got_def.source_range.start.row = Int_val(Field(r, 3));
  got_def.source_range.end.col = Int_val(Field(r, 4));
  got_def.source_range.end.row = Int_val(Field(r, 5));
  if (name_len) *name_len = slen;
  if (def) *def = got_def;
  if (name_buf) {
    uint32_t cn = slen < name_buf_len ? slen : name_buf_len;
    memcpy(name_buf, String_val(name_v), cn);
  } else {
    iter->peeked = 1;
    iter->peek_name_len = slen < 256 ? slen : 255;
    memcpy(iter->peek_name, String_val(name_v), iter->peek_name_len);
    iter->peek_def = got_def;
  }
  return DVC_OK;
}

DVC_API DvcStatus dvc_chart_iterator_destroy(DvcChartIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_chart_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

/* UDF */
#define MAX_UDFS 32
static struct { char name[64]; DvcUdfCallback cb; void *ud; DvcVolatility vol; int eh; } g_udfs[MAX_UDFS];
static int g_udf_count = 0;

DVC_API DvcStatus dvc_udf_register(DvcEngine *engine, const char *name, uint32_t name_len, DvcUdfCallback callback, void *user_data, DvcVolatility volatility) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  if (!callback) return DVC_ERR_NULL_POINTER;
  if (g_udf_count >= MAX_UDFS) return DVC_ERR_OUT_OF_MEMORY;
  uint32_t n = name_len < 63 ? name_len : 63;
  int i = g_udf_count++;
  memcpy(g_udfs[i].name, name, n); g_udfs[i].name[n] = '\0';
  g_udfs[i].cb = callback; g_udfs[i].ud = user_data; g_udfs[i].vol = volatility; g_udfs[i].eh = engine->handle;
  return DVC_OK;
}

DVC_API DvcStatus dvc_udf_unregister(DvcEngine *engine, const char *name, uint32_t name_len, int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  *found = 0;
  for (int i = 0; i < g_udf_count; i++) {
    if (g_udfs[i].eh == engine->handle && strlen(g_udfs[i].name) == name_len && strncmp(g_udfs[i].name, name, name_len) == 0) {
      *found = 1;
      for (int j = i; j < g_udf_count - 1; j++) g_udfs[j] = g_udfs[j + 1];
      g_udf_count--; break;
    }
  }
  return DVC_OK;
}

/* Change tracking */
DVC_API DvcStatus dvc_change_tracking_enable(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback(*get_cb("dvc_change_enable"), Val_int(engine->handle)));
}

DVC_API DvcStatus dvc_change_tracking_disable(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  return Int_val(caml_callback(*get_cb("dvc_change_disable"), Val_int(engine->handle)));
}

DVC_API DvcStatus dvc_change_tracking_is_enabled(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_is_enabled"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_change_iterate(DvcEngine *engine, DvcChangeIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_iterate"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  DvcChangeIterator *it = calloc(1, sizeof(*it));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->handle = Int_val(Field(r, 1));
  *out = it;
  return DVC_OK;
}

DVC_API DvcStatus dvc_change_iterator_next(DvcChangeIterator *iter, DvcChangeType *change_type, uint64_t *epoch, int32_t *done) {
  if (!iter || !done) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_iter_next"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  if (change_type) *change_type = Int_val(Field(r, 1));
  if (epoch) *epoch = Int_val(Field(r, 2));
  *done = Int_val(Field(r, 3)); return DVC_OK;
}

DVC_API DvcStatus dvc_change_get_cell(const DvcChangeIterator *iter, DvcCellAddr *addr) {
  if (!iter || !addr) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_get_cell"), Val_int(iter->handle));
  addr->col = Int_val(Field(r, 1)); addr->row = Int_val(Field(r, 2));
  return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_change_get_name(const DvcChangeIterator *iter, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_get_name"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_change_get_chart_name(const DvcChangeIterator *iter, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_get_chart_name"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_change_get_spill(const DvcChangeIterator *iter, DvcCellAddr *anchor, DvcCellRange *old_range, int32_t *had_old, DvcCellRange *new_range, int32_t *has_new) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  if (anchor) { anchor->col = 0; anchor->row = 0; }
  if (had_old) *had_old = 0;
  if (has_new) *has_new = 0;
  if (old_range) memset(old_range, 0, sizeof(*old_range));
  if (new_range) memset(new_range, 0, sizeof(*new_range));
  return DVC_OK;
}

DVC_API DvcStatus dvc_change_get_format(const DvcChangeIterator *iter, DvcCellAddr *addr, DvcCellFormat *old_fmt, DvcCellFormat *new_fmt) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_get_format"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  if (addr) { addr->col = Int_val(Field(r, 1)); addr->row = Int_val(Field(r, 2)); }
  if (old_fmt) { old_fmt->has_decimals = Bool_val(Field(r, 3)); old_fmt->decimals = Int_val(Field(r, 4)); old_fmt->bold = Bool_val(Field(r, 5)); old_fmt->italic = Bool_val(Field(r, 6)); old_fmt->fg = Int_val(Field(r, 7)); old_fmt->bg = Int_val(Field(r, 8)); }
  if (new_fmt) { new_fmt->has_decimals = Bool_val(Field(r, 9)); new_fmt->decimals = Int_val(Field(r, 10)); new_fmt->bold = Bool_val(Field(r, 11)); new_fmt->italic = Bool_val(Field(r, 12)); new_fmt->fg = Int_val(Field(r, 13)); new_fmt->bg = Int_val(Field(r, 14)); }
  return DVC_OK;
}

DVC_API DvcStatus dvc_change_get_diagnostic(const DvcChangeIterator *iter, DvcDiagnosticCode *code, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_change_get_diagnostic"), Val_int(iter->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  if (code) *code = Int_val(Field(r, 1));
  return copy_str(Field(r, 2), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_change_iterator_destroy(DvcChangeIterator *iter) {
  if (!iter) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  caml_callback(*get_cb("dvc_change_iter_destroy"), Val_int(iter->handle));
  free(iter); return DVC_OK;
}

/* Error/reject */
DVC_API DvcStatus dvc_last_error_message(const DvcEngine *engine, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_last_error_message"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_last_error_kind(const DvcEngine *engine, DvcStatus *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_last_error_kind"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_last_reject_kind(const DvcEngine *engine, DvcRejectKind *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_last_reject_kind"), Val_int(engine->handle));
  *out = Int_val(Field(r, 1)); return Int_val(Field(r, 0));
}

DVC_API DvcStatus dvc_last_reject_context(const DvcEngine *engine, DvcLastRejectContext *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_last_reject_context"), Val_int(engine->handle));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  out->reject_kind = Int_val(Field(r, 1)); out->op_kind = Int_val(Field(r, 2)); out->op_index = Int_val(Field(r, 3));
  out->has_cell = Int_val(Field(r, 4)); out->cell.col = Int_val(Field(r, 5)); out->cell.row = Int_val(Field(r, 6));
  out->has_range = Int_val(Field(r, 7)); out->range.start.col = Int_val(Field(r, 8)); out->range.start.row = Int_val(Field(r, 9));
  out->range.end.col = Int_val(Field(r, 10)); out->range.end.row = Int_val(Field(r, 11));
  return DVC_OK;
}

DVC_API DvcStatus dvc_cell_error_message(const DvcEngine *engine, DvcCellAddr addr, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  OCAML_LOCK();
  value a[] = { Val_int(engine->handle), Val_int(addr.col), Val_int(addr.row) };
  value r = caml_callbackN(*get_cb("dvc_cell_error_message"), 3, a);
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_palette_color_name(DvcPaletteColor color, char *buf, uint32_t buf_len, uint32_t *out_len) {
  OCAML_LOCK();
  value r = caml_callback(*get_cb("dvc_palette_color_name"), Val_int(color));
  int s = Int_val(Field(r, 0)); if (s != DVC_OK) return s;
  return copy_str(Field(r, 1), buf, buf_len, out_len);
}

DVC_API DvcStatus dvc_parse_cell_ref(const DvcEngine *engine, const char *ref_str, uint32_t ref_len, DvcCellAddr *out) {
  return parse_a1(engine, ref_str, ref_len, out);
}

DVC_API uint32_t dvc_api_version(void) {
  OCAML_LOCK();
  return 100;
}
