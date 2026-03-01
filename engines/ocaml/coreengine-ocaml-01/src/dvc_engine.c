#include "dvc_engine.h"

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#define strncasecmp _strnicmp
#endif

typedef struct {
  DvcInputType kind;
  double number;
  char *text;
} CellInput;

typedef struct {
  DvcCellValue value;
  uint64_t value_epoch;
  int32_t stale;
  char *text;
  char *error_message;
  DvcSpillRole spill_role;
  int spill_anchor_found;
  DvcCellAddr spill_anchor;
  int spill_range_found;
  DvcCellRange spill_range;
} CellComputed;

typedef struct {
  char *name;
  DvcInputType kind;
  double number;
  char *text;
} NameEntry;

typedef struct {
  char *name;
  DvcControlDef def;
  double value;
} ControlEntry;

typedef struct {
  char *name;
  DvcChartDef def;
} ChartEntry;

typedef struct {
  char *name;
  DvcUdfCallback callback;
  void *user_data;
  DvcVolatility volatility;
} UdfEntry;

typedef struct {
  DvcChangeType type;
  uint64_t epoch;
  DvcCellAddr cell;
  DvcCellRange old_spill;
  DvcCellRange new_spill;
  int had_old;
  int has_new;
  DvcCellFormat old_fmt;
  DvcCellFormat new_fmt;
  char *name;
  char *chart_name;
  DvcDiagnosticCode diag_code;
  char *diag_message;
} ChangeEntry;

struct DvcEngine {
  DvcSheetBounds bounds;
  DvcRecalcMode recalc_mode;
  DvcIterationConfig iter_cfg;
  uint64_t committed_epoch;
  uint64_t stabilized_epoch;

  CellInput *cells;
  CellComputed *computed;
  DvcCellFormat *formats;
  size_t cell_count;

  NameEntry *names;
  uint32_t name_count;
  uint32_t name_cap;

  ControlEntry *controls;
  uint32_t control_count;
  uint32_t control_cap;

  ChartEntry *charts;
  uint32_t chart_count;
  uint32_t chart_cap;

  UdfEntry *udfs;
  uint32_t udf_count;
  uint32_t udf_cap;

  int *stream_active;
  double *stream_period;
  double *stream_elapsed;
  double *stream_counter;

  int change_tracking_enabled;
  ChangeEntry *changes;
  uint32_t change_count;
  uint32_t change_cap;

  DvcStatus last_error_kind;
  char *last_error_message;
  DvcRejectKind last_reject_kind;
  DvcLastRejectContext last_reject_context;
};

struct DvcCellIterator {
  const DvcEngine *engine;
  uint32_t index;
  int has_current;
  uint32_t current_index;
};

struct DvcNameIterator {
  const DvcEngine *engine;
  uint32_t *order;
  uint32_t count;
  uint32_t index;
  int has_current;
  uint32_t current_index;
};

struct DvcFormatIterator {
  const DvcEngine *engine;
  uint32_t index;
};

struct DvcControlIterator {
  const DvcEngine *engine;
  uint32_t *order;
  uint32_t count;
  uint32_t index;
};

struct DvcChartIterator {
  const DvcEngine *engine;
  uint32_t *order;
  uint32_t count;
  uint32_t index;
};

struct DvcChartOutput {
  uint32_t series_count;
  uint32_t label_count;
  char **labels;
  char **series_names;
  double **series_values;
  uint32_t *series_value_counts;
};

struct DvcChangeIterator {
  ChangeEntry *items;
  uint32_t count;
  uint32_t index;
  int has_current;
};

static int is_default_format(const DvcCellFormat *fmt) {
  return fmt->has_decimals == 0 && fmt->decimals == 0 && fmt->bold == 0 && fmt->italic == 0 && fmt->fg == DVC_COLOR_NONE && fmt->bg == DVC_COLOR_NONE;
}

static DvcCellFormat default_format(void) {
  DvcCellFormat fmt;
  fmt.has_decimals = 0;
  fmt.decimals = 0;
  fmt.bold = 0;
  fmt.italic = 0;
  fmt.fg = DVC_COLOR_NONE;
  fmt.bg = DVC_COLOR_NONE;
  return fmt;
}

static void clear_error(DvcEngine *e) {
  if (!e) return;
  e->last_error_kind = DVC_OK;
  free(e->last_error_message);
  e->last_error_message = NULL;
}

static void clear_reject(DvcEngine *e) {
  if (!e) return;
  e->last_reject_kind = DVC_REJECT_KIND_NONE;
  memset(&e->last_reject_context, 0, sizeof(e->last_reject_context));
}

static void set_error(DvcEngine *e, DvcStatus code, const char *msg) {
  if (!e) return;
  e->last_error_kind = code;
  free(e->last_error_message);
  if (msg) {
    size_t n = strlen(msg);
    e->last_error_message = (char *)malloc(n + 1);
    if (e->last_error_message) memcpy(e->last_error_message, msg, n + 1);
  } else {
    e->last_error_message = NULL;
  }
  clear_reject(e);
}

static void set_reject(DvcEngine *e, DvcStatus code, DvcRejectKind kind, DvcStructuralOpKind op_kind, uint16_t op_index) {
  if (!e) return;
  clear_error(e);
  e->last_reject_kind = kind;
  memset(&e->last_reject_context, 0, sizeof(e->last_reject_context));
  e->last_reject_context.reject_kind = kind;
  e->last_reject_context.op_kind = op_kind;
  e->last_reject_context.op_index = op_index;
  (void)code;
}

static void clear_status(DvcEngine *e) {
  if (!e) return;
  clear_error(e);
  clear_reject(e);
}

static int valid_color(DvcPaletteColor c) {
  return c == DVC_COLOR_NONE || (c >= 0 && c < DVC_PALETTE_COUNT);
}

static int valid_addr(const DvcEngine *e, DvcCellAddr addr) {
  return addr.col >= 1 && addr.row >= 1 && addr.col <= e->bounds.max_columns && addr.row <= e->bounds.max_rows;
}

static uint32_t addr_to_index(const DvcEngine *e, DvcCellAddr addr) {
  return (uint32_t)((addr.row - 1) * e->bounds.max_columns + (addr.col - 1));
}

static DvcCellAddr index_to_addr(const DvcEngine *e, uint32_t index) {
  DvcCellAddr a;
  a.row = (uint16_t)(index / e->bounds.max_columns + 1);
  a.col = (uint16_t)(index % e->bounds.max_columns + 1);
  return a;
}

static void free_cell_input(CellInput *ci) {
  free(ci->text);
  ci->text = NULL;
  ci->kind = DVC_INPUT_EMPTY;
  ci->number = 0.0;
}

static void free_cell_computed(CellComputed *cc) {
  free(cc->text);
  cc->text = NULL;
  free(cc->error_message);
  cc->error_message = NULL;
  cc->value.type = DVC_VALUE_BLANK;
  cc->value.number = 0.0;
  cc->value.bool_val = 0;
  cc->value.error_kind = DVC_CELL_ERR_NULL;
}

static char *dup_n(const char *s, uint32_t n) {
  if (!s) return NULL;
  char *out = (char *)malloc((size_t)n + 1);
  if (!out) return NULL;
  memcpy(out, s, n);
  out[n] = '\0';
  return out;
}

static char *dup_cstr(const char *s) {
  if (!s) return NULL;
  size_t n = strlen(s);
  char *out = (char *)malloc(n + 1);
  if (!out) return NULL;
  memcpy(out, s, n + 1);
  return out;
}

static int cmp_ci(const char *a, const char *b) {
  while (*a && *b) {
    int ca = toupper((unsigned char)*a);
    int cb = toupper((unsigned char)*b);
    if (ca != cb) return ca - cb;
    ++a;
    ++b;
  }
  return toupper((unsigned char)*a) - toupper((unsigned char)*b);
}

static char *upper_name(const char *name, uint32_t len) {
  char *u = dup_n(name, len);
  if (!u) return NULL;
  for (uint32_t i = 0; i < len; ++i) {
    u[i] = (char)toupper((unsigned char)u[i]);
  }
  return u;
}

static int parse_cell_ref_raw(const char *s, uint32_t len, DvcCellAddr *out, int *used_chars) {
  if (len == 0) return 0;
  uint32_t i = 0;
  if (i < len && s[i] == '$') i++;
  if (i >= len || !isalpha((unsigned char)s[i])) return 0;
  uint32_t col = 0;
  uint32_t col_letters = 0;
  while (i < len && isalpha((unsigned char)s[i])) {
    col = col * 26 + (uint32_t)(toupper((unsigned char)s[i]) - 'A' + 1);
    i++;
    col_letters++;
    if (col_letters > 3) return 0;
  }
  if (i < len && s[i] == '$') i++;
  if (i >= len || !isdigit((unsigned char)s[i]) || s[i] == '0') return 0;
  uint32_t row = 0;
  while (i < len && isdigit((unsigned char)s[i])) {
    row = row * 10 + (uint32_t)(s[i] - '0');
    i++;
    if (row > 65535) return 0;
  }
  if (out) {
    out->col = (uint16_t)col;
    out->row = (uint16_t)row;
  }
  if (used_chars) *used_chars = (int)i;
  return 1;
}

static int parse_a1_addr(const DvcEngine *e, const char *s, uint32_t len, DvcCellAddr *out) {
  int used = 0;
  if (!parse_cell_ref_raw(s, len, out, &used)) return 0;
  if ((uint32_t)used != len) return 0;
  if (!valid_addr(e, *out)) return 0;
  return 1;
}

static DvcStatus copy_out_text(const char *src, char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!out_len) return DVC_ERR_NULL_POINTER;
  uint32_t n = src ? (uint32_t)strlen(src) : 0;
  *out_len = n;
  if (!buf) return DVC_OK;
  if (buf_len < n) return DVC_ERR_INVALID_ARGUMENT;
  if (n > 0) memcpy(buf, src, n);
  return DVC_OK;
}

static int ensure_name_cap(DvcEngine *e) {
  if (e->name_count < e->name_cap) return 1;
  uint32_t new_cap = e->name_cap == 0 ? 8u : e->name_cap * 2u;
  NameEntry *n = (NameEntry *)realloc(e->names, sizeof(NameEntry) * new_cap);
  if (!n) return 0;
  e->names = n;
  e->name_cap = new_cap;
  return 1;
}

static int ensure_control_cap(DvcEngine *e) {
  if (e->control_count < e->control_cap) return 1;
  uint32_t new_cap = e->control_cap == 0 ? 8u : e->control_cap * 2u;
  ControlEntry *n = (ControlEntry *)realloc(e->controls, sizeof(ControlEntry) * new_cap);
  if (!n) return 0;
  e->controls = n;
  e->control_cap = new_cap;
  return 1;
}

static int ensure_chart_cap(DvcEngine *e) {
  if (e->chart_count < e->chart_cap) return 1;
  uint32_t new_cap = e->chart_cap == 0 ? 8u : e->chart_cap * 2u;
  ChartEntry *n = (ChartEntry *)realloc(e->charts, sizeof(ChartEntry) * new_cap);
  if (!n) return 0;
  e->charts = n;
  e->chart_cap = new_cap;
  return 1;
}

static int ensure_udf_cap(DvcEngine *e) {
  if (e->udf_count < e->udf_cap) return 1;
  uint32_t new_cap = e->udf_cap == 0 ? 8u : e->udf_cap * 2u;
  UdfEntry *n = (UdfEntry *)realloc(e->udfs, sizeof(UdfEntry) * new_cap);
  if (!n) return 0;
  e->udfs = n;
  e->udf_cap = new_cap;
  return 1;
}

static int ensure_change_cap(DvcEngine *e) {
  if (e->change_count < e->change_cap) return 1;
  uint32_t new_cap = e->change_cap == 0 ? 32u : e->change_cap * 2u;
  ChangeEntry *n = (ChangeEntry *)realloc(e->changes, sizeof(ChangeEntry) * new_cap);
  if (!n) return 0;
  e->changes = n;
  e->change_cap = new_cap;
  return 1;
}

static void change_push_cell(DvcEngine *e, DvcCellAddr addr) {
  if (!e || !e->change_tracking_enabled) return;
  if (!ensure_change_cap(e)) return;
  ChangeEntry *ce = &e->changes[e->change_count++];
  memset(ce, 0, sizeof(*ce));
  ce->type = DVC_CHANGE_CELL_VALUE;
  ce->epoch = e->committed_epoch;
  ce->cell = addr;
}

static void change_push_name(DvcEngine *e, const char *name) {
  if (!e || !e->change_tracking_enabled) return;
  if (!ensure_change_cap(e)) return;
  ChangeEntry *ce = &e->changes[e->change_count++];
  memset(ce, 0, sizeof(*ce));
  ce->type = DVC_CHANGE_NAME_VALUE;
  ce->epoch = e->committed_epoch;
  ce->name = dup_cstr(name);
}

static void change_push_format(DvcEngine *e, DvcCellAddr addr, const DvcCellFormat *oldf, const DvcCellFormat *newf) {
  if (!e || !e->change_tracking_enabled) return;
  if (!ensure_change_cap(e)) return;
  ChangeEntry *ce = &e->changes[e->change_count++];
  memset(ce, 0, sizeof(*ce));
  ce->type = DVC_CHANGE_CELL_FORMAT;
  ce->epoch = e->committed_epoch;
  ce->cell = addr;
  ce->old_fmt = *oldf;
  ce->new_fmt = *newf;
}

static void change_push_diag(DvcEngine *e, DvcDiagnosticCode code, const char *message) {
  if (!e || !e->change_tracking_enabled) return;
  if (!ensure_change_cap(e)) return;
  ChangeEntry *ce = &e->changes[e->change_count++];
  memset(ce, 0, sizeof(*ce));
  ce->type = DVC_CHANGE_DIAGNOSTIC;
  ce->epoch = e->committed_epoch;
  ce->diag_code = code;
  ce->diag_message = dup_cstr(message ? message : "");
}

static int name_is_valid_identifier(const char *name) {
  if (!name || !*name) return 0;
  if (!(isalpha((unsigned char)name[0]) || name[0] == '_')) return 0;
  for (const char *p = name + 1; *p; ++p) {
    if (!(isalnum((unsigned char)*p) || *p == '_')) return 0;
  }
  return 1;
}

static int name_conflicts_builtin(const char *name_up) {
  const char *builtins[] = {
    "TRUE", "FALSE", "SUM", "MIN", "MAX", "AVERAGE", "COUNT", "IF", "IFERROR", "IFNA", "NA", "ERROR",
    "AND", "OR", "NOT", "ISERROR", "ISNA", "ISBLANK", "ISTEXT", "ISNUMBER", "ISLOGICAL", "ERROR.TYPE",
    "ABS", "INT", "ROUND", "SIGN", "SQRT", "EXP", "LN", "LOG10", "SIN", "COS", "TAN", "ATN", "PI",
    "NPV", "PV", "FV", "PMT", "LOOKUP", "CONCAT", "LEN", "SEQUENCE", "RANDARRAY", "LET", "LAMBDA",
    "MAP", "INDIRECT", "OFFSET", "ROW", "COLUMN", "NOW", "RAND", "STREAM", NULL
  };
  for (int i = 0; builtins[i]; ++i) {
    if (strcmp(name_up, builtins[i]) == 0) return 1;
  }
  return 0;
}

static int name_is_cell_like(const char *name_up) {
  DvcCellAddr a;
  int used = 0;
  if (!parse_cell_ref_raw(name_up, (uint32_t)strlen(name_up), &a, &used)) return 0;
  return used == (int)strlen(name_up);
}

static int find_name_index(const DvcEngine *e, const char *name_up) {
  for (uint32_t i = 0; i < e->name_count; ++i) {
    if (strcmp(e->names[i].name, name_up) == 0) return (int)i;
  }
  return -1;
}

static int find_control_index(const DvcEngine *e, const char *name_up) {
  for (uint32_t i = 0; i < e->control_count; ++i) {
    if (strcmp(e->controls[i].name, name_up) == 0) return (int)i;
  }
  return -1;
}

static int find_chart_index(const DvcEngine *e, const char *name_up) {
  for (uint32_t i = 0; i < e->chart_count; ++i) {
    if (strcmp(e->charts[i].name, name_up) == 0) return (int)i;
  }
  return -1;
}

static int find_udf_index(const DvcEngine *e, const char *name_up) {
  for (uint32_t i = 0; i < e->udf_count; ++i) {
    if (strcmp(e->udfs[i].name, name_up) == 0) return (int)i;
  }
  return -1;
}

static int formula_is_stream(const char *f) {
  return f && (strncmp(f, "=STREAM(", 8) == 0 || strncmp(f, "STREAM(", 7) == 0);
}

static int formula_is_volatile(const char *f) {
  return f && (strstr(f, "RAND(") || strstr(f, "RANDARRAY(") || strstr(f, "NOW("));
}

static double parse_first_number(const char *s, double fallback) {
  if (!s) return fallback;
  char *end = NULL;
  double v = strtod(s, &end);
  if (end == s) return fallback;
  return v;
}

typedef struct {
  const char *var_name;
  double var_value;
  int has_var;
  const double *prior_numbers;
  const DvcValueType *prior_types;
  const uint8_t *cycle_nodes;
  int non_iterative_cycle;
} EvalCtx;

typedef struct {
  uint16_t col;
  uint16_t row;
  int abs_col;
  int abs_row;
  int used_chars;
} RefToken;

typedef struct {
  DvcValueType type;
  double number;
  int bool_val;
  DvcCellErrorKind error_kind;
  char text[512];
} SimpleArgValue;

static int starts_with_ci(const char *s, const char *prefix) {
  if (!s || !prefix) return 0;
  while (*prefix) {
    if (!*s) return 0;
    if (toupper((unsigned char)*s) != toupper((unsigned char)*prefix)) return 0;
    ++s;
    ++prefix;
  }
  return 1;
}

static const char *skip_ws(const char *s) {
  while (s && *s && isspace((unsigned char)*s)) ++s;
  return s;
}

static void trim_span(const char *s, const char **out_start, size_t *out_len) {
  const char *start = s;
  const char *end = s ? s + strlen(s) : s;
  while (start && *start && isspace((unsigned char)*start)) ++start;
  while (end && end > start && isspace((unsigned char)*(end - 1))) --end;
  *out_start = start;
  *out_len = (size_t)(end - start);
}

static char *dup_trim(const char *s) {
  const char *start = NULL;
  size_t len = 0;
  trim_span(s, &start, &len);
  return dup_n(start ? start : "", (uint32_t)len);
}

static void trim_inplace(char *s) {
  if (!s) return;
  const char *start = NULL;
  size_t len = 0;
  trim_span(s, &start, &len);
  if (!start) {
    s[0] = '\0';
    return;
  }
  if (start != s) memmove(s, start, len);
  s[len] = '\0';
}

static int parse_ref_token(const char *s, RefToken *out) {
  if (!s || !out) return 0;
  int i = 0;
  int abs_col = 0;
  int abs_row = 0;
  if (s[i] == '$') {
    abs_col = 1;
    ++i;
  }
  if (!isalpha((unsigned char)s[i])) return 0;
  uint32_t col = 0;
  int letters = 0;
  while (isalpha((unsigned char)s[i])) {
    col = col * 26u + (uint32_t)(toupper((unsigned char)s[i]) - 'A' + 1);
    ++i;
    ++letters;
    if (letters > 3) return 0;
  }
  if (s[i] == '$') {
    abs_row = 1;
    ++i;
  }
  if (!isdigit((unsigned char)s[i]) || s[i] == '0') return 0;
  uint32_t row = 0;
  while (isdigit((unsigned char)s[i])) {
    row = row * 10u + (uint32_t)(s[i] - '0');
    ++i;
    if (row > 65535u) return 0;
  }
  out->col = (uint16_t)col;
  out->row = (uint16_t)row;
  out->abs_col = abs_col;
  out->abs_row = abs_row;
  out->used_chars = i;
  return 1;
}

static int col_to_label(uint16_t col, char *buf, size_t cap) {
  if (!buf || cap < 2) return 0;
  char rev[8];
  size_t n = 0;
  uint32_t x = col;
  if (x == 0) return 0;
  while (x > 0 && n < sizeof(rev)) {
    uint32_t rem = (x - 1u) % 26u;
    rev[n++] = (char)('A' + rem);
    x = (x - 1u) / 26u;
  }
  if (x != 0 || n + 1 > cap) return 0;
  for (size_t i = 0; i < n; ++i) buf[i] = rev[n - 1 - i];
  buf[n] = '\0';
  return 1;
}

static int format_ref_token(const RefToken *rt, char *out, size_t cap) {
  if (!rt || !out || cap < 4) return 0;
  char colbuf[8];
  if (!col_to_label(rt->col, colbuf, sizeof(colbuf))) return 0;
  int n = snprintf(out, cap, "%s%s%s%u", rt->abs_col ? "$" : "", colbuf, rt->abs_row ? "$" : "", (unsigned)rt->row);
  return n > 0 && (size_t)n < cap;
}

static int split_top_level_args(const char *s, char parts[][256], int max_parts) {
  if (!s || !parts || max_parts <= 0) return 0;
  int count = 0;
  int depth = 0;
  int in_string = 0;
  const char *seg = s;
  const char *p = s;
  for (;;) {
    char ch = *p;
    int at_end = (ch == '\0');
    if (!at_end) {
      if (ch == '"') {
        in_string = !in_string;
      } else if (!in_string) {
        if (ch == '(') depth++;
        else if (ch == ')') depth--;
      }
    }
    if (at_end || (!in_string && depth == 0 && ch == ',')) {
      if (count >= max_parts) return 0;
      size_t raw_len = (size_t)(p - seg);
      if (raw_len >= 255) raw_len = 255;
      memcpy(parts[count], seg, raw_len);
      parts[count][raw_len] = '\0';
      trim_inplace(parts[count]);
      count++;
      if (at_end) break;
      seg = p + 1;
    }
    if (at_end) break;
    ++p;
  }
  return count;
}

static int parse_r1c1_absolute(const DvcEngine *e, const char *text, DvcCellAddr *out) {
  if (!e || !text || !out) return 0;
  const char *s = skip_ws(text);
  if (toupper((unsigned char)s[0]) != 'R') return 0;
  char *end = NULL;
  long row = strtol(s + 1, &end, 10);
  if (end == s + 1 || toupper((unsigned char)*end) != 'C') return 0;
  long col = strtol(end + 1, &end, 10);
  end = (char *)skip_ws(end);
  if (*end != '\0') return 0;
  if (row <= 0 || col <= 0) return 0;
  DvcCellAddr addr;
  addr.row = (uint16_t)row;
  addr.col = (uint16_t)col;
  if (!valid_addr(e, addr)) return 0;
  *out = addr;
  return 1;
}

static double numeric_from_index_with_cycle(const DvcEngine *e, uint32_t idx, const EvalCtx *ctx) {
  if (ctx && ctx->non_iterative_cycle && ctx->cycle_nodes && ctx->cycle_nodes[idx]) {
    if (ctx->prior_types && ctx->prior_numbers) {
      if (ctx->prior_types[idx] == DVC_VALUE_NUMBER || ctx->prior_types[idx] == DVC_VALUE_BOOL) {
        return ctx->prior_numbers[idx];
      }
    }
    return 0.0;
  }
  const CellComputed *cc = &e->computed[idx];
  if (cc->value.type == DVC_VALUE_NUMBER) return cc->value.number;
  if (cc->value.type == DVC_VALUE_BOOL) return cc->value.bool_val ? 1.0 : 0.0;
  return 0.0;
}

static int eval_numeric_expr_depth(const DvcEngine *e, const char *expr, const EvalCtx *ctx, double *out, int depth);

static int parse_function_body(const char *expr, const char *fn_name, const char **body_start, size_t *body_len) {
  if (!expr || !fn_name || !body_start || !body_len) return 0;
  const char *s = skip_ws(expr);
  while (*s == '=') s = skip_ws(s + 1);
  size_t fn_len = strlen(fn_name);
  if (!starts_with_ci(s, fn_name)) return 0;
  s += fn_len;
  s = skip_ws(s);
  if (*s != '(') return 0;
  int depth = 0;
  const char *p = s;
  while (*p) {
    if (*p == '(') depth++;
    else if (*p == ')') {
      depth--;
      if (depth == 0) {
        *body_start = s + 1;
        *body_len = (size_t)(p - (s + 1));
        return 1;
      }
    }
    ++p;
  }
  return 0;
}

static int eval_numeric_expr_depth(const DvcEngine *e, const char *expr, const EvalCtx *ctx, double *out, int depth) {
  if (!e || !expr || !out || depth > 16) return 0;
  char *work = dup_trim(expr);
  if (!work) return 0;
  char *s = work;
  while (*s == '=') {
    ++s;
    while (isspace((unsigned char)*s)) ++s;
  }
  if (*s == '\0') {
    free(work);
    return 0;
  }

  if (ctx && ctx->has_var && ctx->var_name && cmp_ci(s, ctx->var_name) == 0) {
    *out = ctx->var_value;
    free(work);
    return 1;
  }

  char *end = NULL;
  double n = strtod(s, &end);
  if (end != s && *skip_ws(end) == '\0') {
    *out = n;
    free(work);
    return 1;
  }

  RefToken rt;
  if (parse_ref_token(s, &rt) && s[rt.used_chars] == '\0') {
    DvcCellAddr a = {rt.col, rt.row};
    if (valid_addr(e, a)) {
      *out = numeric_from_index_with_cycle(e, addr_to_index(e, a), ctx);
      free(work);
      return 1;
    }
  }

  if (starts_with_ci(s, "INDIRECT(")) {
    const char *body_start = NULL;
    size_t body_len = 0;
    if (!parse_function_body(s, "INDIRECT", &body_start, &body_len)) {
      free(work);
      return 0;
    }
    char body[512];
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    char args[3][256];
    int argc = split_top_level_args(body, args, 3);
    if (argc >= 1) {
      int a1_mode = 1;
      if (argc >= 2 && (cmp_ci(args[1], "FALSE") == 0 || strcmp(args[1], "0") == 0)) a1_mode = 0;
      const char *raw = args[0];
      char ref_text[256];
      size_t raw_len = strlen(raw);
      if (raw_len >= 2 && raw[0] == '"' && raw[raw_len - 1] == '"') {
        size_t inner_len = raw_len - 2;
        if (inner_len >= sizeof(ref_text)) inner_len = sizeof(ref_text) - 1;
        memcpy(ref_text, raw + 1, inner_len);
        ref_text[inner_len] = '\0';
      } else {
        snprintf(ref_text, sizeof(ref_text), "%s", raw);
      }
      DvcCellAddr a;
      int ok = a1_mode ? parse_a1_addr(e, ref_text, (uint32_t)strlen(ref_text), &a) : parse_r1c1_absolute(e, ref_text, &a);
      if (ok) {
        *out = numeric_from_index_with_cycle(e, addr_to_index(e, a), ctx);
        free(work);
        return 1;
      }
    }
    free(work);
    return 0;
  }

  if (starts_with_ci(s, "OFFSET(")) {
    const char *body_start = NULL;
    size_t body_len = 0;
    if (!parse_function_body(s, "OFFSET", &body_start, &body_len)) {
      free(work);
      return 0;
    }
    char body[512];
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    char args[5][256];
    int argc = split_top_level_args(body, args, 5);
    if (argc >= 3) {
      RefToken base_ref;
      if (parse_ref_token(args[0], &base_ref) && args[0][base_ref.used_chars] == '\0') {
        DvcCellAddr base = {base_ref.col, base_ref.row};
        if (valid_addr(e, base)) {
          double dr = 0.0, dc = 0.0;
          if (eval_numeric_expr_depth(e, args[1], ctx, &dr, depth + 1) && eval_numeric_expr_depth(e, args[2], ctx, &dc, depth + 1)) {
            long tr = (long)base.row + (long)llround(dr);
            long tc = (long)base.col + (long)llround(dc);
            if (tr >= 1 && tc >= 1) {
              DvcCellAddr target = {(uint16_t)tc, (uint16_t)tr};
              if (valid_addr(e, target)) {
                *out = numeric_from_index_with_cycle(e, addr_to_index(e, target), ctx);
                free(work);
                return 1;
              }
            }
          }
        }
      }
    }
    free(work);
    return 0;
  }

  const char *ops = "+-*/";
  for (const char *op = ops; *op; ++op) {
    int depth_paren = 0;
    int in_string = 0;
    for (char *p = s; *p; ++p) {
      if (*p == '"') in_string = !in_string;
      if (in_string) continue;
      if (*p == '(') depth_paren++;
      else if (*p == ')') depth_paren--;
      else if (depth_paren == 0 && *p == *op) {
        if ((p == s) && (*op == '+' || *op == '-')) continue;
        char left[256];
        char right[256];
        size_t llen = (size_t)(p - s);
        size_t rlen = strlen(p + 1);
        if (llen >= sizeof(left) || rlen >= sizeof(right)) {
          free(work);
          return 0;
        }
        memcpy(left, s, llen);
        left[llen] = '\0';
        memcpy(right, p + 1, rlen + 1);
        trim_inplace(left);
        trim_inplace(right);
        double lv = 0.0;
        double rv = 0.0;
        if (!eval_numeric_expr_depth(e, left, ctx, &lv, depth + 1) ||
            !eval_numeric_expr_depth(e, right, ctx, &rv, depth + 1)) {
          free(work);
          return 0;
        }
        if (*op == '+') *out = lv + rv;
        else if (*op == '-') *out = lv - rv;
        else if (*op == '*') *out = lv * rv;
        else {
          if (rv == 0.0) {
            free(work);
            return 0;
          }
          *out = lv / rv;
        }
        free(work);
        return 1;
      }
    }
  }

  free(work);
  return 0;
}

static int eval_numeric_expr(const DvcEngine *e, const char *expr, const EvalCtx *ctx, double *out) {
  return eval_numeric_expr_depth(e, expr, ctx, out, 0);
}

static int parse_ref_or_range_expr(const char *expr, DvcCellAddr *start, DvcCellAddr *end, int *is_range) {
  if (!expr || !start || !end || !is_range) return 0;
  *is_range = 0;
  char *trimmed = dup_trim(expr);
  if (!trimmed) return 0;
  RefToken a;
  if (!parse_ref_token(trimmed, &a)) {
    free(trimmed);
    return 0;
  }
  char *p = trimmed + a.used_chars;
  RefToken b = a;
  if (*p != '\0') {
    size_t sep_len = 0;
    if (*p == ':') sep_len = 1;
    else if (strncmp(p, "...", 3) == 0) sep_len = 3;
    if (sep_len == 0 || !parse_ref_token(p + sep_len, &b) || p[sep_len + b.used_chars] != '\0') {
      free(trimmed);
      return 0;
    }
    *is_range = 1;
  }
  start->col = a.col;
  start->row = a.row;
  end->col = b.col;
  end->row = b.row;
  free(trimmed);
  return 1;
}

static int eval_comparison_or_numeric(const DvcEngine *e, const char *expr, const EvalCtx *ctx, double *out) {
  if (eval_numeric_expr(e, expr, ctx, out)) return 1;
  if (!expr || !out) return 0;
  char *work = dup_trim(expr);
  if (!work) return 0;
  char *s = work;
  int depth_paren = 0;
  int in_string = 0;
  char *op_pos = NULL;
  int op_kind = 0;
  for (char *p = s; *p; ++p) {
    if (*p == '"') in_string = !in_string;
    if (in_string) continue;
    if (*p == '(') depth_paren++;
    else if (*p == ')') depth_paren--;
    else if (depth_paren == 0) {
      if ((*p == '<' || *p == '>') && p[1] == '=') { op_pos = p; op_kind = (*p == '<') ? 1 : 2; break; }
      if (*p == '<' && p[1] == '>') { op_pos = p; op_kind = 3; break; }
      if (*p == '=') { op_pos = p; op_kind = 4; break; }
      if (*p == '<') { op_pos = p; op_kind = 5; break; }
      if (*p == '>') { op_pos = p; op_kind = 6; break; }
    }
  }
  if (!op_pos) {
    free(work);
    return 0;
  }

  size_t op_len = (op_kind <= 3) ? 2u : 1u;
  char left[256];
  char right[256];
  size_t left_len = (size_t)(op_pos - s);
  size_t right_len = strlen(op_pos + op_len);
  if (left_len >= sizeof(left) || right_len >= sizeof(right)) {
    free(work);
    return 0;
  }
  memcpy(left, s, left_len);
  left[left_len] = '\0';
  memcpy(right, op_pos + op_len, right_len + 1);
  trim_inplace(left);
  trim_inplace(right);

  double lv = 0.0;
  double rv = 0.0;
  if (!eval_numeric_expr(e, left, ctx, &lv) || !eval_numeric_expr(e, right, ctx, &rv)) {
    free(work);
    return 0;
  }
  int truth = 0;
  if (op_kind == 1) truth = (lv <= rv);
  else if (op_kind == 2) truth = (lv >= rv);
  else if (op_kind == 3) truth = (lv != rv);
  else if (op_kind == 4) truth = (lv == rv);
  else if (op_kind == 5) truth = (lv < rv);
  else truth = (lv > rv);
  *out = truth ? 1.0 : 0.0;
  free(work);
  return 1;
}

static int eval_simple_arg_value(const DvcEngine *e, const char *arg, const EvalCtx *ctx, SimpleArgValue *out) {
  if (!e || !arg || !out) return 0;
  memset(out, 0, sizeof(*out));
  out->type = DVC_VALUE_BLANK;
  out->error_kind = DVC_CELL_ERR_NULL;

  const char *start = NULL;
  size_t len = 0;
  trim_span(arg, &start, &len);
  if (!start || len == 0) {
    out->type = DVC_VALUE_BLANK;
    return 1;
  }

  if (len >= 2 && start[0] == '"' && start[len - 1] == '"') {
    size_t inner_len = len - 2;
    if (inner_len >= sizeof(out->text)) inner_len = sizeof(out->text) - 1;
    memcpy(out->text, start + 1, inner_len);
    out->text[inner_len] = '\0';
    out->type = DVC_VALUE_TEXT;
    return 1;
  }

  char tmp[256];
  if (len >= sizeof(tmp)) len = sizeof(tmp) - 1;
  memcpy(tmp, start, len);
  tmp[len] = '\0';
  trim_inplace(tmp);

  if (cmp_ci(tmp, "TRUE") == 0) {
    out->type = DVC_VALUE_BOOL;
    out->bool_val = 1;
    out->number = 1.0;
    return 1;
  }
  if (cmp_ci(tmp, "FALSE") == 0) {
    out->type = DVC_VALUE_BOOL;
    out->bool_val = 0;
    out->number = 0.0;
    return 1;
  }
  if (starts_with_ci(tmp, "NA(")) {
    out->type = DVC_VALUE_ERROR;
    out->error_kind = DVC_CELL_ERR_NA;
    return 1;
  }
  if (starts_with_ci(tmp, "ERROR(")) {
    out->type = DVC_VALUE_ERROR;
    out->error_kind = DVC_CELL_ERR_VALUE;
    return 1;
  }

  DvcCellAddr rs, re;
  int is_range = 0;
  if (parse_ref_or_range_expr(tmp, &rs, &re, &is_range) && !is_range && valid_addr(e, rs)) {
    uint32_t idx = addr_to_index(e, rs);
    if (ctx && ctx->non_iterative_cycle && ctx->cycle_nodes && ctx->cycle_nodes[idx] && ctx->prior_types && ctx->prior_numbers) {
      if (ctx->prior_types[idx] == DVC_VALUE_NUMBER || ctx->prior_types[idx] == DVC_VALUE_BOOL) {
        out->type = DVC_VALUE_NUMBER;
        out->number = ctx->prior_numbers[idx];
        return 1;
      }
    }
    const CellComputed *cc = &e->computed[idx];
    out->type = cc->value.type;
    out->number = cc->value.number;
    out->bool_val = cc->value.bool_val;
    out->error_kind = cc->value.error_kind;
    if (cc->value.type == DVC_VALUE_BOOL) {
      out->number = cc->value.bool_val ? 1.0 : 0.0;
    }
    if (cc->value.type == DVC_VALUE_TEXT && cc->text) {
      snprintf(out->text, sizeof(out->text), "%s", cc->text);
    }
    return 1;
  }

  double n = 0.0;
  if (eval_comparison_or_numeric(e, tmp, ctx, &n)) {
    out->type = DVC_VALUE_NUMBER;
    out->number = n;
    return 1;
  }
  return 0;
}

static int append_simple_arg_text(char *dst, size_t cap, size_t *len, const SimpleArgValue *v) {
  if (!dst || !len || !v) return 0;
  char local[128];
  const char *src = "";
  if (v->type == DVC_VALUE_TEXT) {
    src = v->text;
  } else if (v->type == DVC_VALUE_NUMBER) {
    snprintf(local, sizeof(local), "%.17g", v->number);
    src = local;
  } else if (v->type == DVC_VALUE_BOOL) {
    src = v->bool_val ? "TRUE" : "FALSE";
  } else if (v->type == DVC_VALUE_BLANK) {
    src = "";
  } else if (v->type == DVC_VALUE_ERROR) {
    if (v->error_kind == DVC_CELL_ERR_NA) src = "#N/A";
    else src = "#ERROR!";
  }
  size_t src_len = strlen(src);
  if (*len + src_len + 1 > cap) return 0;
  memcpy(dst + *len, src, src_len);
  *len += src_len;
  dst[*len] = '\0';
  return 1;
}

static void set_computed_blank(CellComputed *cc, uint64_t epoch);
static void set_computed_number(CellComputed *cc, double v, uint64_t epoch);
static void set_computed_text(CellComputed *cc, const char *txt, uint64_t epoch);
static void set_computed_bool(CellComputed *cc, int b, uint64_t epoch);
static void set_computed_error(CellComputed *cc, DvcCellErrorKind ek, const char *msg, uint64_t epoch);

static int error_type_code_for_kind(DvcCellErrorKind kind) {
  if (kind == DVC_CELL_ERR_NULL) return 1;
  if (kind == DVC_CELL_ERR_DIV_ZERO) return 2;
  if (kind == DVC_CELL_ERR_VALUE) return 3;
  if (kind == DVC_CELL_ERR_REF) return 4;
  if (kind == DVC_CELL_ERR_NAME || kind == DVC_CELL_ERR_UNKNOWN_NAME) return 5;
  if (kind == DVC_CELL_ERR_NUM) return 6;
  if (kind == DVC_CELL_ERR_NA) return 7;
  return 8;
}

static int try_eval_required_fn(
    DvcEngine *e,
    CellComputed *cc,
    DvcCellAddr self,
    const char *formula,
    const EvalCtx *ctx) {
  if (!e || !cc || !formula) return 0;
  (void)self;

  const char *body_start = NULL;
  size_t body_len = 0;
  char body[768];
  char args[16][256];
  int argc = 0;

  if (parse_function_body(formula, "CONCAT", &body_start, &body_len)) {
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    argc = split_top_level_args(body, args, 16);
    char out_text[1024];
    size_t out_len = 0;
    out_text[0] = '\0';
    for (int i = 0; i < argc; ++i) {
      SimpleArgValue v;
      if (!eval_simple_arg_value(e, args[i], ctx, &v)) return 1;
      if (!append_simple_arg_text(out_text, sizeof(out_text), &out_len, &v)) return 1;
    }
    set_computed_text(cc, out_text, e->committed_epoch);
    return 1;
  }

  if (parse_function_body(formula, "NA", &body_start, &body_len)) {
    set_computed_error(cc, DVC_CELL_ERR_NA, "#N/A", e->committed_epoch);
    return 1;
  }
  if (parse_function_body(formula, "ERROR", &body_start, &body_len)) {
    set_computed_error(cc, DVC_CELL_ERR_VALUE, "ERROR", e->committed_epoch);
    return 1;
  }

  if (parse_function_body(formula, "SUM", &body_start, &body_len) ||
      parse_function_body(formula, "MIN", &body_start, &body_len) ||
      parse_function_body(formula, "MAX", &body_start, &body_len) ||
      parse_function_body(formula, "AVERAGE", &body_start, &body_len) ||
      parse_function_body(formula, "COUNT", &body_start, &body_len)) {
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    argc = split_top_level_args(body, args, 16);
    double agg = 0.0;
    double min_v = 0.0;
    double max_v = 0.0;
    int count = 0;
    int has_value = 0;
    for (int i = 0; i < argc; ++i) {
      DvcCellAddr rs, re;
      int is_range = 0;
      if (parse_ref_or_range_expr(args[i], &rs, &re, &is_range) && valid_addr(e, rs) && valid_addr(e, re)) {
        uint16_t sr = rs.row < re.row ? rs.row : re.row;
        uint16_t er = rs.row > re.row ? rs.row : re.row;
        uint16_t sc = rs.col < re.col ? rs.col : re.col;
        uint16_t ec = rs.col > re.col ? rs.col : re.col;
        for (uint16_t rr = sr; rr <= er; ++rr) {
          for (uint16_t ccx = sc; ccx <= ec; ++ccx) {
            SimpleArgValue v;
            memset(&v, 0, sizeof(v));
            DvcCellAddr a = {ccx, rr};
            uint32_t ridx = addr_to_index(e, a);
            const CellComputed *src = &e->computed[ridx];
            v.type = src->value.type;
            v.number = src->value.number;
            v.bool_val = src->value.bool_val;
            v.error_kind = src->value.error_kind;
            if (src->value.type == DVC_VALUE_TEXT && src->text) {
              snprintf(v.text, sizeof(v.text), "%s", src->text);
            }
            if (starts_with_ci(formula, "COUNT(")) {
              if (v.type == DVC_VALUE_NUMBER || v.type == DVC_VALUE_BOOL) count++;
            } else if (v.type == DVC_VALUE_NUMBER || v.type == DVC_VALUE_BOOL) {
              double nv = (v.type == DVC_VALUE_BOOL) ? (v.bool_val ? 1.0 : 0.0) : v.number;
              agg += nv;
              if (!has_value) {
                min_v = max_v = nv;
                has_value = 1;
              } else {
                if (nv < min_v) min_v = nv;
                if (nv > max_v) max_v = nv;
              }
              count++;
            }
          }
        }
        continue;
      }

      double v = 0.0;
      if (!eval_comparison_or_numeric(e, args[i], ctx, &v)) continue;
      if (starts_with_ci(formula, "COUNT(")) {
        count++;
      } else {
        agg += v;
        if (!has_value) {
          min_v = max_v = v;
          has_value = 1;
        } else {
          if (v < min_v) min_v = v;
          if (v > max_v) max_v = v;
        }
        count++;
      }
    }
    if (starts_with_ci(formula, "SUM(")) set_computed_number(cc, agg, e->committed_epoch);
    else if (starts_with_ci(formula, "MIN(")) set_computed_number(cc, has_value ? min_v : 0.0, e->committed_epoch);
    else if (starts_with_ci(formula, "MAX(")) set_computed_number(cc, has_value ? max_v : 0.0, e->committed_epoch);
    else if (starts_with_ci(formula, "AVERAGE(")) {
      if (count == 0) set_computed_error(cc, DVC_CELL_ERR_DIV_ZERO, "AVERAGE empty", e->committed_epoch);
      else set_computed_number(cc, agg / (double)count, e->committed_epoch);
    } else set_computed_number(cc, (double)count, e->committed_epoch);
    return 1;
  }

  if (parse_function_body(formula, "IF", &body_start, &body_len) ||
      parse_function_body(formula, "IFERROR", &body_start, &body_len) ||
      parse_function_body(formula, "IFNA", &body_start, &body_len)) {
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    argc = split_top_level_args(body, args, 16);
    if (starts_with_ci(formula, "IF(") && argc >= 2) {
      double cond = 0.0;
      if (!eval_comparison_or_numeric(e, args[0], ctx, &cond)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "IF condition", e->committed_epoch);
        return 1;
      }
      const char *selected = (fabs(cond) > 0.0) ? args[1] : ((argc >= 3) ? args[2] : "");
      SimpleArgValue v;
      if (!eval_simple_arg_value(e, selected, ctx, &v)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "IF branch", e->committed_epoch);
        return 1;
      }
      if (v.type == DVC_VALUE_TEXT) set_computed_text(cc, v.text, e->committed_epoch);
      else if (v.type == DVC_VALUE_BOOL) set_computed_bool(cc, v.bool_val, e->committed_epoch);
      else if (v.type == DVC_VALUE_ERROR) set_computed_error(cc, v.error_kind, "IF error", e->committed_epoch);
      else if (v.type == DVC_VALUE_NUMBER) set_computed_number(cc, v.number, e->committed_epoch);
      else set_computed_blank(cc, e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "IFERROR(") && argc >= 2) {
      SimpleArgValue first;
      if (!eval_simple_arg_value(e, args[0], ctx, &first) || first.type == DVC_VALUE_ERROR) {
        SimpleArgValue fallback;
        if (eval_simple_arg_value(e, args[1], ctx, &fallback)) {
          if (fallback.type == DVC_VALUE_TEXT) set_computed_text(cc, fallback.text, e->committed_epoch);
          else if (fallback.type == DVC_VALUE_BOOL) set_computed_bool(cc, fallback.bool_val, e->committed_epoch);
          else if (fallback.type == DVC_VALUE_NUMBER) set_computed_number(cc, fallback.number, e->committed_epoch);
          else if (fallback.type == DVC_VALUE_ERROR) set_computed_error(cc, fallback.error_kind, "IFERROR fallback", e->committed_epoch);
          else set_computed_blank(cc, e->committed_epoch);
          return 1;
        }
      } else {
        if (first.type == DVC_VALUE_TEXT) set_computed_text(cc, first.text, e->committed_epoch);
        else if (first.type == DVC_VALUE_BOOL) set_computed_bool(cc, first.bool_val, e->committed_epoch);
        else if (first.type == DVC_VALUE_NUMBER) set_computed_number(cc, first.number, e->committed_epoch);
        else set_computed_blank(cc, e->committed_epoch);
        return 1;
      }
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "IFERROR", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "IFNA(") && argc >= 2) {
      SimpleArgValue first;
      if (eval_simple_arg_value(e, args[0], ctx, &first) && !(first.type == DVC_VALUE_ERROR && first.error_kind == DVC_CELL_ERR_NA)) {
        if (first.type == DVC_VALUE_TEXT) set_computed_text(cc, first.text, e->committed_epoch);
        else if (first.type == DVC_VALUE_BOOL) set_computed_bool(cc, first.bool_val, e->committed_epoch);
        else if (first.type == DVC_VALUE_NUMBER) set_computed_number(cc, first.number, e->committed_epoch);
        else if (first.type == DVC_VALUE_ERROR) set_computed_error(cc, first.error_kind, "IFNA value", e->committed_epoch);
        else set_computed_blank(cc, e->committed_epoch);
        return 1;
      }
      SimpleArgValue fallback;
      if (eval_simple_arg_value(e, args[1], ctx, &fallback)) {
        if (fallback.type == DVC_VALUE_TEXT) set_computed_text(cc, fallback.text, e->committed_epoch);
        else if (fallback.type == DVC_VALUE_BOOL) set_computed_bool(cc, fallback.bool_val, e->committed_epoch);
        else if (fallback.type == DVC_VALUE_NUMBER) set_computed_number(cc, fallback.number, e->committed_epoch);
        else if (fallback.type == DVC_VALUE_ERROR) set_computed_error(cc, fallback.error_kind, "IFNA fallback", e->committed_epoch);
        else set_computed_blank(cc, e->committed_epoch);
        return 1;
      }
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "IFNA", e->committed_epoch);
      return 1;
    }
    set_computed_error(cc, DVC_CELL_ERR_VALUE, "IF family args", e->committed_epoch);
    return 1;
  }

  if (parse_function_body(formula, "AND", &body_start, &body_len) ||
      parse_function_body(formula, "OR", &body_start, &body_len) ||
      parse_function_body(formula, "NOT", &body_start, &body_len) ||
      parse_function_body(formula, "ISERROR", &body_start, &body_len) ||
      parse_function_body(formula, "ISNA", &body_start, &body_len) ||
      parse_function_body(formula, "ISBLANK", &body_start, &body_len) ||
      parse_function_body(formula, "ISTEXT", &body_start, &body_len) ||
      parse_function_body(formula, "ISNUMBER", &body_start, &body_len) ||
      parse_function_body(formula, "ISLOGICAL", &body_start, &body_len) ||
      parse_function_body(formula, "ERROR.TYPE", &body_start, &body_len)) {
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    argc = split_top_level_args(body, args, 16);
    if (starts_with_ci(formula, "AND(") || starts_with_ci(formula, "OR(")) {
      int truth = starts_with_ci(formula, "AND(") ? 1 : 0;
      for (int i = 0; i < argc; ++i) {
        double v = 0.0;
        if (!eval_comparison_or_numeric(e, args[i], ctx, &v)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "logical arg", e->committed_epoch);
          return 1;
        }
        int b = (fabs(v) > 0.0) ? 1 : 0;
        if (starts_with_ci(formula, "AND(")) truth = truth && b;
        else truth = truth || b;
      }
      set_computed_bool(cc, truth, e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "NOT(")) {
      if (argc != 1) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "NOT arity", e->committed_epoch);
        return 1;
      }
      double v = 0.0;
      if (!eval_comparison_or_numeric(e, args[0], ctx, &v)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "NOT arg", e->committed_epoch);
        return 1;
      }
      set_computed_bool(cc, fabs(v) <= 0.0, e->committed_epoch);
      return 1;
    }
    if (argc < 1) {
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "predicate arg", e->committed_epoch);
      return 1;
    }
    SimpleArgValue v;
    if (!eval_simple_arg_value(e, args[0], ctx, &v)) {
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "predicate eval", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "ISERROR(")) set_computed_bool(cc, v.type == DVC_VALUE_ERROR, e->committed_epoch);
    else if (starts_with_ci(formula, "ISNA(")) set_computed_bool(cc, v.type == DVC_VALUE_ERROR && v.error_kind == DVC_CELL_ERR_NA, e->committed_epoch);
    else if (starts_with_ci(formula, "ISBLANK(")) set_computed_bool(cc, v.type == DVC_VALUE_BLANK, e->committed_epoch);
    else if (starts_with_ci(formula, "ISTEXT(")) set_computed_bool(cc, v.type == DVC_VALUE_TEXT, e->committed_epoch);
    else if (starts_with_ci(formula, "ISNUMBER(")) set_computed_bool(cc, v.type == DVC_VALUE_NUMBER, e->committed_epoch);
    else if (starts_with_ci(formula, "ISLOGICAL(")) set_computed_bool(cc, v.type == DVC_VALUE_BOOL, e->committed_epoch);
    else if (starts_with_ci(formula, "ERROR.TYPE(")) {
      if (v.type != DVC_VALUE_ERROR) set_computed_error(cc, DVC_CELL_ERR_NA, "ERROR.TYPE non-error", e->committed_epoch);
      else set_computed_number(cc, (double)error_type_code_for_kind(v.error_kind), e->committed_epoch);
    }
    return 1;
  }

  if (parse_function_body(formula, "ABS", &body_start, &body_len) ||
      parse_function_body(formula, "INT", &body_start, &body_len) ||
      parse_function_body(formula, "ROUND", &body_start, &body_len) ||
      parse_function_body(formula, "SIGN", &body_start, &body_len) ||
      parse_function_body(formula, "SQRT", &body_start, &body_len) ||
      parse_function_body(formula, "EXP", &body_start, &body_len) ||
      parse_function_body(formula, "LN", &body_start, &body_len) ||
      parse_function_body(formula, "LOG10", &body_start, &body_len) ||
      parse_function_body(formula, "SIN", &body_start, &body_len) ||
      parse_function_body(formula, "COS", &body_start, &body_len) ||
      parse_function_body(formula, "TAN", &body_start, &body_len) ||
      parse_function_body(formula, "ATN", &body_start, &body_len) ||
      parse_function_body(formula, "PI", &body_start, &body_len) ||
      parse_function_body(formula, "NPV", &body_start, &body_len) ||
      parse_function_body(formula, "PV", &body_start, &body_len) ||
      parse_function_body(formula, "FV", &body_start, &body_len) ||
      parse_function_body(formula, "PMT", &body_start, &body_len) ||
      parse_function_body(formula, "LOOKUP", &body_start, &body_len) ||
      parse_function_body(formula, "LEN", &body_start, &body_len) ||
      parse_function_body(formula, "ROW", &body_start, &body_len) ||
      parse_function_body(formula, "COLUMN", &body_start, &body_len)) {
    if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
    memcpy(body, body_start, body_len);
    body[body_len] = '\0';
    argc = split_top_level_args(body, args, 16);
    if (argc < 0) {
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "function args", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "PI(")) {
      set_computed_number(cc, 3.14159265358979323846, e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "ROW(") || starts_with_ci(formula, "COLUMN(")) {
      if (argc == 0 || (argc == 1 && args[0][0] == '\0')) {
        set_computed_number(cc, starts_with_ci(formula, "ROW(") ? (double)self.row : (double)self.col, e->committed_epoch);
        return 1;
      }
      DvcCellAddr rs, re;
      int is_range = 0;
      if (parse_ref_or_range_expr(args[0], &rs, &re, &is_range)) {
        set_computed_number(cc, starts_with_ci(formula, "ROW(") ? (double)rs.row : (double)rs.col, e->committed_epoch);
      } else {
        set_computed_error(cc, DVC_CELL_ERR_REF, "ROW/COLUMN arg", e->committed_epoch);
      }
      return 1;
    }
    if (starts_with_ci(formula, "LEN(")) {
      if (argc < 1) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "LEN arg", e->committed_epoch);
        return 1;
      }
      SimpleArgValue v;
      if (!eval_simple_arg_value(e, args[0], ctx, &v)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "LEN eval", e->committed_epoch);
        return 1;
      }
      if (v.type == DVC_VALUE_TEXT) set_computed_number(cc, (double)strlen(v.text), e->committed_epoch);
      else if (v.type == DVC_VALUE_BOOL) set_computed_number(cc, v.bool_val ? 4.0 : 5.0, e->committed_epoch);
      else if (v.type == DVC_VALUE_NUMBER) {
        char num[64];
        snprintf(num, sizeof(num), "%.17g", v.number);
        set_computed_number(cc, (double)strlen(num), e->committed_epoch);
      } else if (v.type == DVC_VALUE_BLANK) set_computed_number(cc, 0.0, e->committed_epoch);
      else set_computed_error(cc, DVC_CELL_ERR_VALUE, "LEN error", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "LOOKUP(")) {
      if (argc < 2) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "LOOKUP args", e->committed_epoch);
        return 1;
      }
      double needle = 0.0;
      if (!eval_comparison_or_numeric(e, args[0], ctx, &needle)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "LOOKUP needle", e->committed_epoch);
        return 1;
      }
      DvcCellAddr ls, le, rs, re;
      int li = 0;
      int ri = 0;
      if (!parse_ref_or_range_expr(args[1], &ls, &le, &li) || !valid_addr(e, ls) || !valid_addr(e, le)) {
        set_computed_error(cc, DVC_CELL_ERR_REF, "LOOKUP range", e->committed_epoch);
        return 1;
      }
      if (argc >= 3 && (!parse_ref_or_range_expr(args[2], &rs, &re, &ri) || !valid_addr(e, rs) || !valid_addr(e, re))) {
        set_computed_error(cc, DVC_CELL_ERR_REF, "LOOKUP result range", e->committed_epoch);
        return 1;
      }
      uint16_t lsr = ls.row < le.row ? ls.row : le.row;
      uint16_t ler = ls.row > le.row ? ls.row : le.row;
      uint16_t lsc = ls.col < le.col ? ls.col : le.col;
      uint16_t lec = ls.col > le.col ? ls.col : le.col;
      int best_found = 0;
      int best_idx = 0;
      int idx = 0;
      for (uint16_t rr = lsr; rr <= ler; ++rr) {
        for (uint16_t ccx = lsc; ccx <= lec; ++ccx) {
          double v = numeric_from_index_with_cycle(e, addr_to_index(e, (DvcCellAddr){ccx, rr}), ctx);
          if (v <= needle) {
            best_found = 1;
            best_idx = idx;
          }
          idx++;
        }
      }
      if (!best_found) {
        set_computed_error(cc, DVC_CELL_ERR_NA, "LOOKUP no match", e->committed_epoch);
        return 1;
      }
      if (argc < 3) {
        idx = 0;
        for (uint16_t rr = lsr; rr <= ler; ++rr) {
          for (uint16_t ccx = lsc; ccx <= lec; ++ccx) {
            if (idx == best_idx) {
              set_computed_number(cc, numeric_from_index_with_cycle(e, addr_to_index(e, (DvcCellAddr){ccx, rr}), ctx), e->committed_epoch);
              return 1;
            }
            idx++;
          }
        }
      } else {
        uint16_t rsr = rs.row < re.row ? rs.row : re.row;
        uint16_t rer = rs.row > re.row ? rs.row : re.row;
        uint16_t rsc = rs.col < re.col ? rs.col : re.col;
        uint16_t rec = rs.col > re.col ? rs.col : re.col;
        idx = 0;
        for (uint16_t rr = rsr; rr <= rer; ++rr) {
          for (uint16_t ccx = rsc; ccx <= rec; ++ccx) {
            if (idx == best_idx) {
              set_computed_number(cc, numeric_from_index_with_cycle(e, addr_to_index(e, (DvcCellAddr){ccx, rr}), ctx), e->committed_epoch);
              return 1;
            }
            idx++;
          }
        }
      }
      set_computed_error(cc, DVC_CELL_ERR_REF, "LOOKUP index", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "NPV(")) {
      if (argc < 2) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "NPV args", e->committed_epoch);
        return 1;
      }
      double rate = 0.0;
      if (!eval_comparison_or_numeric(e, args[0], ctx, &rate)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "NPV rate", e->committed_epoch);
        return 1;
      }
      double sum = 0.0;
      for (int i = 1; i < argc; ++i) {
        double value = 0.0;
        if (!eval_comparison_or_numeric(e, args[i], ctx, &value)) continue;
        sum += value / pow(1.0 + rate, (double)i);
      }
      set_computed_number(cc, sum, e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "PV(") || starts_with_ci(formula, "FV(") || starts_with_ci(formula, "PMT(")) {
      if (argc < 3) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "financial args", e->committed_epoch);
        return 1;
      }
      double rate = 0.0, nper = 0.0, a2 = 0.0, a3 = 0.0, a4 = 0.0;
      if (!eval_comparison_or_numeric(e, args[0], ctx, &rate) ||
          !eval_comparison_or_numeric(e, args[1], ctx, &nper) ||
          !eval_comparison_or_numeric(e, args[2], ctx, &a2)) {
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "financial eval", e->committed_epoch);
        return 1;
      }
      if (argc >= 4) eval_comparison_or_numeric(e, args[3], ctx, &a3);
      if (argc >= 5) eval_comparison_or_numeric(e, args[4], ctx, &a4);
      if (starts_with_ci(formula, "PV(")) {
        double pmt = a2;
        double fv = a3;
        double typ = a4;
        double out_n = 0.0;
        if (fabs(rate) < 1e-12) out_n = -(fv + pmt * nper);
        else {
          double t = pow(1.0 + rate, nper);
          out_n = -(fv + pmt * (1.0 + rate * typ) * (t - 1.0) / rate) / t;
        }
        set_computed_number(cc, out_n, e->committed_epoch);
      } else if (starts_with_ci(formula, "FV(")) {
        double pmt = a2;
        double pv = a3;
        double typ = a4;
        double out_n = 0.0;
        if (fabs(rate) < 1e-12) out_n = -(pv + pmt * nper);
        else {
          double t = pow(1.0 + rate, nper);
          out_n = -(pv * t + pmt * (1.0 + rate * typ) * (t - 1.0) / rate);
        }
        set_computed_number(cc, out_n, e->committed_epoch);
      } else {
        double pv = a2;
        double fv = a3;
        double typ = a4;
        double out_n = 0.0;
        if (fabs(nper) < 1e-12) {
          set_computed_error(cc, DVC_CELL_ERR_DIV_ZERO, "PMT nper", e->committed_epoch);
          return 1;
        }
        if (fabs(rate) < 1e-12) out_n = -(pv + fv) / nper;
        else {
          double t = pow(1.0 + rate, nper);
          double denom = (1.0 + rate * typ) * (t - 1.0);
          if (fabs(denom) < 1e-12) {
            set_computed_error(cc, DVC_CELL_ERR_DIV_ZERO, "PMT denom", e->committed_epoch);
            return 1;
          }
          out_n = -(rate * (fv + pv * t)) / denom;
        }
        set_computed_number(cc, out_n, e->committed_epoch);
      }
      return 1;
    }

    double v0 = 0.0;
    if (argc >= 1 && !eval_comparison_or_numeric(e, args[0], ctx, &v0)) {
      set_computed_error(cc, DVC_CELL_ERR_VALUE, "function arg", e->committed_epoch);
      return 1;
    }
    if (starts_with_ci(formula, "ABS(")) set_computed_number(cc, fabs(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "INT(")) set_computed_number(cc, floor(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "SIGN(")) set_computed_number(cc, (v0 > 0.0) ? 1.0 : ((v0 < 0.0) ? -1.0 : 0.0), e->committed_epoch);
    else if (starts_with_ci(formula, "SQRT(")) {
      if (v0 < 0.0) set_computed_error(cc, DVC_CELL_ERR_NUM, "SQRT negative", e->committed_epoch);
      else set_computed_number(cc, sqrt(v0), e->committed_epoch);
    } else if (starts_with_ci(formula, "EXP(")) set_computed_number(cc, exp(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "LN(")) {
      if (v0 <= 0.0) set_computed_error(cc, DVC_CELL_ERR_NUM, "LN domain", e->committed_epoch);
      else set_computed_number(cc, log(v0), e->committed_epoch);
    } else if (starts_with_ci(formula, "LOG10(")) {
      if (v0 <= 0.0) set_computed_error(cc, DVC_CELL_ERR_NUM, "LOG10 domain", e->committed_epoch);
      else set_computed_number(cc, log10(v0), e->committed_epoch);
    } else if (starts_with_ci(formula, "SIN(")) set_computed_number(cc, sin(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "COS(")) set_computed_number(cc, cos(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "TAN(")) set_computed_number(cc, tan(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "ATN(")) set_computed_number(cc, atan(v0), e->committed_epoch);
    else if (starts_with_ci(formula, "ROUND(")) {
      double digits = 0.0;
      if (argc >= 2) eval_comparison_or_numeric(e, args[1], ctx, &digits);
      int d = (int)llround(digits);
      if (d >= 0) {
        double f = pow(10.0, (double)d);
        set_computed_number(cc, round(v0 * f) / f, e->committed_epoch);
      } else {
        double f = pow(10.0, (double)(-d));
        set_computed_number(cc, round(v0 / f) * f, e->committed_epoch);
      }
    } else set_computed_error(cc, DVC_CELL_ERR_VALUE, "function eval", e->committed_epoch);
    return 1;
  }

  return 0;
}

static void dep_add_unique(uint32_t *deps, uint32_t *count, uint32_t cap, uint32_t idx) {
  if (!deps || !count || *count >= cap) return;
  for (uint32_t i = 0; i < *count; ++i) {
    if (deps[i] == idx) return;
  }
  deps[*count] = idx;
  (*count)++;
}

static uint32_t formula_collect_dependencies(const DvcEngine *e, const char *formula, uint32_t *deps, uint32_t cap) {
  if (!e || !formula || !deps || cap == 0) return 0;
  uint32_t count = 0;
  const char *s = formula;
  while (*s == '=' || isspace((unsigned char)*s)) s++;
  size_t len = strlen(s);
  int in_string = 0;
  for (size_t i = 0; i < len; ) {
    if (s[i] == '"') {
      in_string = !in_string;
      i++;
      continue;
    }
    if (in_string) {
      i++;
      continue;
    }
    int boundary_before = (i == 0) || !(isalnum((unsigned char)s[i - 1]) || s[i - 1] == '_');
    RefToken r1;
    if (boundary_before && parse_ref_token(s + i, &r1)) {
      char next = s[i + (size_t)r1.used_chars];
      int boundary_after = !(isalnum((unsigned char)next) || next == '_');
      if (boundary_after) {
        size_t pos = i + (size_t)r1.used_chars;
        size_t sep_len = 0;
        if (s[pos] == ':') sep_len = 1;
        else if (s[pos] == '.' && s[pos + 1] == '.' && s[pos + 2] == '.') sep_len = 3;
        if (sep_len > 0) {
          RefToken r2;
          if (parse_ref_token(s + pos + sep_len, &r2)) {
            char end_next = s[pos + sep_len + (size_t)r2.used_chars];
            if (!(isalnum((unsigned char)end_next) || end_next == '_')) {
              DvcCellAddr a = {r1.col, r1.row};
              DvcCellAddr b = {r2.col, r2.row};
              if (valid_addr(e, a) && valid_addr(e, b)) {
                uint16_t sr = a.row < b.row ? a.row : b.row;
                uint16_t er = a.row > b.row ? a.row : b.row;
                uint16_t sc = a.col < b.col ? a.col : b.col;
                uint16_t ec = a.col > b.col ? a.col : b.col;
                for (uint16_t rr = sr; rr <= er; ++rr) {
                  for (uint16_t ccx = sc; ccx <= ec; ++ccx) {
                    dep_add_unique(deps, &count, cap, addr_to_index(e, (DvcCellAddr){ccx, rr}));
                  }
                }
              }
              i = pos + sep_len + (size_t)r2.used_chars;
              continue;
            }
          }
        }
        DvcCellAddr a = {r1.col, r1.row};
        if (valid_addr(e, a)) {
          dep_add_unique(deps, &count, cap, addr_to_index(e, a));
        }
        i += (size_t)r1.used_chars;
        if (s[i] == '#') i++;
        continue;
      }
    }
    i++;
  }
  return count;
}

static int detect_cycle_dfs(
    const DvcEngine *e,
    uint32_t idx,
    uint8_t *state,
    uint32_t *stack,
    uint32_t depth,
    uint8_t *cycle_nodes) {
  state[idx] = 1;
  stack[depth] = idx;
  uint32_t deps[512];
  uint32_t dep_count = formula_collect_dependencies(e, e->cells[idx].text ? e->cells[idx].text : "", deps, 512);
  int any = 0;
  for (uint32_t i = 0; i < dep_count; ++i) {
    uint32_t dep = deps[i];
    if (dep >= e->cell_count || e->cells[dep].kind != DVC_INPUT_FORMULA) continue;
    if (state[dep] == 0) {
      if (detect_cycle_dfs(e, dep, state, stack, depth + 1, cycle_nodes)) any = 1;
    } else if (state[dep] == 1) {
      any = 1;
      int seen = 0;
      for (uint32_t sidx = 0; sidx <= depth; ++sidx) {
        if (stack[sidx] == dep) seen = 1;
        if (seen) cycle_nodes[stack[sidx]] = 1;
      }
      cycle_nodes[dep] = 1;
    }
  }
  state[idx] = 2;
  return any;
}

static int detect_simple_cycles(const DvcEngine *e, uint8_t *cycle_nodes) {
  if (!e || !cycle_nodes) return 0;
  uint8_t *state = (uint8_t *)calloc(e->cell_count ? e->cell_count : 1u, sizeof(uint8_t));
  uint32_t *stack = (uint32_t *)calloc(e->cell_count ? e->cell_count : 1u, sizeof(uint32_t));
  if ((e->cell_count && !state) || (e->cell_count && !stack)) {
    free(state);
    free(stack);
    return 0;
  }
  int any = 0;
  for (uint32_t i = 0; i < e->cell_count; ++i) {
    if (e->cells[i].kind != DVC_INPUT_FORMULA) continue;
    if (state[i] != 0) continue;
    if (detect_cycle_dfs(e, i, state, stack, 0, cycle_nodes)) any = 1;
  }
  free(state);
  free(stack);
  return any;
}

static int rewrite_axis_row(DvcStructuralOpKind op_kind, uint16_t at, uint16_t row, uint16_t *out_row, int *invalid) {
  if (!out_row || !invalid) return 0;
  *invalid = 0;
  *out_row = row;
  if (op_kind == DVC_STRUCT_OP_INSERT_ROW) {
    if (row >= at) *out_row = (uint16_t)(row + 1);
  } else if (op_kind == DVC_STRUCT_OP_DELETE_ROW) {
    if (row == at) *invalid = 1;
    else if (row > at) *out_row = (uint16_t)(row - 1);
  }
  return 1;
}

static int rewrite_axis_col(DvcStructuralOpKind op_kind, uint16_t at, uint16_t col, uint16_t *out_col, int *invalid) {
  if (!out_col || !invalid) return 0;
  *invalid = 0;
  *out_col = col;
  if (op_kind == DVC_STRUCT_OP_INSERT_COL) {
    if (col >= at) *out_col = (uint16_t)(col + 1);
  } else if (op_kind == DVC_STRUCT_OP_DELETE_COL) {
    if (col == at) *invalid = 1;
    else if (col > at) *out_col = (uint16_t)(col - 1);
  }
  return 1;
}

static int rewrite_ref_token(RefToken *rt, DvcStructuralOpKind op_kind, uint16_t at) {
  if (!rt) return 0;
  int invalid = 0;
  if (op_kind == DVC_STRUCT_OP_INSERT_ROW || op_kind == DVC_STRUCT_OP_DELETE_ROW) {
    if (!rt->abs_row) {
      uint16_t row = rt->row;
      rewrite_axis_row(op_kind, at, row, &row, &invalid);
      rt->row = row;
    }
  } else if (op_kind == DVC_STRUCT_OP_INSERT_COL || op_kind == DVC_STRUCT_OP_DELETE_COL) {
    if (!rt->abs_col) {
      uint16_t col = rt->col;
      rewrite_axis_col(op_kind, at, col, &col, &invalid);
      rt->col = col;
    }
  }
  return !invalid;
}

static int append_to_buf(char *dst, size_t cap, size_t *len, const char *src, size_t src_len) {
  if (!dst || !len || !src) return 0;
  if (*len + src_len + 1 > cap) return 0;
  memcpy(dst + *len, src, src_len);
  *len += src_len;
  dst[*len] = '\0';
  return 1;
}

static char *rewrite_formula_text(const char *formula, DvcStructuralOpKind op_kind, uint16_t at) {
  if (!formula) return dup_cstr("");
  size_t in_len = strlen(formula);
  size_t cap = in_len * 4u + 32u;
  char *out = (char *)calloc(cap, 1);
  if (!out) return NULL;
  size_t out_len = 0;
  int in_string = 0;
  for (size_t i = 0; i < in_len;) {
    char ch = formula[i];
    if (ch == '"') {
      in_string = !in_string;
      if (!append_to_buf(out, cap, &out_len, &formula[i], 1)) {
        free(out);
        return NULL;
      }
      i++;
      continue;
    }
    if (in_string) {
      if (!append_to_buf(out, cap, &out_len, &formula[i], 1)) {
        free(out);
        return NULL;
      }
      i++;
      continue;
    }

    int boundary_before = (i == 0) || !(isalnum((unsigned char)formula[i - 1]) || formula[i - 1] == '_');
    RefToken first;
    if (boundary_before && parse_ref_token(formula + i, &first)) {
      char next = formula[i + (size_t)first.used_chars];
      int boundary_after = !(isalnum((unsigned char)next) || next == '_');
      if (boundary_after) {
        size_t pos = i + (size_t)first.used_chars;
        int is_spill = (pos < in_len && formula[pos] == '#');
        int handled_range = 0;
        if (pos < in_len && (formula[pos] == ':' || (formula[pos] == '.' && pos + 2 < in_len && formula[pos + 1] == '.' && formula[pos + 2] == '.'))) {
          size_t sep_len = formula[pos] == ':' ? 1u : 3u;
          RefToken second;
          if (parse_ref_token(formula + pos + sep_len, &second)) {
            char after_second = formula[pos + sep_len + (size_t)second.used_chars];
            if (!(isalnum((unsigned char)after_second) || after_second == '_')) {
              handled_range = 1;
              int ok1 = rewrite_ref_token(&first, op_kind, at);
              int ok2 = rewrite_ref_token(&second, op_kind, at);
              if (!ok1 || !ok2) {
                const char *ref_err = "#REF!";
                if (!append_to_buf(out, cap, &out_len, ref_err, strlen(ref_err))) {
                  free(out);
                  return NULL;
                }
              } else {
                char ref1[32];
                char ref2[32];
                if (!format_ref_token(&first, ref1, sizeof(ref1)) || !format_ref_token(&second, ref2, sizeof(ref2))) {
                  free(out);
                  return NULL;
                }
                if (!append_to_buf(out, cap, &out_len, ref1, strlen(ref1)) ||
                    !append_to_buf(out, cap, &out_len, formula + pos, sep_len) ||
                    !append_to_buf(out, cap, &out_len, ref2, strlen(ref2))) {
                  free(out);
                  return NULL;
                }
              }
              i = pos + sep_len + (size_t)second.used_chars;
              continue;
            }
          }
        }
        if (!handled_range) {
          int ok = rewrite_ref_token(&first, op_kind, at);
          if (!ok) {
            const char *ref_err = "#REF!";
            if (!append_to_buf(out, cap, &out_len, ref_err, strlen(ref_err))) {
              free(out);
              return NULL;
            }
          } else {
            char ref1[32];
            if (!format_ref_token(&first, ref1, sizeof(ref1)) ||
                !append_to_buf(out, cap, &out_len, ref1, strlen(ref1))) {
              free(out);
              return NULL;
            }
            if (is_spill && !append_to_buf(out, cap, &out_len, "#", 1)) {
              free(out);
              return NULL;
            }
          }
          i += (size_t)first.used_chars + (size_t)(is_spill ? 1 : 0);
          continue;
        }
      }
    }

    if (!append_to_buf(out, cap, &out_len, &formula[i], 1)) {
      free(out);
      return NULL;
    }
    i++;
  }
  return out;
}

static void rewrite_formulas_for_structural_op(DvcEngine *engine, DvcStructuralOpKind op_kind, uint16_t at) {
  if (!engine) return;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    CellInput *ci = &engine->cells[i];
    if (ci->kind != DVC_INPUT_FORMULA || !ci->text) continue;
    char *rewritten = rewrite_formula_text(ci->text, op_kind, at);
    if (!rewritten) continue;
    free(ci->text);
    ci->text = rewritten;
  }
  for (uint32_t i = 0; i < engine->name_count; ++i) {
    NameEntry *ne = &engine->names[i];
    if (ne->kind != DVC_INPUT_FORMULA || !ne->text) continue;
    char *rewritten = rewrite_formula_text(ne->text, op_kind, at);
    if (!rewritten) continue;
    free(ne->text);
    ne->text = rewritten;
  }
}

static int structural_find_spill_constraint(const DvcEngine *engine,
                                            DvcStructuralOpKind op_kind,
                                            uint16_t at,
                                            DvcCellAddr *blocked_cell_out,
                                            DvcCellRange *blocked_range_out) {
  if (!engine) return 0;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    const CellComputed *cc = &engine->computed[i];
    if (!cc->spill_range_found || cc->spill_role != DVC_SPILL_ANCHOR) continue;
    uint16_t r1 = cc->spill_range.start.row;
    uint16_t r2 = cc->spill_range.end.row;
    uint16_t c1 = cc->spill_range.start.col;
    uint16_t c2 = cc->spill_range.end.col;
    int hit = 0;
    if (op_kind == DVC_STRUCT_OP_INSERT_ROW && at > r1 && at <= r2) hit = 1;
    if (op_kind == DVC_STRUCT_OP_DELETE_ROW && at >= r1 && at <= r2) hit = 1;
    if (op_kind == DVC_STRUCT_OP_INSERT_COL && at > c1 && at <= c2) hit = 1;
    if (op_kind == DVC_STRUCT_OP_DELETE_COL && at >= c1 && at <= c2) hit = 1;
    if (!hit) continue;
    if (blocked_cell_out) {
      *blocked_cell_out = cc->spill_anchor_found ? cc->spill_anchor : index_to_addr(engine, i);
    }
    if (blocked_range_out) {
      *blocked_range_out = cc->spill_range;
    }
    return 1;
  }
  return 0;
}

static void set_computed_blank(CellComputed *cc, uint64_t epoch) {
  free_cell_computed(cc);
  cc->value.type = DVC_VALUE_BLANK;
  cc->value.number = 0.0;
  cc->value.bool_val = 0;
  cc->value.error_kind = DVC_CELL_ERR_NULL;
  cc->value_epoch = epoch;
  cc->stale = 0;
  cc->spill_role = DVC_SPILL_NONE;
  cc->spill_anchor_found = 0;
  cc->spill_range_found = 0;
}

static void set_computed_number(CellComputed *cc, double v, uint64_t epoch) {
  free_cell_computed(cc);
  cc->value.type = DVC_VALUE_NUMBER;
  cc->value.number = v;
  cc->value.bool_val = 0;
  cc->value.error_kind = DVC_CELL_ERR_NULL;
  cc->value_epoch = epoch;
  cc->stale = 0;
  cc->spill_role = DVC_SPILL_NONE;
  cc->spill_anchor_found = 0;
  cc->spill_range_found = 0;
}

static void set_computed_text(CellComputed *cc, const char *txt, uint64_t epoch) {
  free_cell_computed(cc);
  cc->value.type = DVC_VALUE_TEXT;
  cc->value.number = 0.0;
  cc->value.bool_val = 0;
  cc->value.error_kind = DVC_CELL_ERR_NULL;
  cc->text = dup_cstr(txt ? txt : "");
  cc->value_epoch = epoch;
  cc->stale = 0;
  cc->spill_role = DVC_SPILL_NONE;
  cc->spill_anchor_found = 0;
  cc->spill_range_found = 0;
}

static void set_computed_bool(CellComputed *cc, int b, uint64_t epoch) {
  free_cell_computed(cc);
  cc->value.type = DVC_VALUE_BOOL;
  cc->value.number = 0.0;
  cc->value.bool_val = b ? 1 : 0;
  cc->value.error_kind = DVC_CELL_ERR_NULL;
  cc->value_epoch = epoch;
  cc->stale = 0;
  cc->spill_role = DVC_SPILL_NONE;
  cc->spill_anchor_found = 0;
  cc->spill_range_found = 0;
}

static void set_computed_error(CellComputed *cc, DvcCellErrorKind ek, const char *msg, uint64_t epoch) {
  free_cell_computed(cc);
  cc->value.type = DVC_VALUE_ERROR;
  cc->value.number = 0.0;
  cc->value.bool_val = 0;
  cc->value.error_kind = ek;
  cc->error_message = dup_cstr(msg ? msg : "error");
  cc->value_epoch = epoch;
  cc->stale = 0;
  cc->spill_role = DVC_SPILL_NONE;
  cc->spill_anchor_found = 0;
  cc->spill_range_found = 0;
}

static void set_computed_from_index(DvcEngine *e, CellComputed *dst, uint32_t ref_idx, const EvalCtx *ctx, uint64_t epoch) {
  if (ctx && ctx->non_iterative_cycle && ctx->cycle_nodes && ctx->cycle_nodes[ref_idx]) {
    double pv = 0.0;
    if (ctx->prior_types && ctx->prior_numbers &&
        (ctx->prior_types[ref_idx] == DVC_VALUE_NUMBER || ctx->prior_types[ref_idx] == DVC_VALUE_BOOL)) {
      pv = ctx->prior_numbers[ref_idx];
    }
    set_computed_number(dst, pv, epoch);
    return;
  }
  CellComputed *src = &e->computed[ref_idx];
  if (src->value.type == DVC_VALUE_NUMBER) {
    set_computed_number(dst, src->value.number, epoch);
  } else if (src->value.type == DVC_VALUE_TEXT) {
    set_computed_text(dst, src->text ? src->text : "", epoch);
  } else if (src->value.type == DVC_VALUE_BOOL) {
    set_computed_bool(dst, src->value.bool_val, epoch);
  } else if (src->value.type == DVC_VALUE_ERROR) {
    set_computed_error(dst, src->value.error_kind, src->error_message ? src->error_message : "error", epoch);
  } else {
    set_computed_blank(dst, epoch);
  }
}

static double get_cell_number_or_zero(const DvcEngine *e, DvcCellAddr addr) {
  if (!valid_addr(e, addr)) return 0.0;
  uint32_t idx = addr_to_index(e, addr);
  const CellComputed *cc = &e->computed[idx];
  if (cc->value.type == DVC_VALUE_NUMBER) return cc->value.number;
  if (cc->value.type == DVC_VALUE_BOOL) return cc->value.bool_val ? 1.0 : 0.0;
  return 0.0;
}

static DvcStatus do_recalculate(DvcEngine *e) {
  double *prior_numbers = (double *)calloc(e->cell_count ? e->cell_count : 1u, sizeof(double));
  DvcValueType *prior_types = (DvcValueType *)calloc(e->cell_count ? e->cell_count : 1u, sizeof(DvcValueType));
  uint8_t *cycle_nodes = (uint8_t *)calloc(e->cell_count ? e->cell_count : 1u, sizeof(uint8_t));
  if ((e->cell_count && !prior_numbers) || (e->cell_count && !prior_types) || (e->cell_count && !cycle_nodes)) {
    free(prior_numbers);
    free(prior_types);
    free(cycle_nodes);
    return DVC_ERR_OUT_OF_MEMORY;
  }

  for (uint32_t i = 0; i < e->cell_count; ++i) {
    prior_types[i] = e->computed[i].value.type;
    if (e->computed[i].value.type == DVC_VALUE_NUMBER) prior_numbers[i] = e->computed[i].value.number;
    else if (e->computed[i].value.type == DVC_VALUE_BOOL) prior_numbers[i] = e->computed[i].value.bool_val ? 1.0 : 0.0;
    else prior_numbers[i] = 0.0;
  }

  int has_cycle = detect_simple_cycles(e, cycle_nodes);
  if (has_cycle && !e->iter_cfg.enabled) {
    change_push_diag(e, DVC_DIAG_CIRCULAR_REFERENCE_DETECTED, "Circular reference detected");
  }

  for (uint32_t i = 0; i < e->cell_count; ++i) {
    e->computed[i].spill_role = DVC_SPILL_NONE;
    e->computed[i].spill_anchor_found = 0;
    e->computed[i].spill_range_found = 0;
    if (!(e->cells[i].kind == DVC_INPUT_FORMULA && formula_is_stream(e->cells[i].text))) {
      e->stream_active[i] = 0;
      e->stream_period[i] = 0.0;
      e->stream_elapsed[i] = 0.0;
    }
  }

  for (uint32_t i = 0; i < e->cell_count; ++i) {
    CellInput *ci = &e->cells[i];
    CellComputed *cc = &e->computed[i];
    if (ci->kind == DVC_INPUT_EMPTY) {
      if (cc->spill_anchor_found) {
        continue;
      }
      set_computed_blank(cc, e->committed_epoch);
    } else if (ci->kind == DVC_INPUT_NUMBER) {
      set_computed_number(cc, ci->number, e->committed_epoch);
    } else if (ci->kind == DVC_INPUT_TEXT) {
      set_computed_text(cc, ci->text ? ci->text : "", e->committed_epoch);
    } else if (ci->kind == DVC_INPUT_FORMULA) {
      const char *f = ci->text ? ci->text : "";
      while (*f == '=') f++;
      while (isspace((unsigned char)*f)) f++;

      if ((strncasecmp(f, "SEQUENCE(", 9) == 0) || (strncasecmp(f, "RANDARRAY(", 10) == 0)) {
        int is_randarray = (toupper((unsigned char)f[0]) == 'R');
        int rows = 1;
        int cols = 1;
        double start_or_min = is_randarray ? 0.0 : 1.0;
        double step_or_max = 1.0;
        int whole = 0;

        const char *body_start = NULL;
        size_t body_len = 0;
        if (!parse_function_body(f, is_randarray ? "RANDARRAY" : "SEQUENCE", &body_start, &body_len)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid dynamic-array call", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        char body[512];
        if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
        memcpy(body, body_start, body_len);
        body[body_len] = '\0';

        char args[5][256];
        int argc = split_top_level_args(body, args, 5);
        if (argc < 0) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid dynamic-array arguments", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        EvalCtx arg_ctx;
        memset(&arg_ctx, 0, sizeof(arg_ctx));
        arg_ctx.prior_numbers = prior_numbers;
        arg_ctx.prior_types = prior_types;
        arg_ctx.cycle_nodes = cycle_nodes;
        arg_ctx.non_iterative_cycle = !e->iter_cfg.enabled;

        int args_ok = 1;
        double parsed = 0.0;
        if (argc >= 1 && args[0][0] != '\0' && !eval_numeric_expr(e, args[0], &arg_ctx, &parsed)) args_ok = 0;
        if (args_ok && argc >= 1 && args[0][0] != '\0') rows = (int)llround(parsed);
        if (args_ok && argc >= 2 && args[1][0] != '\0' && !eval_numeric_expr(e, args[1], &arg_ctx, &parsed)) args_ok = 0;
        if (args_ok && argc >= 2 && args[1][0] != '\0') cols = (int)llround(parsed);
        if (args_ok && argc >= 3 && args[2][0] != '\0' && !eval_numeric_expr(e, args[2], &arg_ctx, &parsed)) args_ok = 0;
        if (args_ok && argc >= 3 && args[2][0] != '\0') start_or_min = parsed;
        if (args_ok && argc >= 4 && args[3][0] != '\0' && !eval_numeric_expr(e, args[3], &arg_ctx, &parsed)) args_ok = 0;
        if (args_ok && argc >= 4 && args[3][0] != '\0') step_or_max = parsed;
        if (args_ok && argc >= 5 && args[4][0] != '\0' && !eval_numeric_expr(e, args[4], &arg_ctx, &parsed)) args_ok = 0;
        if (args_ok && argc >= 5 && args[4][0] != '\0') whole = (fabs(parsed) > 0.0) ? 1 : 0;

        if (!args_ok) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid dynamic-array arguments", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        if (rows <= 0) rows = 1;
        if (cols <= 0) cols = 1;
        if (!is_randarray && argc < 4) step_or_max = 1.0;
        if (is_randarray && step_or_max < start_or_min) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "RANDARRAY max < min", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        DvcCellAddr anchor = index_to_addr(e, i);
        int end_row = anchor.row + rows - 1;
        int end_col = anchor.col + cols - 1;
        if (end_row > e->bounds.max_rows || end_col > e->bounds.max_columns) {
          set_computed_error(cc, DVC_CELL_ERR_SPILL, "spill out of bounds", e->committed_epoch);
          change_push_cell(e, anchor);
          continue;
        }

        int blocked = 0;
        for (int rr = 0; rr < rows && !blocked; ++rr) {
          for (int ccx = 0; ccx < cols; ++ccx) {
            DvcCellAddr t;
            t.row = (uint16_t)(anchor.row + rr);
            t.col = (uint16_t)(anchor.col + ccx);
            uint32_t tidx = addr_to_index(e, t);
            if (tidx != i && e->cells[tidx].kind != DVC_INPUT_EMPTY) {
              blocked = 1;
              break;
            }
          }
        }

        if (blocked) {
          set_computed_error(cc, DVC_CELL_ERR_SPILL, "spill blocked", e->committed_epoch);
          change_push_cell(e, anchor);
          continue;
        }

        if (is_randarray) {
          e->stream_counter[i] += 1.0;
        }

        for (int rr = 0; rr < rows; ++rr) {
          for (int ccx = 0; ccx < cols; ++ccx) {
            DvcCellAddr t;
            t.row = (uint16_t)(anchor.row + rr);
            t.col = (uint16_t)(anchor.col + ccx);
            uint32_t tidx = addr_to_index(e, t);
            CellComputed *dst = &e->computed[tidx];
            double v = 0.0;
            if (is_randarray) {
              double seed = e->stream_counter[i] + (double)(rr * cols + ccx + 1);
              double frac = fmod(seed * 0.61803398875 + 0.1234567, 1.0);
              if (frac < 0.0) frac += 1.0;
              if (whole) {
                long lo = (long)ceil(start_or_min);
                long hi = (long)floor(step_or_max);
                if (hi < lo) {
                  set_computed_error(cc, DVC_CELL_ERR_VALUE, "RANDARRAY integer bounds invalid", e->committed_epoch);
                  change_push_cell(e, anchor);
                  goto dynamic_array_done;
                }
                long width = hi - lo + 1;
                long pick = (long)floor(frac * (double)width);
                if (pick >= width) pick = width - 1;
                v = (double)(lo + pick);
              } else {
                double span = step_or_max - start_or_min;
                v = start_or_min + span * frac;
              }
            } else {
              v = start_or_min + step_or_max * (rr * cols + ccx);
            }
            set_computed_number(dst, v, e->committed_epoch);
            dst->spill_anchor_found = 1;
            dst->spill_anchor = anchor;
            dst->spill_range_found = 1;
            dst->spill_range.start = anchor;
            dst->spill_range.end.row = (uint16_t)end_row;
            dst->spill_range.end.col = (uint16_t)end_col;
            dst->spill_role = (rr == 0 && ccx == 0) ? DVC_SPILL_ANCHOR : DVC_SPILL_MEMBER;
          }
        }

        change_push_cell(e, anchor);
dynamic_array_done:
        continue;
      }

      if (starts_with_ci(f, "MAP(")) {
        const char *body_start = NULL;
        size_t body_len = 0;
        if (!parse_function_body(f, "MAP", &body_start, &body_len)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid MAP", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }
        char body[512];
        if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
        memcpy(body, body_start, body_len);
        body[body_len] = '\0';
        char args[3][256];
        int argc = split_top_level_args(body, args, 3);
        if (argc < 2) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "MAP requires args", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        RefToken r1;
        RefToken r2;
        if (!parse_ref_token(args[0], &r1)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "MAP source must be range", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }
        char *sep = args[0] + r1.used_chars;
        int sep_len = 0;
        if (*sep == ':') sep_len = 1;
        else if (strncmp(sep, "...", 3) == 0) sep_len = 3;
        if (sep_len == 0 || !parse_ref_token(sep + sep_len, &r2) || sep[sep_len + r2.used_chars] != '\0') {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "MAP source must be range", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }
        uint16_t sr = r1.row, er = r2.row;
        uint16_t sc = r1.col, ec = r2.col;
        if (sr > er) { uint16_t t = sr; sr = er; er = t; }
        if (sc > ec) { uint16_t t = sc; sc = ec; ec = t; }
        int rows = (int)(er - sr + 1);
        int cols = (int)(ec - sc + 1);

        const char *lambda_body_start = NULL;
        size_t lambda_body_len = 0;
        if (!parse_function_body(args[1], "LAMBDA", &lambda_body_start, &lambda_body_len)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "MAP requires LAMBDA", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }
        char lambda_body[512];
        if (lambda_body_len >= sizeof(lambda_body)) lambda_body_len = sizeof(lambda_body) - 1;
        memcpy(lambda_body, lambda_body_start, lambda_body_len);
        lambda_body[lambda_body_len] = '\0';
        char lambda_args[3][256];
        int lambda_argc = split_top_level_args(lambda_body, lambda_args, 3);
        if (lambda_argc < 2) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "MAP lambda arity", e->committed_epoch);
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        DvcCellAddr anchor = index_to_addr(e, i);
        int end_row = anchor.row + rows - 1;
        int end_col = anchor.col + cols - 1;
        if (end_row > e->bounds.max_rows || end_col > e->bounds.max_columns) {
          set_computed_error(cc, DVC_CELL_ERR_SPILL, "spill out of bounds", e->committed_epoch);
          change_push_cell(e, anchor);
          continue;
        }
        int blocked = 0;
        for (int rr = 0; rr < rows && !blocked; ++rr) {
          for (int ccx = 0; ccx < cols; ++ccx) {
            DvcCellAddr t = {(uint16_t)(anchor.col + ccx), (uint16_t)(anchor.row + rr)};
            uint32_t tidx = addr_to_index(e, t);
            if (tidx != i && e->cells[tidx].kind != DVC_INPUT_EMPTY) {
              blocked = 1;
              break;
            }
          }
        }
        if (blocked) {
          set_computed_error(cc, DVC_CELL_ERR_SPILL, "spill blocked", e->committed_epoch);
          change_push_cell(e, anchor);
          continue;
        }

        for (int rr = 0; rr < rows; ++rr) {
          for (int ccx = 0; ccx < cols; ++ccx) {
            DvcCellAddr src_addr = {(uint16_t)(sc + ccx), (uint16_t)(sr + rr)};
            uint32_t src_idx = addr_to_index(e, src_addr);
            double src_val = numeric_from_index_with_cycle(e, src_idx, &(EvalCtx){
              .prior_numbers = prior_numbers,
              .prior_types = prior_types,
              .cycle_nodes = cycle_nodes,
              .non_iterative_cycle = !e->iter_cfg.enabled
            });
            EvalCtx lambda_ctx;
            memset(&lambda_ctx, 0, sizeof(lambda_ctx));
            lambda_ctx.has_var = 1;
            lambda_ctx.var_name = lambda_args[0];
            lambda_ctx.var_value = src_val;
            lambda_ctx.prior_numbers = prior_numbers;
            lambda_ctx.prior_types = prior_types;
            lambda_ctx.cycle_nodes = cycle_nodes;
            lambda_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
            double out_num = 0.0;
            if (!eval_numeric_expr(e, lambda_args[1], &lambda_ctx, &out_num)) {
              out_num = 0.0;
            }
            DvcCellAddr dst_addr = {(uint16_t)(anchor.col + ccx), (uint16_t)(anchor.row + rr)};
            uint32_t dst_idx = addr_to_index(e, dst_addr);
            CellComputed *dst = &e->computed[dst_idx];
            set_computed_number(dst, out_num, e->committed_epoch);
            dst->spill_anchor_found = 1;
            dst->spill_anchor = anchor;
            dst->spill_range_found = 1;
            dst->spill_range.start = anchor;
            dst->spill_range.end.row = (uint16_t)end_row;
            dst->spill_range.end.col = (uint16_t)end_col;
            dst->spill_role = (rr == 0 && ccx == 0) ? DVC_SPILL_ANCHOR : DVC_SPILL_MEMBER;
          }
        }
        change_push_cell(e, anchor);
        continue;
      }

      if (starts_with_ci(f, "LET(")) {
        const char *body_start = NULL;
        size_t body_len = 0;
        if (parse_function_body(f, "LET", &body_start, &body_len)) {
          char body[512];
          if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
          memcpy(body, body_start, body_len);
          body[body_len] = '\0';
          char args[4][256];
          int argc = split_top_level_args(body, args, 4);
          if (argc >= 3) {
            EvalCtx base_ctx;
            memset(&base_ctx, 0, sizeof(base_ctx));
            base_ctx.prior_numbers = prior_numbers;
            base_ctx.prior_types = prior_types;
            base_ctx.cycle_nodes = cycle_nodes;
            base_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
            double bind_value = 0.0;
            if (eval_numeric_expr(e, args[1], &base_ctx, &bind_value)) {
              EvalCtx let_ctx = base_ctx;
              let_ctx.has_var = 1;
              let_ctx.var_name = args[0];
              let_ctx.var_value = bind_value;
              double out_num = 0.0;
              if (eval_numeric_expr(e, args[2], &let_ctx, &out_num)) {
                set_computed_number(cc, out_num, e->committed_epoch);
                change_push_cell(e, index_to_addr(e, i));
                continue;
              }
            }
          }
        }
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid LET", e->committed_epoch);
        change_push_cell(e, index_to_addr(e, i));
        continue;
      }

      if (starts_with_ci(f, "LAMBDA(")) {
        const char *first_body = NULL;
        size_t first_body_len = 0;
        if (parse_function_body(f, "LAMBDA", &first_body, &first_body_len)) {
          const char *after = first_body + first_body_len + 1;
          after = skip_ws(after);
          if (*after == '(') {
            int depth = 0;
            const char *p = after;
            const char *endp = NULL;
            while (*p) {
              if (*p == '(') depth++;
              else if (*p == ')') {
                depth--;
                if (depth == 0) {
                  endp = p;
                  break;
                }
              }
              ++p;
            }
            if (endp) {
              char lambda_body[512];
              size_t l_len = first_body_len;
              if (l_len >= sizeof(lambda_body)) l_len = sizeof(lambda_body) - 1;
              memcpy(lambda_body, first_body, l_len);
              lambda_body[l_len] = '\0';
              char lambda_args[3][256];
              int lambda_argc = split_top_level_args(lambda_body, lambda_args, 3);
              if (lambda_argc >= 2) {
                char invoke_arg[256];
                size_t inv_len = (size_t)(endp - (after + 1));
                if (inv_len >= sizeof(invoke_arg)) inv_len = sizeof(invoke_arg) - 1;
                memcpy(invoke_arg, after + 1, inv_len);
                invoke_arg[inv_len] = '\0';
                trim_inplace(invoke_arg);
                EvalCtx base_ctx;
                memset(&base_ctx, 0, sizeof(base_ctx));
                base_ctx.prior_numbers = prior_numbers;
                base_ctx.prior_types = prior_types;
                base_ctx.cycle_nodes = cycle_nodes;
                base_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
                double arg_val = 0.0;
                if (eval_numeric_expr(e, invoke_arg, &base_ctx, &arg_val)) {
                  EvalCtx lambda_ctx = base_ctx;
                  lambda_ctx.has_var = 1;
                  lambda_ctx.var_name = lambda_args[0];
                  lambda_ctx.var_value = arg_val;
                  double out_num = 0.0;
                  if (eval_numeric_expr(e, lambda_args[1], &lambda_ctx, &out_num)) {
                    set_computed_number(cc, out_num, e->committed_epoch);
                    change_push_cell(e, index_to_addr(e, i));
                    continue;
                  }
                }
              }
            }
          }
        }
        set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid LAMBDA invocation", e->committed_epoch);
        change_push_cell(e, index_to_addr(e, i));
        continue;
      }

      if (strncasecmp(f, "RAND()", 6) == 0) {
        double seed = e->stream_counter[i] + 1.0;
        e->stream_counter[i] = seed;
        double v = fmod(seed * 0.61803398875 + 0.13579, 1.0);
        set_computed_number(cc, v, e->committed_epoch);
      } else if (strncasecmp(f, "NOW()", 5) == 0) {
        set_computed_number(cc, (double)e->committed_epoch, e->committed_epoch);
      } else if (strncasecmp(f, "STREAM(", 7) == 0) {
        const char *p = strchr(f, '(');
        double period = 1.0;
        if (p) {
          period = strtod(p + 1, NULL);
        }
        if (period <= 0.0) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "STREAM period must be > 0", e->committed_epoch);
        } else {
          e->stream_active[i] = 1;
          e->stream_period[i] = period;
          set_computed_number(cc, e->stream_counter[i], e->committed_epoch);
        }
      } else if (starts_with_ci(f, "INDIRECT(")) {
        const char *body_start = NULL;
        size_t body_len = 0;
        if (!parse_function_body(f, "INDIRECT", &body_start, &body_len)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid INDIRECT", e->committed_epoch);
        } else {
          char body[512];
          if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
          memcpy(body, body_start, body_len);
          body[body_len] = '\0';
          char args[3][256];
          int argc = split_top_level_args(body, args, 3);
          int a1_mode = 1;
          if (argc >= 2 && (cmp_ci(args[1], "FALSE") == 0 || strcmp(args[1], "0") == 0)) a1_mode = 0;
          if (argc >= 1) {
            const char *raw = args[0];
            char ref_text[256];
            size_t raw_len = strlen(raw);
            if (raw_len >= 2 && raw[0] == '"' && raw[raw_len - 1] == '"') {
              size_t inner_len = raw_len - 2;
              if (inner_len >= sizeof(ref_text)) inner_len = sizeof(ref_text) - 1;
              memcpy(ref_text, raw + 1, inner_len);
              ref_text[inner_len] = '\0';
            } else {
              snprintf(ref_text, sizeof(ref_text), "%s", raw);
            }
            DvcCellAddr ref_addr;
            int ok = a1_mode ? parse_a1_addr(e, ref_text, (uint32_t)strlen(ref_text), &ref_addr)
                             : parse_r1c1_absolute(e, ref_text, &ref_addr);
            if (ok) {
              EvalCtx ref_ctx;
              memset(&ref_ctx, 0, sizeof(ref_ctx));
              ref_ctx.prior_numbers = prior_numbers;
              ref_ctx.prior_types = prior_types;
              ref_ctx.cycle_nodes = cycle_nodes;
              ref_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
              set_computed_from_index(e, cc, addr_to_index(e, ref_addr), &ref_ctx, e->committed_epoch);
            } else {
              set_computed_error(cc, DVC_CELL_ERR_REF, "INDIRECT reference invalid", e->committed_epoch);
            }
          } else {
            set_computed_error(cc, DVC_CELL_ERR_VALUE, "INDIRECT requires argument", e->committed_epoch);
          }
        }
      } else if (starts_with_ci(f, "OFFSET(")) {
        const char *body_start = NULL;
        size_t body_len = 0;
        if (!parse_function_body(f, "OFFSET", &body_start, &body_len)) {
          set_computed_error(cc, DVC_CELL_ERR_VALUE, "invalid OFFSET", e->committed_epoch);
        } else {
          char body[512];
          if (body_len >= sizeof(body)) body_len = sizeof(body) - 1;
          memcpy(body, body_start, body_len);
          body[body_len] = '\0';
          char args[5][256];
          int argc = split_top_level_args(body, args, 5);
          if (argc < 3) {
            set_computed_error(cc, DVC_CELL_ERR_VALUE, "OFFSET requires args", e->committed_epoch);
          } else {
            RefToken base_ref;
            EvalCtx eval_ctx;
            memset(&eval_ctx, 0, sizeof(eval_ctx));
            eval_ctx.prior_numbers = prior_numbers;
            eval_ctx.prior_types = prior_types;
            eval_ctx.cycle_nodes = cycle_nodes;
            eval_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
            if (!parse_ref_token(args[0], &base_ref) || args[0][base_ref.used_chars] != '\0') {
              set_computed_error(cc, DVC_CELL_ERR_REF, "OFFSET base invalid", e->committed_epoch);
            } else {
              DvcCellAddr base = {base_ref.col, base_ref.row};
              double dr = 0.0, dc = 0.0;
              if (!valid_addr(e, base) ||
                  !eval_numeric_expr(e, args[1], &eval_ctx, &dr) ||
                  !eval_numeric_expr(e, args[2], &eval_ctx, &dc)) {
                set_computed_error(cc, DVC_CELL_ERR_VALUE, "OFFSET arguments invalid", e->committed_epoch);
              } else {
                long tr = (long)base.row + (long)llround(dr);
                long tc = (long)base.col + (long)llround(dc);
                if (tr < 1 || tc < 1) {
                  set_computed_error(cc, DVC_CELL_ERR_REF, "OFFSET target out of bounds", e->committed_epoch);
                } else {
                  DvcCellAddr target = {(uint16_t)tc, (uint16_t)tr};
                  if (!valid_addr(e, target)) {
                    set_computed_error(cc, DVC_CELL_ERR_REF, "OFFSET target out of bounds", e->committed_epoch);
                  } else {
                    set_computed_from_index(e, cc, addr_to_index(e, target), &eval_ctx, e->committed_epoch);
                  }
                }
              }
            }
          }
        }
      } else {
        EvalCtx fn_ctx;
        memset(&fn_ctx, 0, sizeof(fn_ctx));
        fn_ctx.prior_numbers = prior_numbers;
        fn_ctx.prior_types = prior_types;
        fn_ctx.cycle_nodes = cycle_nodes;
        fn_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
        if (try_eval_required_fn(e, cc, index_to_addr(e, i), f, &fn_ctx)) {
          change_push_cell(e, index_to_addr(e, i));
          continue;
        }

        DvcCellAddr ref;
        int used = 0;
        if (parse_cell_ref_raw(f, (uint32_t)strlen(f), &ref, &used) && (size_t)used == strlen(f) && valid_addr(e, ref)) {
          uint32_t ridx = addr_to_index(e, ref);
          EvalCtx ref_ctx;
          memset(&ref_ctx, 0, sizeof(ref_ctx));
          ref_ctx.prior_numbers = prior_numbers;
          ref_ctx.prior_types = prior_types;
          ref_ctx.cycle_nodes = cycle_nodes;
          ref_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
          set_computed_from_index(e, cc, ridx, &ref_ctx, e->committed_epoch);
        } else {
          char *end = NULL;
          double n = strtod(f, &end);
          if (end != f && *end == '\0') {
            set_computed_number(cc, n, e->committed_epoch);
          } else {
            const char *plus = strchr(f, '+');
            const char *minus = strchr(f, '-');
            const char *mul = strchr(f, '*');
            const char *div = strchr(f, '/');
            const char *op = plus ? plus : (minus ? minus : (mul ? mul : div));
            if (op) {
              char left[128];
              char right[128];
              size_t l = (size_t)(op - f);
              size_t r = strlen(op + 1);
              if (l >= sizeof(left)) l = sizeof(left) - 1;
              if (r >= sizeof(right)) r = sizeof(right) - 1;
              memcpy(left, f, l);
              left[l] = '\0';
              memcpy(right, op + 1, r);
              right[r] = '\0';

              double lv = parse_first_number(left, 0.0);
              double rv = parse_first_number(right, 0.0);

              DvcCellAddr ra;
              EvalCtx num_ctx;
              memset(&num_ctx, 0, sizeof(num_ctx));
              num_ctx.prior_numbers = prior_numbers;
              num_ctx.prior_types = prior_types;
              num_ctx.cycle_nodes = cycle_nodes;
              num_ctx.non_iterative_cycle = !e->iter_cfg.enabled;
              if (parse_a1_addr(e, left, (uint32_t)strlen(left), &ra)) lv = numeric_from_index_with_cycle(e, addr_to_index(e, ra), &num_ctx);
              else eval_numeric_expr(e, left, &num_ctx, &lv);
              if (parse_a1_addr(e, right, (uint32_t)strlen(right), &ra)) rv = numeric_from_index_with_cycle(e, addr_to_index(e, ra), &num_ctx);
              else eval_numeric_expr(e, right, &num_ctx, &rv);

              double out = 0.0;
              if (*op == '+') out = lv + rv;
              if (*op == '-') out = lv - rv;
              if (*op == '*') out = lv * rv;
              if (*op == '/') {
                if (rv == 0.0) {
                  set_computed_error(cc, DVC_CELL_ERR_DIV_ZERO, "division by zero", e->committed_epoch);
                  continue;
                }
                out = lv / rv;
              }
              set_computed_number(cc, out, e->committed_epoch);
            } else {
              set_computed_error(cc, DVC_CELL_ERR_VALUE, "unsupported formula", e->committed_epoch);
            }
          }
        }
      }
      change_push_cell(e, index_to_addr(e, i));
    }
  }

  if (has_cycle && e->iter_cfg.enabled) {
    uint32_t max_iters = e->iter_cfg.max_iterations;
    if (max_iters < 1) max_iters = 1;
    double tol = e->iter_cfg.convergence_tolerance;
    if (tol < 0.0) tol = -tol;
    for (uint32_t iter = 1; iter < max_iters; ++iter) {
      double max_delta = 0.0;
      int any_numeric_update = 0;
      for (uint32_t i = 0; i < e->cell_count; ++i) {
        if (!cycle_nodes[i] || e->cells[i].kind != DVC_INPUT_FORMULA) continue;
        CellComputed *cc = &e->computed[i];
        DvcValueType before_t = cc->value.type;
        double before_v = 0.0;
        if (before_t == DVC_VALUE_NUMBER) before_v = cc->value.number;
        else if (before_t == DVC_VALUE_BOOL) before_v = cc->value.bool_val ? 1.0 : 0.0;

        const char *f = e->cells[i].text ? e->cells[i].text : "";
        while (*f == '=') f++;
        while (isspace((unsigned char)*f)) f++;

        EvalCtx iter_ctx;
        memset(&iter_ctx, 0, sizeof(iter_ctx));
        iter_ctx.prior_numbers = prior_numbers;
        iter_ctx.prior_types = prior_types;
        iter_ctx.cycle_nodes = cycle_nodes;
        iter_ctx.non_iterative_cycle = 0;

        if (try_eval_required_fn(e, cc, index_to_addr(e, i), f, &iter_ctx)) {
        } else {
          DvcCellAddr ref;
          int used = 0;
          if (parse_cell_ref_raw(f, (uint32_t)strlen(f), &ref, &used) && (size_t)used == strlen(f) && valid_addr(e, ref)) {
            set_computed_from_index(e, cc, addr_to_index(e, ref), &iter_ctx, e->committed_epoch);
          } else {
            double out_num = 0.0;
            if (eval_comparison_or_numeric(e, f, &iter_ctx, &out_num)) {
              if (starts_with_ci(f, "AND(") || starts_with_ci(f, "OR(") || starts_with_ci(f, "NOT(") ||
                  starts_with_ci(f, "ISERROR(") || starts_with_ci(f, "ISNA(") || starts_with_ci(f, "ISBLANK(") ||
                  starts_with_ci(f, "ISTEXT(") || starts_with_ci(f, "ISNUMBER(") || starts_with_ci(f, "ISLOGICAL(")) {
                set_computed_bool(cc, fabs(out_num) > 0.0, e->committed_epoch);
              } else {
                set_computed_number(cc, out_num, e->committed_epoch);
              }
            }
          }
        }

        DvcValueType after_t = cc->value.type;
        double after_v = 0.0;
        if (after_t == DVC_VALUE_NUMBER) after_v = cc->value.number;
        else if (after_t == DVC_VALUE_BOOL) after_v = cc->value.bool_val ? 1.0 : 0.0;
        if ((before_t == DVC_VALUE_NUMBER || before_t == DVC_VALUE_BOOL) &&
            (after_t == DVC_VALUE_NUMBER || after_t == DVC_VALUE_BOOL)) {
          double delta = fabs(after_v - before_v);
          if (delta > max_delta) max_delta = delta;
          any_numeric_update = 1;
        }
      }
      if (!any_numeric_update || max_delta <= tol) break;
    }
  }

  e->stabilized_epoch = e->committed_epoch;
  for (uint32_t i = 0; i < e->cell_count; ++i) {
    e->computed[i].stale = 0;
  }
  free(prior_numbers);
  free(prior_types);
  free(cycle_nodes);
  return DVC_OK;
}

static void mark_stale(DvcEngine *e) {
  for (uint32_t i = 0; i < e->cell_count; ++i) {
    e->computed[i].stale = (e->computed[i].value_epoch < e->committed_epoch) ? 1 : 0;
  }
}
DvcStatus dvc_engine_create(DvcEngine **out) {
  if (!out) return DVC_ERR_NULL_POINTER;
  DvcSheetBounds b;
  b.max_columns = 63;
  b.max_rows = 254;
  return dvc_engine_create_with_bounds(b, out);
}

DvcStatus dvc_engine_create_with_bounds(DvcSheetBounds bounds, DvcEngine **out) {
  if (!out) return DVC_ERR_NULL_POINTER;
  if (bounds.max_columns == 0 || bounds.max_rows == 0 || bounds.max_columns > 1024 || bounds.max_rows > 4096) {
    return DVC_ERR_INVALID_ARGUMENT;
  }

  DvcEngine *e = (DvcEngine *)calloc(1, sizeof(DvcEngine));
  if (!e) return DVC_ERR_OUT_OF_MEMORY;

  e->bounds = bounds;
  e->recalc_mode = DVC_RECALC_AUTOMATIC;
  e->iter_cfg.enabled = 0;
  e->iter_cfg.max_iterations = 100;
  e->iter_cfg.convergence_tolerance = 0.001;
  e->committed_epoch = 0;
  e->stabilized_epoch = 0;
  e->last_error_kind = DVC_OK;
  e->last_reject_kind = DVC_REJECT_KIND_NONE;

  e->cell_count = (size_t)bounds.max_columns * (size_t)bounds.max_rows;
  e->cells = (CellInput *)calloc(e->cell_count, sizeof(CellInput));
  e->computed = (CellComputed *)calloc(e->cell_count, sizeof(CellComputed));
  e->formats = (DvcCellFormat *)calloc(e->cell_count, sizeof(DvcCellFormat));
  e->stream_active = (int *)calloc(e->cell_count, sizeof(int));
  e->stream_period = (double *)calloc(e->cell_count, sizeof(double));
  e->stream_elapsed = (double *)calloc(e->cell_count, sizeof(double));
  e->stream_counter = (double *)calloc(e->cell_count, sizeof(double));

  if (!e->cells || !e->computed || !e->formats || !e->stream_active || !e->stream_period || !e->stream_elapsed || !e->stream_counter) {
    dvc_engine_destroy(e);
    return DVC_ERR_OUT_OF_MEMORY;
  }

  for (size_t i = 0; i < e->cell_count; ++i) {
    e->formats[i] = default_format();
    set_computed_blank(&e->computed[i], 0);
  }

  *out = e;
  clear_status(e);
  return DVC_OK;
}

DvcStatus dvc_engine_destroy(DvcEngine *engine) {
  if (!engine) return DVC_OK;

  for (size_t i = 0; i < engine->cell_count; ++i) {
    free_cell_input(&engine->cells[i]);
    free_cell_computed(&engine->computed[i]);
  }
  free(engine->cells);
  free(engine->computed);
  free(engine->formats);
  free(engine->stream_active);
  free(engine->stream_period);
  free(engine->stream_elapsed);
  free(engine->stream_counter);

  for (uint32_t i = 0; i < engine->name_count; ++i) {
    free(engine->names[i].name);
    free(engine->names[i].text);
  }
  free(engine->names);

  for (uint32_t i = 0; i < engine->control_count; ++i) {
    free(engine->controls[i].name);
  }
  free(engine->controls);

  for (uint32_t i = 0; i < engine->chart_count; ++i) {
    free(engine->charts[i].name);
  }
  free(engine->charts);

  for (uint32_t i = 0; i < engine->udf_count; ++i) {
    free(engine->udfs[i].name);
  }
  free(engine->udfs);

  for (uint32_t i = 0; i < engine->change_count; ++i) {
    free(engine->changes[i].name);
    free(engine->changes[i].chart_name);
    free(engine->changes[i].diag_message);
  }
  free(engine->changes);

  free(engine->last_error_message);
  free(engine);
  return DVC_OK;
}

DvcStatus dvc_engine_clear(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  for (size_t i = 0; i < engine->cell_count; ++i) {
    free_cell_input(&engine->cells[i]);
    set_computed_blank(&engine->computed[i], engine->committed_epoch + 1);
    engine->formats[i] = default_format();
    engine->stream_active[i] = 0;
    engine->stream_period[i] = 0.0;
    engine->stream_elapsed[i] = 0.0;
    engine->stream_counter[i] = 0.0;
  }
  for (uint32_t i = 0; i < engine->name_count; ++i) {
    free(engine->names[i].name);
    free(engine->names[i].text);
  }
  engine->name_count = 0;
  for (uint32_t i = 0; i < engine->control_count; ++i) {
    free(engine->controls[i].name);
  }
  engine->control_count = 0;
  for (uint32_t i = 0; i < engine->chart_count; ++i) {
    free(engine->charts[i].name);
  }
  engine->chart_count = 0;
  for (uint32_t i = 0; i < engine->udf_count; ++i) {
    free(engine->udfs[i].name);
  }
  engine->udf_count = 0;
  engine->committed_epoch++;
  engine->stabilized_epoch = engine->committed_epoch;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_engine_bounds(const DvcEngine *engine, DvcSheetBounds *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->bounds;
  return DVC_OK;
}

DvcStatus dvc_engine_get_recalc_mode(const DvcEngine *engine, DvcRecalcMode *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->recalc_mode;
  return DVC_OK;
}

DvcStatus dvc_engine_set_recalc_mode(DvcEngine *engine, DvcRecalcMode mode) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (mode != DVC_RECALC_AUTOMATIC && mode != DVC_RECALC_MANUAL) {
    set_error(engine, DVC_ERR_INVALID_ARGUMENT, "invalid recalc mode");
    return DVC_ERR_INVALID_ARGUMENT;
  }
  engine->recalc_mode = mode;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_engine_committed_epoch(const DvcEngine *engine, uint64_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->committed_epoch;
  return DVC_OK;
}

DvcStatus dvc_engine_stabilized_epoch(const DvcEngine *engine, uint64_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->stabilized_epoch;
  return DVC_OK;
}

DvcStatus dvc_engine_is_stable(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = (engine->committed_epoch == engine->stabilized_epoch) ? 1 : 0;
  return DVC_OK;
}

DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double value) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) {
    set_error(engine, DVC_ERR_OUT_OF_BOUNDS, "cell out of bounds");
    return DVC_ERR_OUT_OF_BOUNDS;
  }
  uint32_t idx = addr_to_index(engine, addr);
  free_cell_input(&engine->cells[idx]);
  engine->cells[idx].kind = DVC_INPUT_NUMBER;
  engine->cells[idx].number = value;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_cell(engine, addr);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_set_text(DvcEngine *engine, DvcCellAddr addr,
                            const char *text, uint32_t text_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!text) {
    set_error(engine, DVC_ERR_NULL_POINTER, "text is null");
    return DVC_ERR_NULL_POINTER;
  }
  if (!valid_addr(engine, addr)) {
    set_error(engine, DVC_ERR_OUT_OF_BOUNDS, "cell out of bounds");
    return DVC_ERR_OUT_OF_BOUNDS;
  }
  uint32_t idx = addr_to_index(engine, addr);
  free_cell_input(&engine->cells[idx]);
  engine->cells[idx].kind = DVC_INPUT_TEXT;
  engine->cells[idx].text = dup_n(text, text_len);
  if (text_len > 0 && !engine->cells[idx].text) {
    set_error(engine, DVC_ERR_OUT_OF_MEMORY, "out of memory");
    return DVC_ERR_OUT_OF_MEMORY;
  }
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_cell(engine, addr);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_set_formula(DvcEngine *engine, DvcCellAddr addr,
                               const char *formula, uint32_t formula_len) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!formula) {
    set_error(engine, DVC_ERR_NULL_POINTER, "formula is null");
    return DVC_ERR_NULL_POINTER;
  }
  if (!valid_addr(engine, addr)) {
    set_error(engine, DVC_ERR_OUT_OF_BOUNDS, "cell out of bounds");
    return DVC_ERR_OUT_OF_BOUNDS;
  }
  if (formula_len == 0) {
    set_error(engine, DVC_ERR_PARSE, "empty formula");
    return DVC_ERR_PARSE;
  }
  uint32_t idx = addr_to_index(engine, addr);
  free_cell_input(&engine->cells[idx]);
  engine->cells[idx].kind = DVC_INPUT_FORMULA;
  engine->cells[idx].text = dup_n(formula, formula_len);
  if (!engine->cells[idx].text) {
    set_error(engine, DVC_ERR_OUT_OF_MEMORY, "out of memory");
    return DVC_ERR_OUT_OF_MEMORY;
  }
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_cell(engine, addr);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_clear(DvcEngine *engine, DvcCellAddr addr) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) {
    set_error(engine, DVC_ERR_OUT_OF_BOUNDS, "cell out of bounds");
    return DVC_ERR_OUT_OF_BOUNDS;
  }
  uint32_t idx = addr_to_index(engine, addr);
  free_cell_input(&engine->cells[idx]);
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_cell(engine, addr);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_get_state(const DvcEngine *engine, DvcCellAddr addr,
                             DvcCellState *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  uint32_t idx = addr_to_index(engine, addr);
  out->value = engine->computed[idx].value;
  out->value_epoch = engine->computed[idx].value_epoch;
  out->stale = (engine->computed[idx].value_epoch < engine->committed_epoch) ? 1 : 0;
  return DVC_OK;
}

DvcStatus dvc_cell_get_text(const DvcEngine *engine, DvcCellAddr addr,
                            char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine || !out_len) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  uint32_t idx = addr_to_index(engine, addr);
  if (engine->computed[idx].value.type != DVC_VALUE_TEXT) {
    *out_len = 0;
    return DVC_OK;
  }
  return copy_out_text(engine->computed[idx].text ? engine->computed[idx].text : "", buf, buf_len, out_len);
}

DvcStatus dvc_cell_get_input_type(const DvcEngine *engine, DvcCellAddr addr,
                                  DvcInputType *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  *out = engine->cells[addr_to_index(engine, addr)].kind;
  return DVC_OK;
}

DvcStatus dvc_cell_get_input_text(const DvcEngine *engine, DvcCellAddr addr,
                                  char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine || !out_len) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  CellInput *ci = &engine->cells[addr_to_index(engine, addr)];
  if (ci->kind == DVC_INPUT_EMPTY) {
    *out_len = 0;
    return DVC_OK;
  }
  if (ci->kind == DVC_INPUT_NUMBER) {
    char tmp[64];
    snprintf(tmp, sizeof(tmp), "%.17g", ci->number);
    return copy_out_text(tmp, buf, buf_len, out_len);
  }
  return copy_out_text(ci->text ? ci->text : "", buf, buf_len, out_len);
}

DvcStatus dvc_cell_set_number_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 double value) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_set_number(engine, addr, value);
}

DvcStatus dvc_cell_set_text_a1(DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               const char *text, uint32_t text_len) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_set_text(engine, addr, text, text_len);
}

DvcStatus dvc_cell_set_formula_a1(DvcEngine *engine,
                                  const char *cell_ref, uint32_t ref_len,
                                  const char *formula, uint32_t formula_len) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_set_formula(engine, addr, formula, formula_len);
}

DvcStatus dvc_cell_clear_a1(DvcEngine *engine,
                            const char *cell_ref, uint32_t ref_len) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_clear(engine, addr);
}

DvcStatus dvc_cell_get_state_a1(const DvcEngine *engine,
                                const char *cell_ref, uint32_t ref_len,
                                DvcCellState *out) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_get_state(engine, addr, out);
}

DvcStatus dvc_cell_get_text_a1(const DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               char *buf, uint32_t buf_len, uint32_t *out_len) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_get_text(engine, addr, buf, buf_len, out_len);
}

DvcStatus dvc_cell_get_input_type_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     DvcInputType *out) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_get_input_type(engine, addr, out);
}

DvcStatus dvc_cell_get_input_text_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_get_input_text(engine, addr, buf, buf_len, out_len);
}

DvcStatus dvc_name_set_number(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              double value) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_conflicts_builtin(u) || name_is_cell_like(u)) {
    free(u);
    set_error(engine, DVC_ERR_INVALID_NAME, "invalid name");
    return DVC_ERR_INVALID_NAME;
  }
  int idx = find_name_index(engine, u);
  if (idx < 0) {
    if (!ensure_name_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    idx = (int)engine->name_count++;
    engine->names[idx].name = u;
    engine->names[idx].text = NULL;
  } else {
    free(u);
    free(engine->names[idx].text);
    engine->names[idx].text = NULL;
  }
  engine->names[idx].kind = DVC_INPUT_NUMBER;
  engine->names[idx].number = value;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_name(engine, engine->names[idx].name);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_name_set_text(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const char *text, uint32_t text_len) {
  if (!engine || !name || !text) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_conflicts_builtin(u) || name_is_cell_like(u)) {
    free(u);
    set_error(engine, DVC_ERR_INVALID_NAME, "invalid name");
    return DVC_ERR_INVALID_NAME;
  }
  int idx = find_name_index(engine, u);
  if (idx < 0) {
    if (!ensure_name_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    idx = (int)engine->name_count++;
    engine->names[idx].name = u;
  } else {
    free(u);
    free(engine->names[idx].text);
  }
  engine->names[idx].kind = DVC_INPUT_TEXT;
  engine->names[idx].text = dup_n(text, text_len);
  engine->names[idx].number = 0.0;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_name(engine, engine->names[idx].name);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_name_set_formula(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               const char *formula, uint32_t formula_len) {
  if (!engine || !name || !formula) return DVC_ERR_NULL_POINTER;
  if (formula_len == 0) return DVC_ERR_PARSE;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_conflicts_builtin(u) || name_is_cell_like(u)) {
    free(u);
    set_error(engine, DVC_ERR_INVALID_NAME, "invalid name");
    return DVC_ERR_INVALID_NAME;
  }
  int idx = find_name_index(engine, u);
  if (idx < 0) {
    if (!ensure_name_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    idx = (int)engine->name_count++;
    engine->names[idx].name = u;
  } else {
    free(u);
    free(engine->names[idx].text);
  }
  engine->names[idx].kind = DVC_INPUT_FORMULA;
  engine->names[idx].text = dup_n(formula, formula_len);
  engine->names[idx].number = 0.0;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  change_push_name(engine, engine->names[idx].name);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_name_clear(DvcEngine *engine,
                         const char *name, uint32_t name_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_name_index(engine, u);
  free(u);
  if (idx < 0) {
    clear_status(engine);
    return DVC_OK;
  }
  free(engine->names[idx].name);
  free(engine->names[idx].text);
  for (uint32_t i = (uint32_t)idx + 1; i < engine->name_count; ++i) {
    engine->names[i - 1] = engine->names[i];
  }
  engine->name_count--;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_name_get_input_type(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  DvcInputType *out) {
  if (!engine || !name || !out) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_name_index(engine, u);
  free(u);
  *out = (idx < 0) ? DVC_INPUT_EMPTY : engine->names[idx].kind;
  return DVC_OK;
}

DvcStatus dvc_name_get_input_text(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len) {
  if (!engine || !name || !out_len) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_name_index(engine, u);
  free(u);
  if (idx < 0) {
    *out_len = 0;
    return DVC_OK;
  }
  NameEntry *ne = &engine->names[idx];
  if (ne->kind == DVC_INPUT_EMPTY) {
    *out_len = 0;
    return DVC_OK;
  }
  if (ne->kind == DVC_INPUT_NUMBER) {
    char tmp[64];
    snprintf(tmp, sizeof(tmp), "%.17g", ne->number);
    return copy_out_text(tmp, buf, buf_len, out_len);
  }
  return copy_out_text(ne->text ? ne->text : "", buf, buf_len, out_len);
}

DvcStatus dvc_recalculate(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  clear_status(engine);
  return do_recalculate(engine);
}

DvcStatus dvc_has_volatile_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = 0;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    if (engine->cells[i].kind == DVC_INPUT_FORMULA && formula_is_volatile(engine->cells[i].text)) {
      *out = 1;
      break;
    }
  }
  return DVC_OK;
}

DvcStatus dvc_has_externally_invalidated_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = 0;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    if (engine->cells[i].kind == DVC_INPUT_FORMULA && formula_is_stream(engine->cells[i].text)) {
      *out = 1;
      break;
    }
  }
  if (!*out) {
    for (uint32_t i = 0; i < engine->udf_count; ++i) {
      if (engine->udfs[i].volatility == DVC_VOLATILITY_EXTERNALLY_INVALIDATED) {
        *out = 1;
        break;
      }
    }
  }
  return DVC_OK;
}

DvcStatus dvc_invalidate_volatile(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_has_stream_cells(const DvcEngine *engine, int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = 0;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    if (engine->stream_active[i] || (engine->cells[i].kind == DVC_INPUT_FORMULA && formula_is_stream(engine->cells[i].text))) {
      *out = 1;
      return DVC_OK;
    }
  }
  return DVC_OK;
}

DvcStatus dvc_tick_streams(DvcEngine *engine, double elapsed_secs,
                           int32_t *any_advanced) {
  if (!engine || !any_advanced) return DVC_ERR_NULL_POINTER;
  if (elapsed_secs < 0.0) return DVC_ERR_INVALID_ARGUMENT;
  *any_advanced = 0;
  for (uint32_t i = 0; i < engine->cell_count; ++i) {
    if (!engine->stream_active[i]) continue;
    double period = engine->stream_period[i];
    if (period <= 0.0) continue;
    engine->stream_elapsed[i] += elapsed_secs;
    while (engine->stream_elapsed[i] >= period) {
      engine->stream_elapsed[i] -= period;
      engine->stream_counter[i] += 1.0;
      *any_advanced = 1;
    }
  }
  if (*any_advanced) {
    engine->committed_epoch++;
    if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  }
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_invalidate_udf(DvcEngine *engine,
                              const char *name, uint32_t name_len) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_udf_index(engine, u);
  free(u);
  if (idx < 0) {
    set_error(engine, DVC_ERR_INVALID_NAME, "udf not registered");
    return DVC_ERR_INVALID_NAME;
  }
  engine->committed_epoch++;
  if (engine->recalc_mode == DVC_RECALC_AUTOMATIC) do_recalculate(engine); else mark_stale(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_get_format(const DvcEngine *engine, DvcCellAddr addr,
                              DvcCellFormat *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  *out = engine->formats[addr_to_index(engine, addr)];
  return DVC_OK;
}

DvcStatus dvc_cell_set_format(DvcEngine *engine, DvcCellAddr addr,
                              const DvcCellFormat *format) {
  if (!engine || !format) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  if (!valid_color(format->fg) || !valid_color(format->bg) || format->decimals > 9) {
    set_error(engine, DVC_ERR_INVALID_ARGUMENT, "invalid format");
    return DVC_ERR_INVALID_ARGUMENT;
  }
  uint32_t idx = addr_to_index(engine, addr);
  DvcCellFormat oldf = engine->formats[idx];
  engine->formats[idx] = *format;
  if (is_default_format(format)) {
    engine->formats[idx] = default_format();
  }
  engine->committed_epoch++;
  change_push_format(engine, addr, &oldf, &engine->formats[idx]);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_cell_get_format_a1(const DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 DvcCellFormat *out) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_get_format(engine, addr, out);
}

DvcStatus dvc_cell_set_format_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 const DvcCellFormat *format) {
  if (!engine || !cell_ref) return DVC_ERR_NULL_POINTER;
  DvcCellAddr addr;
  if (!parse_a1_addr(engine, cell_ref, ref_len, &addr)) return DVC_ERR_INVALID_ADDRESS;
  return dvc_cell_set_format(engine, addr, format);
}

DvcStatus dvc_cell_spill_role(const DvcEngine *engine, DvcCellAddr addr,
                              DvcSpillRole *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  *out = engine->computed[addr_to_index(engine, addr)].spill_role;
  return DVC_OK;
}

DvcStatus dvc_cell_spill_anchor(const DvcEngine *engine, DvcCellAddr addr,
                                DvcCellAddr *out, int32_t *found) {
  if (!engine || !found) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  CellComputed *cc = &engine->computed[addr_to_index(engine, addr)];
  if (!cc->spill_anchor_found) {
    *found = 0;
    return DVC_OK;
  }
  *found = 1;
  if (out) *out = cc->spill_anchor;
  return DVC_OK;
}

DvcStatus dvc_cell_spill_range(const DvcEngine *engine, DvcCellAddr addr,
                               DvcCellRange *out, int32_t *found) {
  if (!engine || !found) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  CellComputed *cc = &engine->computed[addr_to_index(engine, addr)];
  if (!cc->spill_range_found) {
    *found = 0;
    return DVC_OK;
  }
  *found = 1;
  if (out) *out = cc->spill_range;
  return DVC_OK;
}

DvcStatus dvc_cell_iterate(const DvcEngine *engine, DvcCellIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcCellIterator *it = (DvcCellIterator *)calloc(1, sizeof(DvcCellIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->engine = engine;
  it->index = 0;
  it->has_current = 0;
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_cell_iterator_next(DvcCellIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcInputType *input_type,
                                 int32_t *done) {
  if (!iter || !addr || !input_type || !done) return DVC_ERR_NULL_POINTER;
  while (iter->index < iter->engine->cell_count) {
    uint32_t i = iter->index++;
    if (iter->engine->cells[i].kind != DVC_INPUT_EMPTY) {
      iter->current_index = i;
      iter->has_current = 1;
      *addr = index_to_addr(iter->engine, i);
      *input_type = iter->engine->cells[i].kind;
      *done = 0;
      return DVC_OK;
    }
  }
  iter->has_current = 0;
  *done = 1;
  return DVC_OK;
}

DvcStatus dvc_cell_iterator_get_text(const DvcCellIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len) {
  if (!iter || !out_len) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current) { *out_len = 0; return DVC_OK; }
  CellInput *ci = &iter->engine->cells[iter->current_index];
  if (ci->kind == DVC_INPUT_EMPTY) { *out_len = 0; return DVC_OK; }
  if (ci->kind == DVC_INPUT_NUMBER) {
    char tmp[64];
    snprintf(tmp, sizeof(tmp), "%.17g", ci->number);
    return copy_out_text(tmp, buf, buf_len, out_len);
  }
  return copy_out_text(ci->text ? ci->text : "", buf, buf_len, out_len);
}

DvcStatus dvc_cell_iterator_destroy(DvcCellIterator *iter) {
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_name_iterate(const DvcEngine *engine, DvcNameIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcNameIterator *it = (DvcNameIterator *)calloc(1, sizeof(DvcNameIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->engine = engine;
  it->count = engine->name_count;
  it->order = (uint32_t *)calloc(it->count ? it->count : 1u, sizeof(uint32_t));
  if (it->count && !it->order) { free(it); return DVC_ERR_OUT_OF_MEMORY; }
  for (uint32_t i = 0; i < it->count; ++i) it->order[i] = i;
  for (uint32_t i = 0; i < it->count; ++i) {
    for (uint32_t j = i + 1; j < it->count; ++j) {
      if (cmp_ci(engine->names[it->order[i]].name, engine->names[it->order[j]].name) > 0) {
        uint32_t tmp = it->order[i];
        it->order[i] = it->order[j];
        it->order[j] = tmp;
      }
    }
  }
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_name_iterator_next(DvcNameIterator *iter,
                                 char *name_buf, uint32_t name_buf_len,
                                 uint32_t *name_len,
                                 DvcInputType *input_type,
                                 int32_t *done) {
  if (!iter || !name_len || !input_type || !done) return DVC_ERR_NULL_POINTER;
  if (iter->index >= iter->count) {
    iter->has_current = 0;
    *done = 1;
    return DVC_OK;
  }
  iter->current_index = iter->order[iter->index++];
  iter->has_current = 1;
  NameEntry *ne = &iter->engine->names[iter->current_index];
  *input_type = ne->kind;
  *done = 0;
  DvcStatus s = copy_out_text(ne->name, name_buf, name_buf_len, name_len);
  return s;
}

DvcStatus dvc_name_iterator_get_text(const DvcNameIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len) {
  if (!iter || !out_len) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current) { *out_len = 0; return DVC_OK; }
  NameEntry *ne = &iter->engine->names[iter->current_index];
  if (ne->kind == DVC_INPUT_EMPTY) { *out_len = 0; return DVC_OK; }
  if (ne->kind == DVC_INPUT_NUMBER) {
    char tmp[64];
    snprintf(tmp, sizeof(tmp), "%.17g", ne->number);
    return copy_out_text(tmp, buf, buf_len, out_len);
  }
  return copy_out_text(ne->text ? ne->text : "", buf, buf_len, out_len);
}

DvcStatus dvc_name_iterator_destroy(DvcNameIterator *iter) {
  if (!iter) return DVC_OK;
  free(iter->order);
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_format_iterate(const DvcEngine *engine, DvcFormatIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcFormatIterator *it = (DvcFormatIterator *)calloc(1, sizeof(DvcFormatIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->engine = engine;
  it->index = 0;
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_format_iterator_next(DvcFormatIterator *iter,
                                   DvcCellAddr *addr,
                                   DvcCellFormat *format,
                                   int32_t *done) {
  if (!iter || !addr || !format || !done) return DVC_ERR_NULL_POINTER;
  while (iter->index < iter->engine->cell_count) {
    uint32_t i = iter->index++;
    if (!is_default_format(&iter->engine->formats[i])) {
      *addr = index_to_addr(iter->engine, i);
      *format = iter->engine->formats[i];
      *done = 0;
      return DVC_OK;
    }
  }
  *done = 1;
  return DVC_OK;
}

DvcStatus dvc_format_iterator_destroy(DvcFormatIterator *iter) {
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_insert_row(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (at == 0 || at > engine->bounds.max_rows) return DVC_ERR_OUT_OF_BOUNDS;
  DvcCellAddr blocked_cell = {0, 0};
  DvcCellRange blocked_range;
  memset(&blocked_range, 0, sizeof(blocked_range));
  if (structural_find_spill_constraint(engine, DVC_STRUCT_OP_INSERT_ROW, at, &blocked_cell, &blocked_range)) {
    set_reject(engine, DVC_REJECT_STRUCTURAL_CONSTRAINT, DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT, DVC_STRUCT_OP_INSERT_ROW, at);
    engine->last_reject_context.has_cell = 1;
    engine->last_reject_context.cell = blocked_cell;
    engine->last_reject_context.has_range = 1;
    engine->last_reject_context.range = blocked_range;
    return DVC_REJECT_STRUCTURAL_CONSTRAINT;
  }
  for (uint16_t c = 1; c <= engine->bounds.max_columns; ++c) {
    DvcCellAddr a = {c, at};
    if (engine->cells[addr_to_index(engine, a)].kind != DVC_INPUT_EMPTY) {
      break;
    }
  }
  uint16_t cols = engine->bounds.max_columns;
  uint16_t rows = engine->bounds.max_rows;
  for (int r = rows; r >= (int)at + 1; --r) {
    for (int c = 1; c <= cols; ++c) {
      DvcCellAddr dst = {(uint16_t)c, (uint16_t)r};
      DvcCellAddr src = {(uint16_t)c, (uint16_t)(r - 1)};
      uint32_t didx = addr_to_index(engine, dst);
      uint32_t sidx = addr_to_index(engine, src);
      free_cell_input(&engine->cells[didx]);
      engine->cells[didx] = engine->cells[sidx];
      engine->cells[sidx].kind = DVC_INPUT_EMPTY;
      engine->cells[sidx].text = NULL;
      engine->cells[sidx].number = 0.0;
      free_cell_computed(&engine->computed[didx]);
      engine->computed[didx] = engine->computed[sidx];
      memset(&engine->computed[sidx], 0, sizeof(CellComputed));
      engine->computed[sidx].value.type = DVC_VALUE_BLANK;
      engine->formats[didx] = engine->formats[sidx];
      engine->formats[sidx] = default_format();
      engine->stream_active[didx] = engine->stream_active[sidx];
      engine->stream_period[didx] = engine->stream_period[sidx];
      engine->stream_elapsed[didx] = engine->stream_elapsed[sidx];
      engine->stream_counter[didx] = engine->stream_counter[sidx];
      engine->stream_active[sidx] = 0;
      engine->stream_period[sidx] = 0.0;
      engine->stream_elapsed[sidx] = 0.0;
      engine->stream_counter[sidx] = 0.0;
    }
  }
  for (int c = 1; c <= cols; ++c) {
    DvcCellAddr a = {(uint16_t)c, at};
    uint32_t idx = addr_to_index(engine, a);
    free_cell_input(&engine->cells[idx]);
    set_computed_blank(&engine->computed[idx], engine->committed_epoch + 1);
    engine->formats[idx] = default_format();
    engine->stream_active[idx] = 0;
    engine->stream_period[idx] = 0.0;
    engine->stream_elapsed[idx] = 0.0;
    engine->stream_counter[idx] = 0.0;
  }
  rewrite_formulas_for_structural_op(engine, DVC_STRUCT_OP_INSERT_ROW, at);
  engine->committed_epoch++;
  do_recalculate(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_delete_row(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (at == 0 || at > engine->bounds.max_rows) return DVC_ERR_OUT_OF_BOUNDS;
  DvcCellAddr blocked_cell = {0, 0};
  DvcCellRange blocked_range;
  memset(&blocked_range, 0, sizeof(blocked_range));
  if (structural_find_spill_constraint(engine, DVC_STRUCT_OP_DELETE_ROW, at, &blocked_cell, &blocked_range)) {
    set_reject(engine, DVC_REJECT_STRUCTURAL_CONSTRAINT, DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT, DVC_STRUCT_OP_DELETE_ROW, at);
    engine->last_reject_context.has_cell = 1;
    engine->last_reject_context.cell = blocked_cell;
    engine->last_reject_context.has_range = 1;
    engine->last_reject_context.range = blocked_range;
    return DVC_REJECT_STRUCTURAL_CONSTRAINT;
  }
  uint16_t cols = engine->bounds.max_columns;
  uint16_t rows = engine->bounds.max_rows;
  for (uint16_t r = at; r < rows; ++r) {
    for (uint16_t c = 1; c <= cols; ++c) {
      DvcCellAddr dst = {c, r};
      DvcCellAddr src = {c, (uint16_t)(r + 1)};
      uint32_t didx = addr_to_index(engine, dst);
      uint32_t sidx = addr_to_index(engine, src);
      free_cell_input(&engine->cells[didx]);
      engine->cells[didx] = engine->cells[sidx];
      engine->cells[sidx].kind = DVC_INPUT_EMPTY;
      engine->cells[sidx].text = NULL;
      engine->cells[sidx].number = 0.0;
      free_cell_computed(&engine->computed[didx]);
      engine->computed[didx] = engine->computed[sidx];
      memset(&engine->computed[sidx], 0, sizeof(CellComputed));
      engine->computed[sidx].value.type = DVC_VALUE_BLANK;
      engine->formats[didx] = engine->formats[sidx];
      engine->formats[sidx] = default_format();
      engine->stream_active[didx] = engine->stream_active[sidx];
      engine->stream_period[didx] = engine->stream_period[sidx];
      engine->stream_elapsed[didx] = engine->stream_elapsed[sidx];
      engine->stream_counter[didx] = engine->stream_counter[sidx];
      engine->stream_active[sidx] = 0;
      engine->stream_period[sidx] = 0.0;
      engine->stream_elapsed[sidx] = 0.0;
      engine->stream_counter[sidx] = 0.0;
    }
  }
  for (uint16_t c = 1; c <= cols; ++c) {
    DvcCellAddr a = {c, rows};
    uint32_t idx = addr_to_index(engine, a);
    free_cell_input(&engine->cells[idx]);
    set_computed_blank(&engine->computed[idx], engine->committed_epoch + 1);
    engine->formats[idx] = default_format();
    engine->stream_active[idx] = 0;
    engine->stream_period[idx] = 0.0;
    engine->stream_elapsed[idx] = 0.0;
    engine->stream_counter[idx] = 0.0;
  }
  rewrite_formulas_for_structural_op(engine, DVC_STRUCT_OP_DELETE_ROW, at);
  engine->committed_epoch++;
  do_recalculate(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_insert_col(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (at == 0 || at > engine->bounds.max_columns) return DVC_ERR_OUT_OF_BOUNDS;
  DvcCellAddr blocked_cell = {0, 0};
  DvcCellRange blocked_range;
  memset(&blocked_range, 0, sizeof(blocked_range));
  if (structural_find_spill_constraint(engine, DVC_STRUCT_OP_INSERT_COL, at, &blocked_cell, &blocked_range)) {
    set_reject(engine, DVC_REJECT_STRUCTURAL_CONSTRAINT, DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT, DVC_STRUCT_OP_INSERT_COL, at);
    engine->last_reject_context.has_cell = 1;
    engine->last_reject_context.cell = blocked_cell;
    engine->last_reject_context.has_range = 1;
    engine->last_reject_context.range = blocked_range;
    return DVC_REJECT_STRUCTURAL_CONSTRAINT;
  }
  uint16_t cols = engine->bounds.max_columns;
  uint16_t rows = engine->bounds.max_rows;
  for (uint16_t r = 1; r <= rows; ++r) {
    for (int c = cols; c >= (int)at + 1; --c) {
      DvcCellAddr dst = {(uint16_t)c, r};
      DvcCellAddr src = {(uint16_t)(c - 1), r};
      uint32_t didx = addr_to_index(engine, dst);
      uint32_t sidx = addr_to_index(engine, src);
      free_cell_input(&engine->cells[didx]);
      engine->cells[didx] = engine->cells[sidx];
      engine->cells[sidx].kind = DVC_INPUT_EMPTY;
      engine->cells[sidx].text = NULL;
      engine->cells[sidx].number = 0.0;
      free_cell_computed(&engine->computed[didx]);
      engine->computed[didx] = engine->computed[sidx];
      memset(&engine->computed[sidx], 0, sizeof(CellComputed));
      engine->computed[sidx].value.type = DVC_VALUE_BLANK;
      engine->formats[didx] = engine->formats[sidx];
      engine->formats[sidx] = default_format();
      engine->stream_active[didx] = engine->stream_active[sidx];
      engine->stream_period[didx] = engine->stream_period[sidx];
      engine->stream_elapsed[didx] = engine->stream_elapsed[sidx];
      engine->stream_counter[didx] = engine->stream_counter[sidx];
      engine->stream_active[sidx] = 0;
      engine->stream_period[sidx] = 0.0;
      engine->stream_elapsed[sidx] = 0.0;
      engine->stream_counter[sidx] = 0.0;
    }
    DvcCellAddr a = {at, r};
    uint32_t idx = addr_to_index(engine, a);
    free_cell_input(&engine->cells[idx]);
    set_computed_blank(&engine->computed[idx], engine->committed_epoch + 1);
    engine->formats[idx] = default_format();
    engine->stream_active[idx] = 0;
    engine->stream_period[idx] = 0.0;
    engine->stream_elapsed[idx] = 0.0;
    engine->stream_counter[idx] = 0.0;
  }
  rewrite_formulas_for_structural_op(engine, DVC_STRUCT_OP_INSERT_COL, at);
  engine->committed_epoch++;
  do_recalculate(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_delete_col(DvcEngine *engine, uint16_t at) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  if (at == 0 || at > engine->bounds.max_columns) return DVC_ERR_OUT_OF_BOUNDS;
  DvcCellAddr blocked_cell = {0, 0};
  DvcCellRange blocked_range;
  memset(&blocked_range, 0, sizeof(blocked_range));
  if (structural_find_spill_constraint(engine, DVC_STRUCT_OP_DELETE_COL, at, &blocked_cell, &blocked_range)) {
    set_reject(engine, DVC_REJECT_STRUCTURAL_CONSTRAINT, DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT, DVC_STRUCT_OP_DELETE_COL, at);
    engine->last_reject_context.has_cell = 1;
    engine->last_reject_context.cell = blocked_cell;
    engine->last_reject_context.has_range = 1;
    engine->last_reject_context.range = blocked_range;
    return DVC_REJECT_STRUCTURAL_CONSTRAINT;
  }
  uint16_t cols = engine->bounds.max_columns;
  uint16_t rows = engine->bounds.max_rows;
  for (uint16_t r = 1; r <= rows; ++r) {
    for (uint16_t c = at; c < cols; ++c) {
      DvcCellAddr dst = {c, r};
      DvcCellAddr src = {(uint16_t)(c + 1), r};
      uint32_t didx = addr_to_index(engine, dst);
      uint32_t sidx = addr_to_index(engine, src);
      free_cell_input(&engine->cells[didx]);
      engine->cells[didx] = engine->cells[sidx];
      engine->cells[sidx].kind = DVC_INPUT_EMPTY;
      engine->cells[sidx].text = NULL;
      engine->cells[sidx].number = 0.0;
      free_cell_computed(&engine->computed[didx]);
      engine->computed[didx] = engine->computed[sidx];
      memset(&engine->computed[sidx], 0, sizeof(CellComputed));
      engine->computed[sidx].value.type = DVC_VALUE_BLANK;
      engine->formats[didx] = engine->formats[sidx];
      engine->formats[sidx] = default_format();
      engine->stream_active[didx] = engine->stream_active[sidx];
      engine->stream_period[didx] = engine->stream_period[sidx];
      engine->stream_elapsed[didx] = engine->stream_elapsed[sidx];
      engine->stream_counter[didx] = engine->stream_counter[sidx];
      engine->stream_active[sidx] = 0;
      engine->stream_period[sidx] = 0.0;
      engine->stream_elapsed[sidx] = 0.0;
      engine->stream_counter[sidx] = 0.0;
    }
    DvcCellAddr a = {cols, r};
    uint32_t idx = addr_to_index(engine, a);
    free_cell_input(&engine->cells[idx]);
    set_computed_blank(&engine->computed[idx], engine->committed_epoch + 1);
    engine->formats[idx] = default_format();
    engine->stream_active[idx] = 0;
    engine->stream_period[idx] = 0.0;
    engine->stream_elapsed[idx] = 0.0;
    engine->stream_counter[idx] = 0.0;
  }
  rewrite_formulas_for_structural_op(engine, DVC_STRUCT_OP_DELETE_COL, at);
  engine->committed_epoch++;
  do_recalculate(engine);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_engine_get_iteration_config(const DvcEngine *engine,
                                           DvcIterationConfig *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->iter_cfg;
  return DVC_OK;
}

DvcStatus dvc_engine_set_iteration_config(DvcEngine *engine,
                                           const DvcIterationConfig *config) {
  if (!engine || !config) return DVC_ERR_NULL_POINTER;
  if (config->max_iterations == 0 || config->convergence_tolerance < 0.0) {
    set_error(engine, DVC_ERR_INVALID_ARGUMENT, "invalid iteration config");
    return DVC_ERR_INVALID_ARGUMENT;
  }
  engine->iter_cfg = *config;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_control_define(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              const DvcControlDef *def) {
  if (!engine || !name || !def) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_conflicts_builtin(u) || name_is_cell_like(u)) {
    free(u);
    return DVC_ERR_INVALID_NAME;
  }
  if (def->kind == DVC_CONTROL_SLIDER && (def->min > def->max || def->step <= 0.0)) {
    free(u);
    return DVC_ERR_INVALID_ARGUMENT;
  }
  int cidx = find_control_index(engine, u);
  if (cidx < 0) {
    if (!ensure_control_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    cidx = (int)engine->control_count++;
    engine->controls[cidx].name = u;
  } else {
    free(u);
  }
  engine->controls[cidx].def = *def;
  if (def->kind == DVC_CONTROL_SLIDER) engine->controls[cidx].value = def->min;
  else engine->controls[cidx].value = 0.0;
  dvc_name_set_number(engine, engine->controls[cidx].name, (uint32_t)strlen(engine->controls[cidx].name), engine->controls[cidx].value);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_control_remove(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_control_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; return DVC_OK; }
  *found = 1;
  free(engine->controls[idx].name);
  for (uint32_t i = (uint32_t)idx + 1; i < engine->control_count; ++i) {
    engine->controls[i - 1] = engine->controls[i];
  }
  engine->control_count--;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_control_set_value(DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double value) {
  if (!engine || !name) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_control_index(engine, u);
  free(u);
  if (idx < 0) return DVC_ERR_INVALID_NAME;
  ControlEntry *ce = &engine->controls[idx];
  if (ce->def.kind == DVC_CONTROL_CHECKBOX) {
    if (!(value == 0.0 || value == 1.0)) return DVC_ERR_INVALID_ARGUMENT;
    ce->value = value;
  } else if (ce->def.kind == DVC_CONTROL_SLIDER) {
    if (value < ce->def.min) value = ce->def.min;
    if (value > ce->def.max) value = ce->def.max;
    ce->value = value;
  } else {
    ce->value = 0.0;
  }
  dvc_name_set_number(engine, ce->name, (uint32_t)strlen(ce->name), ce->value);
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_control_get_value(const DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double *out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_control_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; return DVC_OK; }
  *found = 1;
  *out = engine->controls[idx].value;
  return DVC_OK;
}

DvcStatus dvc_control_get_def(const DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               DvcControlDef *out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_control_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; return DVC_OK; }
  *found = 1;
  *out = engine->controls[idx].def;
  return DVC_OK;
}

DvcStatus dvc_control_iterate(const DvcEngine *engine,
                               DvcControlIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcControlIterator *it = (DvcControlIterator *)calloc(1, sizeof(DvcControlIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->engine = engine;
  it->count = engine->control_count;
  it->order = (uint32_t *)calloc(it->count ? it->count : 1u, sizeof(uint32_t));
  if (it->count && !it->order) { free(it); return DVC_ERR_OUT_OF_MEMORY; }
  for (uint32_t i = 0; i < it->count; ++i) it->order[i] = i;
  for (uint32_t i = 0; i < it->count; ++i) {
    for (uint32_t j = i + 1; j < it->count; ++j) {
      if (cmp_ci(engine->controls[it->order[i]].name, engine->controls[it->order[j]].name) > 0) {
        uint32_t tmp = it->order[i]; it->order[i] = it->order[j]; it->order[j] = tmp;
      }
    }
  }
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_control_iterator_next(DvcControlIterator *iter,
                                     char *name_buf, uint32_t name_buf_len,
                                     uint32_t *name_len,
                                     DvcControlDef *def,
                                     double *value,
                                     int32_t *done) {
  if (!iter || !name_len || !def || !value || !done) return DVC_ERR_NULL_POINTER;
  if (iter->index >= iter->count) { *done = 1; return DVC_OK; }
  uint32_t idx = iter->order[iter->index];
  ControlEntry *ce = &iter->engine->controls[idx];
  uint32_t n = ce->name ? (uint32_t)strlen(ce->name) : 0u;
  *name_len = n;
  if (name_buf && name_buf_len < n) return DVC_ERR_INVALID_ARGUMENT;
  *def = ce->def;
  *value = ce->value;
  *done = 0;
  if (!name_buf) return DVC_OK;
  DvcStatus s = copy_out_text(ce->name, name_buf, name_buf_len, name_len);
  if (s != DVC_OK) return s;
  iter->index++;
  return DVC_OK;
}

DvcStatus dvc_control_iterator_destroy(DvcControlIterator *iter) {
  if (!iter) return DVC_OK;
  free(iter->order);
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_chart_define(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const DvcChartDef *def) {
  if (!engine || !name || !def) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, def->source_range.start) || !valid_addr(engine, def->source_range.end)) {
    return DVC_ERR_OUT_OF_BOUNDS;
  }
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_is_cell_like(u)) {
    free(u);
    return DVC_ERR_INVALID_NAME;
  }
  int idx = find_chart_index(engine, u);
  if (idx < 0) {
    if (!ensure_chart_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    idx = (int)engine->chart_count++;
    engine->charts[idx].name = u;
  } else {
    free(u);
  }
  engine->charts[idx].def = *def;
  engine->committed_epoch++;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_chart_remove(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_chart_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; return DVC_OK; }
  *found = 1;
  free(engine->charts[idx].name);
  for (uint32_t i = (uint32_t)idx + 1; i < engine->chart_count; ++i) engine->charts[i - 1] = engine->charts[i];
  engine->chart_count--;
  engine->committed_epoch++;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_chart_get_output(const DvcEngine *engine,
                                const char *name, uint32_t name_len,
                                DvcChartOutput **out, int32_t *found) {
  if (!engine || !name || !out || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_chart_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; *out = NULL; return DVC_OK; }
  *found = 1;
  DvcChartDef def = engine->charts[idx].def;
  uint16_t r1 = def.source_range.start.row, r2 = def.source_range.end.row;
  uint16_t c1 = def.source_range.start.col, c2 = def.source_range.end.col;
  if (r1 > r2) { uint16_t t2=r1;r1=r2;r2=t2; }
  if (c1 > c2) { uint16_t t2=c1;c1=c2;c2=t2; }

  DvcChartOutput *co = (DvcChartOutput *)calloc(1, sizeof(DvcChartOutput));
  if (!co) return DVC_ERR_OUT_OF_MEMORY;
  co->series_count = (uint32_t)(c2 - c1 + 1);
  co->label_count = (uint32_t)(r2 - r1 + 1);
  co->labels = (char **)calloc(co->label_count ? co->label_count : 1u, sizeof(char *));
  co->series_names = (char **)calloc(co->series_count ? co->series_count : 1u, sizeof(char *));
  co->series_values = (double **)calloc(co->series_count ? co->series_count : 1u, sizeof(double *));
  co->series_value_counts = (uint32_t *)calloc(co->series_count ? co->series_count : 1u, sizeof(uint32_t));
  if (!co->labels || !co->series_names || !co->series_values || !co->series_value_counts) {
    free(co->labels); free(co->series_names); free(co->series_values); free(co->series_value_counts); free(co);
    return DVC_ERR_OUT_OF_MEMORY;
  }
  for (uint32_t r = 0; r < co->label_count; ++r) {
    char tmp[32];
    snprintf(tmp, sizeof(tmp), "R%u", (unsigned)(r1 + r));
    co->labels[r] = dup_cstr(tmp);
  }
  for (uint32_t c = 0; c < co->series_count; ++c) {
    char tmp[32];
    snprintf(tmp, sizeof(tmp), "C%u", (unsigned)(c1 + c));
    co->series_names[c] = dup_cstr(tmp);
    co->series_value_counts[c] = co->label_count;
    co->series_values[c] = (double *)calloc(co->label_count ? co->label_count : 1u, sizeof(double));
    for (uint32_t r = 0; r < co->label_count; ++r) {
      DvcCellAddr a = {(uint16_t)(c1 + c), (uint16_t)(r1 + r)};
      co->series_values[c][r] = get_cell_number_or_zero(engine, a);
    }
  }
  *out = co;
  return DVC_OK;
}

DvcStatus dvc_chart_output_series_count(const DvcChartOutput *output,
                                         uint32_t *out) {
  if (!output || !out) return DVC_ERR_NULL_POINTER;
  *out = output->series_count;
  return DVC_OK;
}

DvcStatus dvc_chart_output_label_count(const DvcChartOutput *output,
                                        uint32_t *out) {
  if (!output || !out) return DVC_ERR_NULL_POINTER;
  *out = output->label_count;
  return DVC_OK;
}

DvcStatus dvc_chart_output_label(const DvcChartOutput *output,
                                  uint32_t index,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len) {
  if (!output || !out_len) return DVC_ERR_NULL_POINTER;
  if (index >= output->label_count) return DVC_ERR_OUT_OF_BOUNDS;
  return copy_out_text(output->labels[index], buf, buf_len, out_len);
}

DvcStatus dvc_chart_output_series_name(const DvcChartOutput *output,
                                        uint32_t series_index,
                                        char *buf, uint32_t buf_len,
                                        uint32_t *out_len) {
  if (!output || !out_len) return DVC_ERR_NULL_POINTER;
  if (series_index >= output->series_count) return DVC_ERR_OUT_OF_BOUNDS;
  return copy_out_text(output->series_names[series_index], buf, buf_len, out_len);
}

DvcStatus dvc_chart_output_series_values(const DvcChartOutput *output,
                                          uint32_t series_index,
                                          double *buf, uint32_t buf_len,
                                          uint32_t *out_count) {
  if (!output || !out_count) return DVC_ERR_NULL_POINTER;
  if (series_index >= output->series_count) return DVC_ERR_OUT_OF_BOUNDS;
  uint32_t n = output->series_value_counts[series_index];
  *out_count = n;
  if (buf && buf_len > 0) {
    uint32_t copy_n = (buf_len < n) ? buf_len : n;
    memcpy(buf, output->series_values[series_index], sizeof(double) * copy_n);
  }
  return DVC_OK;
}

DvcStatus dvc_chart_iterate(const DvcEngine *engine,
                             DvcChartIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcChartIterator *it = (DvcChartIterator *)calloc(1, sizeof(DvcChartIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->engine = engine;
  it->count = engine->chart_count;
  it->order = (uint32_t *)calloc(it->count ? it->count : 1u, sizeof(uint32_t));
  if (it->count && !it->order) { free(it); return DVC_ERR_OUT_OF_MEMORY; }
  for (uint32_t i = 0; i < it->count; ++i) it->order[i] = i;
  for (uint32_t i = 0; i < it->count; ++i) for (uint32_t j = i + 1; j < it->count; ++j) if (cmp_ci(engine->charts[it->order[i]].name, engine->charts[it->order[j]].name) > 0) { uint32_t tmp=it->order[i]; it->order[i]=it->order[j]; it->order[j]=tmp; }
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_chart_iterator_next(DvcChartIterator *iter,
                                   char *name_buf, uint32_t name_buf_len,
                                   uint32_t *name_len,
                                   DvcChartDef *def,
                                   int32_t *done) {
  if (!iter || !name_len || !def || !done) return DVC_ERR_NULL_POINTER;
  if (iter->index >= iter->count) { *done = 1; return DVC_OK; }
  uint32_t idx = iter->order[iter->index];
  ChartEntry *ce = &iter->engine->charts[idx];
  uint32_t n = ce->name ? (uint32_t)strlen(ce->name) : 0u;
  *name_len = n;
  if (name_buf && name_buf_len < n) return DVC_ERR_INVALID_ARGUMENT;
  *def = ce->def;
  *done = 0;
  if (!name_buf) return DVC_OK;
  DvcStatus s = copy_out_text(ce->name, name_buf, name_buf_len, name_len);
  if (s != DVC_OK) return s;
  iter->index++;
  return DVC_OK;
}

DvcStatus dvc_chart_iterator_destroy(DvcChartIterator *iter) {
  if (!iter) return DVC_OK;
  free(iter->order);
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_udf_register(DvcEngine *engine,
                             const char *name, uint32_t name_len,
                             DvcUdfCallback callback,
                             void *user_data,
                             DvcVolatility volatility) {
  if (!engine || !name || !callback) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  if (!name_is_valid_identifier(u) || name_is_cell_like(u)) { free(u); return DVC_ERR_INVALID_NAME; }
  int idx = find_udf_index(engine, u);
  if (idx < 0) {
    if (!ensure_udf_cap(engine)) { free(u); return DVC_ERR_OUT_OF_MEMORY; }
    idx = (int)engine->udf_count++;
    engine->udfs[idx].name = u;
  } else {
    free(u);
  }
  engine->udfs[idx].callback = callback;
  engine->udfs[idx].user_data = user_data;
  engine->udfs[idx].volatility = volatility;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_udf_unregister(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               int32_t *found) {
  if (!engine || !name || !found) return DVC_ERR_NULL_POINTER;
  char *u = upper_name(name, name_len);
  if (!u) return DVC_ERR_OUT_OF_MEMORY;
  int idx = find_udf_index(engine, u);
  free(u);
  if (idx < 0) { *found = 0; return DVC_OK; }
  *found = 1;
  free(engine->udfs[idx].name);
  for (uint32_t i = (uint32_t)idx + 1; i < engine->udf_count; ++i) engine->udfs[i - 1] = engine->udfs[i];
  engine->udf_count--;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_change_tracking_enable(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  engine->change_tracking_enabled = 1;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_change_tracking_disable(DvcEngine *engine) {
  if (!engine) return DVC_ERR_NULL_POINTER;
  engine->change_tracking_enabled = 0;
  for (uint32_t i = 0; i < engine->change_count; ++i) {
    free(engine->changes[i].name);
    free(engine->changes[i].chart_name);
    free(engine->changes[i].diag_message);
  }
  engine->change_count = 0;
  clear_status(engine);
  return DVC_OK;
}

DvcStatus dvc_change_tracking_is_enabled(const DvcEngine *engine,
                                          int32_t *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->change_tracking_enabled ? 1 : 0;
  return DVC_OK;
}

DvcStatus dvc_change_iterate(DvcEngine *engine, DvcChangeIterator **out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  DvcChangeIterator *it = (DvcChangeIterator *)calloc(1, sizeof(DvcChangeIterator));
  if (!it) return DVC_ERR_OUT_OF_MEMORY;
  it->count = engine->change_count;
  it->items = engine->changes;
  it->index = 0;
  it->has_current = 0;
  engine->changes = NULL;
  engine->change_count = 0;
  engine->change_cap = 0;
  *out = it;
  return DVC_OK;
}

DvcStatus dvc_change_iterator_next(DvcChangeIterator *iter,
                                    DvcChangeType *change_type,
                                    uint64_t *epoch,
                                    int32_t *done) {
  if (!iter || !change_type || !epoch || !done) return DVC_ERR_NULL_POINTER;
  if (iter->index >= iter->count) {
    iter->has_current = 0;
    *done = 1;
    return DVC_OK;
  }
  iter->has_current = 1;
  ChangeEntry *ce = &iter->items[iter->index++];
  *change_type = ce->type;
  *epoch = ce->epoch;
  *done = 0;
  return DVC_OK;
}

DvcStatus dvc_change_get_cell(const DvcChangeIterator *iter,
                               DvcCellAddr *addr) {
  if (!iter || !addr) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  *addr = iter->items[iter->index - 1].cell;
  return DVC_OK;
}

DvcStatus dvc_change_get_name(const DvcChangeIterator *iter,
                               char *buf, uint32_t buf_len,
                               uint32_t *out_len) {
  if (!iter || !out_len) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  return copy_out_text(iter->items[iter->index - 1].name ? iter->items[iter->index - 1].name : "", buf, buf_len, out_len);
}

DvcStatus dvc_change_get_chart_name(const DvcChangeIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len) {
  if (!iter || !out_len) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  return copy_out_text(iter->items[iter->index - 1].chart_name ? iter->items[iter->index - 1].chart_name : "", buf, buf_len, out_len);
}

DvcStatus dvc_change_get_spill(const DvcChangeIterator *iter,
                                DvcCellAddr *anchor,
                                DvcCellRange *old_range, int32_t *had_old,
                                DvcCellRange *new_range, int32_t *has_new) {
  if (!iter || !anchor || !old_range || !had_old || !new_range || !has_new) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  ChangeEntry *ce = &iter->items[iter->index - 1];
  *anchor = ce->cell;
  *old_range = ce->old_spill;
  *new_range = ce->new_spill;
  *had_old = ce->had_old;
  *has_new = ce->has_new;
  return DVC_OK;
}

DvcStatus dvc_change_get_format(const DvcChangeIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcCellFormat *old_fmt,
                                 DvcCellFormat *new_fmt) {
  if (!iter || !addr || !old_fmt || !new_fmt) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  ChangeEntry *ce = &iter->items[iter->index - 1];
  *addr = ce->cell;
  *old_fmt = ce->old_fmt;
  *new_fmt = ce->new_fmt;
  return DVC_OK;
}

DvcStatus dvc_change_get_diagnostic(const DvcChangeIterator *iter,
                                     DvcDiagnosticCode *code,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len) {
  if (!iter || !code || !out_len) return DVC_ERR_NULL_POINTER;
  if (!iter->has_current || iter->index == 0) return DVC_ERR_INVALID_ARGUMENT;
  ChangeEntry *ce = &iter->items[iter->index - 1];
  *code = ce->diag_code;
  return copy_out_text(ce->diag_message ? ce->diag_message : "", buf, buf_len, out_len);
}

DvcStatus dvc_change_iterator_destroy(DvcChangeIterator *iter) {
  if (!iter) return DVC_OK;
  for (uint32_t i = 0; i < iter->count; ++i) {
    free(iter->items[i].name);
    free(iter->items[i].chart_name);
    free(iter->items[i].diag_message);
  }
  free(iter->items);
  free(iter);
  return DVC_OK;
}

DvcStatus dvc_last_error_message(const DvcEngine *engine,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len) {
  if (!engine || !out_len) return DVC_ERR_NULL_POINTER;
  return copy_out_text(engine->last_error_message ? engine->last_error_message : "", buf, buf_len, out_len);
}

DvcStatus dvc_last_error_kind(const DvcEngine *engine, DvcStatus *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->last_error_kind;
  return DVC_OK;
}

DvcStatus dvc_last_reject_kind(const DvcEngine *engine, DvcRejectKind *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->last_reject_kind;
  return DVC_OK;
}

DvcStatus dvc_last_reject_context(const DvcEngine *engine,
                                  DvcLastRejectContext *out) {
  if (!engine || !out) return DVC_ERR_NULL_POINTER;
  *out = engine->last_reject_context;
  return DVC_OK;
}

DvcStatus dvc_cell_error_message(const DvcEngine *engine, DvcCellAddr addr,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len) {
  if (!engine || !out_len) return DVC_ERR_NULL_POINTER;
  if (!valid_addr(engine, addr)) return DVC_ERR_OUT_OF_BOUNDS;
  CellComputed *cc = &engine->computed[addr_to_index(engine, addr)];
  if (cc->value.type != DVC_VALUE_ERROR) {
    *out_len = 0;
    return DVC_OK;
  }
  return copy_out_text(cc->error_message ? cc->error_message : "error", buf, buf_len, out_len);
}

DvcStatus dvc_palette_color_name(DvcPaletteColor color,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len) {
  static const char *names[] = {
    "MIST","SAGE","FERN","MOSS","OLIVE","SEAFOAM","LAGOON","TEAL",
    "SKY","CLOUD","SAND","CLAY","PEACH","ROSE","LAVENDER","SLATE"
  };
  if (!out_len) return DVC_ERR_NULL_POINTER;
  if (color < 0 || color >= DVC_PALETTE_COUNT) {
    *out_len = 0;
    return DVC_ERR_INVALID_ARGUMENT;
  }
  return copy_out_text(names[color], buf, buf_len, out_len);
}

DvcStatus dvc_parse_cell_ref(const DvcEngine *engine,
                             const char *ref_str, uint32_t ref_len,
                             DvcCellAddr *out) {
  if (!engine || !ref_str || !out) return DVC_ERR_NULL_POINTER;
  if (!parse_a1_addr(engine, ref_str, ref_len, out)) return DVC_ERR_INVALID_ADDRESS;
  return DVC_OK;
}

uint32_t dvc_api_version(void){
  return (0u << 16) | (1u << 8) | 0u;
}







