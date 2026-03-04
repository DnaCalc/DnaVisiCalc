#ifndef DVC_ENGINE_H
#define DVC_ENGINE_H

#include <stdint.h>

#ifdef _WIN32
  #ifdef DVC_EXPORTS
    #define DVC_API __declspec(dllexport)
  #else
    #define DVC_API __declspec(dllimport)
  #endif
#else
  #define DVC_API
#endif

typedef int32_t DvcStatus;

#define DVC_OK                               0
#define DVC_REJECT_STRUCTURAL_CONSTRAINT     1
#define DVC_REJECT_POLICY                    2
#define DVC_ERR_NULL_POINTER                -1
#define DVC_ERR_OUT_OF_BOUNDS               -2
#define DVC_ERR_INVALID_ADDRESS             -3
#define DVC_ERR_PARSE                       -4
#define DVC_ERR_DEPENDENCY                  -5
#define DVC_ERR_INVALID_NAME                -6
#define DVC_ERR_OUT_OF_MEMORY               -7
#define DVC_ERR_INVALID_ARGUMENT            -8

typedef int32_t DvcRejectKind;
#define DVC_REJECT_KIND_NONE                  0
#define DVC_REJECT_KIND_STRUCTURAL_CONSTRAINT 1
#define DVC_REJECT_KIND_POLICY                2

typedef int32_t DvcValueType;
#define DVC_VALUE_NUMBER 0
#define DVC_VALUE_TEXT   1
#define DVC_VALUE_BOOL   2
#define DVC_VALUE_BLANK  3
#define DVC_VALUE_ERROR  4

typedef int32_t DvcCellErrorKind;
#define DVC_CELL_ERR_DIV_ZERO      0
#define DVC_CELL_ERR_VALUE         1
#define DVC_CELL_ERR_NAME          2
#define DVC_CELL_ERR_UNKNOWN_NAME  3
#define DVC_CELL_ERR_REF           4
#define DVC_CELL_ERR_SPILL         5
#define DVC_CELL_ERR_CYCLE         6
#define DVC_CELL_ERR_NA            7
#define DVC_CELL_ERR_NULL          8
#define DVC_CELL_ERR_NUM           9

typedef int32_t DvcRecalcMode;
#define DVC_RECALC_AUTOMATIC 0
#define DVC_RECALC_MANUAL    1

typedef int32_t DvcInputType;
#define DVC_INPUT_EMPTY   0
#define DVC_INPUT_NUMBER  1
#define DVC_INPUT_TEXT    2
#define DVC_INPUT_FORMULA 3

typedef int32_t DvcSpillRole;
#define DVC_SPILL_NONE   0
#define DVC_SPILL_ANCHOR 1
#define DVC_SPILL_MEMBER 2

typedef int32_t DvcPaletteColor;
#define DVC_COLOR_NONE     -1
#define DVC_COLOR_MIST      0
#define DVC_COLOR_SAGE      1
#define DVC_COLOR_FERN      2
#define DVC_COLOR_MOSS      3
#define DVC_COLOR_OLIVE     4
#define DVC_COLOR_SEAFOAM   5
#define DVC_COLOR_LAGOON    6
#define DVC_COLOR_TEAL      7
#define DVC_COLOR_SKY       8
#define DVC_COLOR_CLOUD     9
#define DVC_COLOR_SAND     10
#define DVC_COLOR_CLAY     11
#define DVC_COLOR_PEACH    12
#define DVC_COLOR_ROSE     13
#define DVC_COLOR_LAVENDER 14
#define DVC_COLOR_SLATE    15
#define DVC_PALETTE_COUNT  16

typedef struct {
  uint16_t col;
  uint16_t row;
} DvcCellAddr;

typedef struct {
  DvcCellAddr start;
  DvcCellAddr end;
} DvcCellRange;

typedef struct {
  uint16_t max_columns;
  uint16_t max_rows;
} DvcSheetBounds;

typedef struct {
  DvcValueType type;
  double number;
  int32_t bool_val;
  DvcCellErrorKind error_kind;
} DvcCellValue;

typedef struct {
  DvcCellValue value;
  uint64_t value_epoch;
  int32_t stale;
} DvcCellState;

typedef struct {
  int32_t has_decimals;
  uint8_t decimals;
  int32_t bold;
  int32_t italic;
  DvcPaletteColor fg;
  DvcPaletteColor bg;
} DvcCellFormat;

typedef struct DvcEngine DvcEngine;
typedef struct DvcCellIterator DvcCellIterator;
typedef struct DvcNameIterator DvcNameIterator;
typedef struct DvcFormatIterator DvcFormatIterator;
typedef struct DvcControlIterator DvcControlIterator;
typedef struct DvcChartIterator DvcChartIterator;
typedef struct DvcChangeIterator DvcChangeIterator;
typedef struct DvcChartOutput DvcChartOutput;

typedef int32_t DvcControlKind;
#define DVC_CONTROL_SLIDER   0
#define DVC_CONTROL_CHECKBOX 1
#define DVC_CONTROL_BUTTON   2

typedef struct {
  DvcControlKind kind;
  double min;
  double max;
  double step;
} DvcControlDef;

typedef struct {
  DvcCellRange source_range;
} DvcChartDef;

typedef int32_t DvcVolatility;
#define DVC_VOLATILITY_STANDARD               0
#define DVC_VOLATILITY_VOLATILE               1
#define DVC_VOLATILITY_EXTERNALLY_INVALIDATED 2

typedef int32_t DvcChangeType;
#define DVC_CHANGE_CELL_VALUE   0
#define DVC_CHANGE_NAME_VALUE   1
#define DVC_CHANGE_CHART_OUTPUT 2
#define DVC_CHANGE_SPILL_REGION 3
#define DVC_CHANGE_CELL_FORMAT  4
#define DVC_CHANGE_DIAGNOSTIC   5

typedef int32_t DvcDiagnosticCode;
#define DVC_DIAG_CIRCULAR_REFERENCE_DETECTED 0

typedef struct {
  int32_t enabled;
  uint32_t max_iterations;
  double convergence_tolerance;
} DvcIterationConfig;

typedef int32_t DvcStructuralOpKind;
#define DVC_STRUCT_OP_NONE       0
#define DVC_STRUCT_OP_INSERT_ROW 1
#define DVC_STRUCT_OP_DELETE_ROW 2
#define DVC_STRUCT_OP_INSERT_COL 3
#define DVC_STRUCT_OP_DELETE_COL 4

typedef struct {
  DvcRejectKind reject_kind;
  DvcStructuralOpKind op_kind;
  uint16_t op_index;
  int32_t has_cell;
  DvcCellAddr cell;
  int32_t has_range;
  DvcCellRange range;
} DvcLastRejectContext;

typedef DvcStatus (*DvcUdfCallback)(
  void *user_data,
  const DvcCellValue *args,
  uint32_t arg_count,
  DvcCellValue *out
);

#ifdef __cplusplus
extern "C" {
#endif
DVC_API DvcStatus dvc_engine_create(DvcEngine **out);

DVC_API DvcStatus dvc_engine_create_with_bounds(DvcSheetBounds bounds, DvcEngine **out);

DVC_API DvcStatus dvc_engine_destroy(DvcEngine *engine);

DVC_API DvcStatus dvc_engine_clear(DvcEngine *engine);

DVC_API DvcStatus dvc_engine_bounds(const DvcEngine *engine, DvcSheetBounds *out);

DVC_API DvcStatus dvc_engine_get_recalc_mode(const DvcEngine *engine, DvcRecalcMode *out);

DVC_API DvcStatus dvc_engine_set_recalc_mode(DvcEngine *engine, DvcRecalcMode mode);

DVC_API DvcStatus dvc_engine_committed_epoch(const DvcEngine *engine, uint64_t *out);

DVC_API DvcStatus dvc_engine_stabilized_epoch(const DvcEngine *engine, uint64_t *out);

DVC_API DvcStatus dvc_engine_is_stable(const DvcEngine *engine, int32_t *out);

DVC_API DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double value);

DVC_API DvcStatus dvc_cell_set_text(DvcEngine *engine, DvcCellAddr addr,
                            const char *text, uint32_t text_len);

DVC_API DvcStatus dvc_cell_set_formula(DvcEngine *engine, DvcCellAddr addr,
                               const char *formula, uint32_t formula_len);

DVC_API DvcStatus dvc_cell_clear(DvcEngine *engine, DvcCellAddr addr);

DVC_API DvcStatus dvc_cell_get_state(const DvcEngine *engine, DvcCellAddr addr,
                             DvcCellState *out);

DVC_API DvcStatus dvc_cell_get_text(const DvcEngine *engine, DvcCellAddr addr,
                            char *buf, uint32_t buf_len, uint32_t *out_len);

DVC_API DvcStatus dvc_cell_get_input_type(const DvcEngine *engine, DvcCellAddr addr,
                                  DvcInputType *out);

DVC_API DvcStatus dvc_cell_get_input_text(const DvcEngine *engine, DvcCellAddr addr,
                                  char *buf, uint32_t buf_len, uint32_t *out_len);

DVC_API DvcStatus dvc_cell_set_number_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 double value);

DVC_API DvcStatus dvc_cell_set_text_a1(DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               const char *text, uint32_t text_len);

DVC_API DvcStatus dvc_cell_set_formula_a1(DvcEngine *engine,
                                  const char *cell_ref, uint32_t ref_len,
                                  const char *formula, uint32_t formula_len);

DVC_API DvcStatus dvc_cell_clear_a1(DvcEngine *engine,
                            const char *cell_ref, uint32_t ref_len);

DVC_API DvcStatus dvc_cell_get_state_a1(const DvcEngine *engine,
                                const char *cell_ref, uint32_t ref_len,
                                DvcCellState *out);

DVC_API DvcStatus dvc_cell_get_text_a1(const DvcEngine *engine,
                               const char *cell_ref, uint32_t ref_len,
                               char *buf, uint32_t buf_len, uint32_t *out_len);

DVC_API DvcStatus dvc_cell_get_input_type_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     DvcInputType *out);

DVC_API DvcStatus dvc_cell_get_input_text_a1(const DvcEngine *engine,
                                     const char *cell_ref, uint32_t ref_len,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DVC_API DvcStatus dvc_name_set_number(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              double value);

DVC_API DvcStatus dvc_name_set_text(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const char *text, uint32_t text_len);

DVC_API DvcStatus dvc_name_set_formula(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               const char *formula, uint32_t formula_len);

DVC_API DvcStatus dvc_name_clear(DvcEngine *engine,
                         const char *name, uint32_t name_len);

DVC_API DvcStatus dvc_name_get_input_type(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  DvcInputType *out);

DVC_API DvcStatus dvc_name_get_input_text(const DvcEngine *engine,
                                  const char *name, uint32_t name_len,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len);

DVC_API DvcStatus dvc_recalculate(DvcEngine *engine);

DVC_API DvcStatus dvc_has_volatile_cells(const DvcEngine *engine, int32_t *out);

DVC_API DvcStatus dvc_has_externally_invalidated_cells(const DvcEngine *engine, int32_t *out);

DVC_API DvcStatus dvc_invalidate_volatile(DvcEngine *engine);

DVC_API DvcStatus dvc_has_stream_cells(const DvcEngine *engine, int32_t *out);

DVC_API DvcStatus dvc_tick_streams(DvcEngine *engine, double elapsed_secs,
                           int32_t *any_advanced);

DVC_API DvcStatus dvc_invalidate_udf(DvcEngine *engine,
                              const char *name, uint32_t name_len);

DVC_API DvcStatus dvc_cell_get_format(const DvcEngine *engine, DvcCellAddr addr,
                              DvcCellFormat *out);

DVC_API DvcStatus dvc_cell_set_format(DvcEngine *engine, DvcCellAddr addr,
                              const DvcCellFormat *format);

DVC_API DvcStatus dvc_cell_get_format_a1(const DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 DvcCellFormat *out);

DVC_API DvcStatus dvc_cell_set_format_a1(DvcEngine *engine,
                                 const char *cell_ref, uint32_t ref_len,
                                 const DvcCellFormat *format);

DVC_API DvcStatus dvc_cell_spill_role(const DvcEngine *engine, DvcCellAddr addr,
                              DvcSpillRole *out);

DVC_API DvcStatus dvc_cell_spill_anchor(const DvcEngine *engine, DvcCellAddr addr,
                                DvcCellAddr *out, int32_t *found);

DVC_API DvcStatus dvc_cell_spill_range(const DvcEngine *engine, DvcCellAddr addr,
                               DvcCellRange *out, int32_t *found);

DVC_API DvcStatus dvc_cell_iterate(const DvcEngine *engine, DvcCellIterator **out);

DVC_API DvcStatus dvc_cell_iterator_next(DvcCellIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcInputType *input_type,
                                 int32_t *done);

DVC_API DvcStatus dvc_cell_iterator_get_text(const DvcCellIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DVC_API DvcStatus dvc_cell_iterator_destroy(DvcCellIterator *iter);

DVC_API DvcStatus dvc_name_iterate(const DvcEngine *engine, DvcNameIterator **out);

DVC_API DvcStatus dvc_name_iterator_next(DvcNameIterator *iter,
                                 char *name_buf, uint32_t name_buf_len,
                                 uint32_t *name_len,
                                 DvcInputType *input_type,
                                 int32_t *done);

DVC_API DvcStatus dvc_name_iterator_get_text(const DvcNameIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DVC_API DvcStatus dvc_name_iterator_destroy(DvcNameIterator *iter);

DVC_API DvcStatus dvc_format_iterate(const DvcEngine *engine, DvcFormatIterator **out);

DVC_API DvcStatus dvc_format_iterator_next(DvcFormatIterator *iter,
                                   DvcCellAddr *addr,
                                   DvcCellFormat *format,
                                   int32_t *done);

DVC_API DvcStatus dvc_format_iterator_destroy(DvcFormatIterator *iter);

DVC_API DvcStatus dvc_insert_row(DvcEngine *engine, uint16_t at);

DVC_API DvcStatus dvc_delete_row(DvcEngine *engine, uint16_t at);

DVC_API DvcStatus dvc_insert_col(DvcEngine *engine, uint16_t at);

DVC_API DvcStatus dvc_delete_col(DvcEngine *engine, uint16_t at);

DVC_API DvcStatus dvc_engine_get_iteration_config(const DvcEngine *engine,
                                           DvcIterationConfig *out);

DVC_API DvcStatus dvc_engine_set_iteration_config(DvcEngine *engine,
                                           const DvcIterationConfig *config);

DVC_API DvcStatus dvc_control_define(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              const DvcControlDef *def);

DVC_API DvcStatus dvc_control_remove(DvcEngine *engine,
                              const char *name, uint32_t name_len,
                              int32_t *found);

DVC_API DvcStatus dvc_control_set_value(DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double value);

DVC_API DvcStatus dvc_control_get_value(const DvcEngine *engine,
                                 const char *name, uint32_t name_len,
                                 double *out, int32_t *found);

DVC_API DvcStatus dvc_control_get_def(const DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               DvcControlDef *out, int32_t *found);

DVC_API DvcStatus dvc_control_iterate(const DvcEngine *engine,
                               DvcControlIterator **out);

DVC_API DvcStatus dvc_control_iterator_next(DvcControlIterator *iter,
                                     char *name_buf, uint32_t name_buf_len,
                                     uint32_t *name_len,
                                     DvcControlDef *def,
                                     double *value,
                                     int32_t *done);

DVC_API DvcStatus dvc_control_iterator_destroy(DvcControlIterator *iter);

DVC_API DvcStatus dvc_chart_define(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            const DvcChartDef *def);

DVC_API DvcStatus dvc_chart_remove(DvcEngine *engine,
                            const char *name, uint32_t name_len,
                            int32_t *found);

DVC_API DvcStatus dvc_chart_get_output(const DvcEngine *engine,
                                const char *name, uint32_t name_len,
                                DvcChartOutput **out, int32_t *found);

DVC_API DvcStatus dvc_chart_output_series_count(const DvcChartOutput *output,
                                         uint32_t *out);

DVC_API DvcStatus dvc_chart_output_label_count(const DvcChartOutput *output,
                                        uint32_t *out);

DVC_API DvcStatus dvc_chart_output_label(const DvcChartOutput *output,
                                  uint32_t index,
                                  char *buf, uint32_t buf_len,
                                  uint32_t *out_len);

DVC_API DvcStatus dvc_chart_output_series_name(const DvcChartOutput *output,
                                        uint32_t series_index,
                                        char *buf, uint32_t buf_len,
                                        uint32_t *out_len);

DVC_API DvcStatus dvc_chart_output_series_values(const DvcChartOutput *output,
                                          uint32_t series_index,
                                          double *buf, uint32_t buf_len,
                                          uint32_t *out_count);

DVC_API DvcStatus dvc_chart_iterate(const DvcEngine *engine,
                             DvcChartIterator **out);

DVC_API DvcStatus dvc_chart_iterator_next(DvcChartIterator *iter,
                                   char *name_buf, uint32_t name_buf_len,
                                   uint32_t *name_len,
                                   DvcChartDef *def,
                                   int32_t *done);

DVC_API DvcStatus dvc_chart_iterator_destroy(DvcChartIterator *iter);

DVC_API DvcStatus dvc_udf_register(DvcEngine *engine,
                             const char *name, uint32_t name_len,
                             DvcUdfCallback callback,
                             void *user_data,
                             DvcVolatility volatility);

DVC_API DvcStatus dvc_udf_unregister(DvcEngine *engine,
                               const char *name, uint32_t name_len,
                               int32_t *found);

DVC_API DvcStatus dvc_change_tracking_enable(DvcEngine *engine);

DVC_API DvcStatus dvc_change_tracking_disable(DvcEngine *engine);

DVC_API DvcStatus dvc_change_tracking_is_enabled(const DvcEngine *engine,
                                          int32_t *out);

DVC_API DvcStatus dvc_change_iterate(DvcEngine *engine, DvcChangeIterator **out);

DVC_API DvcStatus dvc_change_iterator_next(DvcChangeIterator *iter,
                                    DvcChangeType *change_type,
                                    uint64_t *epoch,
                                    int32_t *done);

DVC_API DvcStatus dvc_change_get_cell(const DvcChangeIterator *iter,
                               DvcCellAddr *addr);

DVC_API DvcStatus dvc_change_get_name(const DvcChangeIterator *iter,
                               char *buf, uint32_t buf_len,
                               uint32_t *out_len);

DVC_API DvcStatus dvc_change_get_chart_name(const DvcChangeIterator *iter,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DVC_API DvcStatus dvc_change_get_spill(const DvcChangeIterator *iter,
                                DvcCellAddr *anchor,
                                DvcCellRange *old_range, int32_t *had_old,
                                DvcCellRange *new_range, int32_t *has_new);

DVC_API DvcStatus dvc_change_get_format(const DvcChangeIterator *iter,
                                 DvcCellAddr *addr,
                                 DvcCellFormat *old_fmt,
                                 DvcCellFormat *new_fmt);

DVC_API DvcStatus dvc_change_get_diagnostic(const DvcChangeIterator *iter,
                                     DvcDiagnosticCode *code,
                                     char *buf, uint32_t buf_len,
                                     uint32_t *out_len);

DVC_API DvcStatus dvc_change_iterator_destroy(DvcChangeIterator *iter);

DVC_API DvcStatus dvc_last_error_message(const DvcEngine *engine,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);

DVC_API DvcStatus dvc_last_error_kind(const DvcEngine *engine, DvcStatus *out);

DVC_API DvcStatus dvc_last_reject_kind(const DvcEngine *engine, DvcRejectKind *out);

DVC_API DvcStatus dvc_last_reject_context(const DvcEngine *engine,
                                  DvcLastRejectContext *out);

DVC_API DvcStatus dvc_cell_error_message(const DvcEngine *engine, DvcCellAddr addr,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);

DVC_API DvcStatus dvc_palette_color_name(DvcPaletteColor color,
                                 char *buf, uint32_t buf_len,
                                 uint32_t *out_len);

DVC_API DvcStatus dvc_parse_cell_ref(const DvcEngine *engine,
                             const char *ref_str, uint32_t ref_len,
                             DvcCellAddr *out);

DVC_API uint32_t dvc_api_version(void);

#ifdef __cplusplus
}
#endif

#endif
