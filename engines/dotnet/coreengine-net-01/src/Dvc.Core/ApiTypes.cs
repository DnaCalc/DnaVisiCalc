using System.Runtime.InteropServices;

namespace Dvc.Core;

public enum DvcStatus : int
{
    Ok = 0,
    RejectStructuralConstraint = 1,
    RejectPolicy = 2,
    ErrNullPointer = -1,
    ErrOutOfBounds = -2,
    ErrInvalidAddress = -3,
    ErrParse = -4,
    ErrDependency = -5,
    ErrInvalidName = -6,
    ErrOutOfMemory = -7,
    ErrInvalidArgument = -8,
}

public enum DvcRejectKind : int
{
    None = 0,
    StructuralConstraint = 1,
    Policy = 2,
}

public enum DvcValueType : int
{
    Number = 0,
    Text = 1,
    Bool = 2,
    Blank = 3,
    Error = 4,
}

public enum DvcCellErrorKind : int
{
    DivZero = 0,
    Value = 1,
    Name = 2,
    UnknownName = 3,
    Ref = 4,
    Spill = 5,
    Cycle = 6,
    Na = 7,
    Null = 8,
    Num = 9,
}

public enum DvcRecalcMode : int
{
    Automatic = 0,
    Manual = 1,
}

public enum DvcInputType : int
{
    Empty = 0,
    Number = 1,
    Text = 2,
    Formula = 3,
}

public enum DvcSpillRole : int
{
    None = 0,
    Anchor = 1,
    Member = 2,
}

public enum DvcPaletteColor : int
{
    None = -1,
    Mist = 0,
    Sage = 1,
    Fern = 2,
    Moss = 3,
    Olive = 4,
    Seafoam = 5,
    Lagoon = 6,
    Teal = 7,
    Sky = 8,
    Cloud = 9,
    Sand = 10,
    Clay = 11,
    Peach = 12,
    Rose = 13,
    Lavender = 14,
    Slate = 15,
}

public enum DvcStructuralOpKind : int
{
    None = 0,
    InsertRow = 1,
    DeleteRow = 2,
    InsertCol = 3,
    DeleteCol = 4,
}

public enum DvcVolatility : int
{
    Standard = 0,
    Volatile = 1,
    ExternallyInvalidated = 2,
}

public enum DvcControlKind : int
{
    Slider = 0,
    Checkbox = 1,
    Button = 2,
}

public enum DvcChangeType : int
{
    CellValue = 0,
    NameValue = 1,
    ChartOutput = 2,
    SpillRegion = 3,
    CellFormat = 4,
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcCellAddr
{
    public ushort Col;
    public ushort Row;

    public DvcCellAddr(ushort col, ushort row)
    {
        Col = col;
        Row = row;
    }

    public readonly bool IsValid => Col > 0 && Row > 0;

    public readonly long Key => ((long)Row << 32) | Col;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcCellRange
{
    public DvcCellAddr Start;
    public DvcCellAddr End;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcSheetBounds
{
    public ushort MaxColumns;
    public ushort MaxRows;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcCellValue
{
    public DvcValueType Type;
    public double Number;
    public int BoolVal;
    public DvcCellErrorKind ErrorKind;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcCellState
{
    public DvcCellValue Value;
    public ulong ValueEpoch;
    public int Stale;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcCellFormat : IEquatable<DvcCellFormat>
{
    public int HasDecimals;
    public byte Decimals;
    public int Bold;
    public int Italic;
    public DvcPaletteColor Fg;
    public DvcPaletteColor Bg;

    public static DvcCellFormat Default => new()
    {
        HasDecimals = 0,
        Decimals = 0,
        Bold = 0,
        Italic = 0,
        Fg = DvcPaletteColor.None,
        Bg = DvcPaletteColor.None,
    };

    public readonly bool IsDefault =>
        HasDecimals == 0 &&
        Decimals == 0 &&
        Bold == 0 &&
        Italic == 0 &&
        Fg == DvcPaletteColor.None &&
        Bg == DvcPaletteColor.None;

    public readonly bool Equals(DvcCellFormat other) =>
        HasDecimals == other.HasDecimals &&
        Decimals == other.Decimals &&
        Bold == other.Bold &&
        Italic == other.Italic &&
        Fg == other.Fg &&
        Bg == other.Bg;

    public override readonly bool Equals(object? obj) => obj is DvcCellFormat other && Equals(other);

    public override readonly int GetHashCode() => HashCode.Combine(HasDecimals, Decimals, Bold, Italic, Fg, Bg);
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcIterationConfig
{
    public int Enabled;
    public uint MaxIterations;
    public double ConvergenceTolerance;

    public static DvcIterationConfig Default => new()
    {
        Enabled = 0,
        MaxIterations = 100,
        ConvergenceTolerance = 0.001,
    };
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcLastRejectContext
{
    public DvcRejectKind RejectKind;
    public DvcStructuralOpKind OpKind;
    public ushort OpIndex;
    public int HasCell;
    public DvcCellAddr Cell;
    public int HasRange;
    public DvcCellRange Range;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcControlDef
{
    public DvcControlKind Kind;
    public double Min;
    public double Max;
    public double Step;
}

[StructLayout(LayoutKind.Sequential)]
public struct DvcChartDef
{
    public DvcCellRange SourceRange;
}

public static class DvcApiVersion
{
    public const uint Packed = (0u << 16) | (1u << 8) | 0u;
}
