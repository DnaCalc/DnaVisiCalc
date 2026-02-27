using System.Globalization;

namespace Dvc.Core;

internal readonly record struct FormulaFeatures(bool HasVolatile, bool HasExternal, bool HasStream);

internal sealed record InputEntry(
    DvcInputType Kind,
    double Number,
    string Text,
    string Formula,
    FormulaFeatures Features)
{
    public static InputEntry NumberValue(double value) =>
        new(DvcInputType.Number, value, string.Empty, string.Empty, default);

    public static InputEntry TextValue(string value) =>
        new(DvcInputType.Text, 0.0, value, string.Empty, default);

    public static InputEntry FormulaValue(string formula, FormulaFeatures features) =>
        new(DvcInputType.Formula, 0.0, string.Empty, formula, features);
}

internal readonly record struct CellEval(
    DvcValueType Type,
    double Number,
    int Bool,
    string Text,
    DvcCellErrorKind ErrorKind,
    string ErrorMessage,
    ulong Epoch)
{
    public static CellEval NumberValue(double value, ulong epoch = 0) =>
        new(DvcValueType.Number, value, 0, string.Empty, 0, string.Empty, epoch);

    public static CellEval TextValue(string text, ulong epoch = 0) =>
        new(DvcValueType.Text, 0.0, 0, text, 0, string.Empty, epoch);

    public static CellEval BoolValue(bool value, ulong epoch = 0) =>
        new(DvcValueType.Bool, 0.0, value ? 1 : 0, string.Empty, 0, string.Empty, epoch);

    public static CellEval BlankValue(ulong epoch = 0) =>
        new(DvcValueType.Blank, 0.0, 0, string.Empty, 0, string.Empty, epoch);

    public static CellEval ErrorValue(DvcCellErrorKind kind, string message, ulong epoch = 0) =>
        new(DvcValueType.Error, 0.0, 0, string.Empty, kind, message, epoch);

    public DvcCellValue ToCellValue() =>
        new()
        {
            Type = Type,
            Number = Number,
            BoolVal = Bool,
            ErrorKind = ErrorKind,
        };

    public bool TryAsNumber(out double value)
    {
        value = 0.0;
        if (Type == DvcValueType.Number)
        {
            value = Number;
            return true;
        }

        if (Type == DvcValueType.Bool)
        {
            value = Bool;
            return true;
        }

        if (Type == DvcValueType.Text && double.TryParse(Text, NumberStyles.Float, CultureInfo.InvariantCulture, out value))
        {
            return true;
        }

        return false;
    }

    public string AsText() =>
        Type switch
        {
            DvcValueType.Text => Text,
            DvcValueType.Number => Number.ToString("G17", CultureInfo.InvariantCulture),
            DvcValueType.Bool => Bool == 0 ? "FALSE" : "TRUE",
            DvcValueType.Blank => string.Empty,
            DvcValueType.Error => "#ERROR!",
            _ => string.Empty,
        };
}

internal sealed class EvalValue
{
    public CellEval Scalar { get; private init; } = CellEval.BlankValue();
    public CellEval[,]? Matrix { get; private init; }

    public static EvalValue FromScalar(CellEval scalar) => new() { Scalar = scalar };
    public static EvalValue FromMatrix(CellEval[,] matrix) => new() { Matrix = matrix, Scalar = matrix[0, 0] };
}

internal readonly record struct CellAddressToken(int Col, int Row, bool ColAbsolute, bool RowAbsolute)
{
    public DvcCellAddr ToAbsolute(DvcCellAddr origin)
    {
        var col = Col;
        var row = Row;
        return new DvcCellAddr((ushort)col, (ushort)row);
    }

    public string ToA1()
    {
        var colName = A1Ref.ToColumnName(Col);
        var rowName = Row.ToString(CultureInfo.InvariantCulture);
        return $"{(ColAbsolute ? "$" : string.Empty)}{colName}{(RowAbsolute ? "$" : string.Empty)}{rowName}";
    }
}

internal static class A1Ref
{
    public static bool TryParseCellRef(string text, out CellAddressToken token)
    {
        token = default;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        var t = text.Trim().ToUpperInvariant();
        var i = 0;
        var colAbs = false;
        var rowAbs = false;
        if (t[i] == '$')
        {
            colAbs = true;
            i++;
        }

        var colStart = i;
        while (i < t.Length && t[i] is >= 'A' and <= 'Z')
        {
            i++;
        }

        if (colStart == i)
        {
            return false;
        }

        if (i < t.Length && t[i] == '$')
        {
            rowAbs = true;
            i++;
        }

        var rowStart = i;
        while (i < t.Length && char.IsDigit(t[i]))
        {
            i++;
        }

        if (rowStart == i || i != t.Length)
        {
            return false;
        }

        if (!int.TryParse(t[rowStart..], NumberStyles.None, CultureInfo.InvariantCulture, out var row) || row <= 0)
        {
            return false;
        }

        var col = ToColumnIndex(t[colStart..(rowAbs ? rowStart - 1 : rowStart)]);
        if (col <= 0)
        {
            return false;
        }

        token = new CellAddressToken(col, row, colAbs, rowAbs);
        return true;
    }

    public static int ToColumnIndex(string colText)
    {
        var value = 0;
        foreach (var ch in colText)
        {
            if (ch is < 'A' or > 'Z')
            {
                return -1;
            }

            value = (value * 26) + (ch - 'A' + 1);
        }

        return value;
    }

    public static string ToColumnName(int col)
    {
        if (col <= 0)
        {
            return string.Empty;
        }

        var chars = new Stack<char>();
        var c = col;
        while (c > 0)
        {
            var rem = (c - 1) % 26;
            chars.Push((char)('A' + rem));
            c = (c - 1) / 26;
        }

        return new string(chars.ToArray());
    }
}

internal sealed record SpillInfo(DvcCellAddr Anchor, CellEval[,] Matrix, DvcCellRange Range)
{
    public CellEval GetValue(DvcCellAddr addr)
    {
        var r = addr.Row - Anchor.Row;
        var c = addr.Col - Anchor.Col;
        return Matrix[r, c];
    }
}

internal sealed class StreamState
{
    public StreamState(double period)
    {
        Period = period;
    }

    public double Period { get; set; }
    public double Accumulator { get; set; }
    public double Counter { get; set; }
}

internal sealed class ControlState
{
    public ControlState(DvcControlDef def)
    {
        Def = def;
    }

    public DvcControlDef Def { get; }
}

internal sealed class ChartState
{
    public ChartState(DvcChartDef def)
    {
        Def = def;
    }

    public DvcChartDef Def { get; }
    public ChartOutput? Output { get; set; }
}

public sealed record ChartOutput(string Name, List<string> Labels, List<double> SeriesValues);

internal sealed record UdfState(DvcVolatility Volatility, DvcEngineCore.UdfCallback Callback);
