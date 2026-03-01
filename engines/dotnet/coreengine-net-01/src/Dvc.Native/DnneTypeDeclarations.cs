namespace Dvc.Native;

internal static class DnneNativeTypes
{
    public const string C99Declarations = @"typedef int32_t DvcStatus;
typedef int32_t DvcRejectKind;
typedef int32_t DvcValueType;
typedef int32_t DvcCellErrorKind;
typedef int32_t DvcRecalcMode;
typedef int32_t DvcInputType;
typedef int32_t DvcSpillRole;
typedef int32_t DvcPaletteColor;
typedef int32_t DvcStructuralOpKind;
typedef int32_t DvcVolatility;
typedef int32_t DvcControlKind;
typedef int32_t DvcChangeType;
typedef int32_t DvcDiagnosticCode;
typedef struct DvcCellAddr { uint16_t Col; uint16_t Row; } DvcCellAddr;
typedef struct DvcCellRange { DvcCellAddr Start; DvcCellAddr End; } DvcCellRange;
typedef struct DvcSheetBounds { uint16_t MaxColumns; uint16_t MaxRows; } DvcSheetBounds;
typedef struct DvcCellValue { DvcValueType Type; double Number; int32_t BoolVal; DvcCellErrorKind ErrorKind; } DvcCellValue;
typedef struct DvcCellState { DvcCellValue Value; uint64_t ValueEpoch; int32_t Stale; } DvcCellState;
typedef struct DvcCellFormat {
    int32_t HasDecimals;
    uint8_t Decimals;
    int32_t Bold;
    int32_t Italic;
    DvcPaletteColor Fg;
    DvcPaletteColor Bg;
} DvcCellFormat;
typedef struct DvcIterationConfig { int32_t Enabled; uint32_t MaxIterations; double ConvergenceTolerance; } DvcIterationConfig;
typedef struct DvcLastRejectContext {
    DvcRejectKind RejectKind;
    DvcStructuralOpKind OpKind;
    uint16_t OpIndex;
    int32_t HasCell;
    DvcCellAddr Cell;
    int32_t HasRange;
    DvcCellRange Range;
} DvcLastRejectContext;
typedef struct DvcControlDef { DvcControlKind Kind; double Min; double Max; double Step; } DvcControlDef;
typedef struct DvcChartDef { DvcCellRange SourceRange; } DvcChartDef;";
}
