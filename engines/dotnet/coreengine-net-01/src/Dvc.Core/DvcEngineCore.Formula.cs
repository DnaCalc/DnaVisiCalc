using System.Globalization;
using System.Text.RegularExpressions;

namespace Dvc.Core;

public sealed partial class DvcEngineCore
{
    private static readonly HashSet<string> Builtins = new(StringComparer.Ordinal)
    {
        "SUM", "IF", "SEQUENCE", "RAND", "RANDARRAY", "NOW", "STREAM",
        "AND", "OR", "NOT", "MIN", "MAX", "ABS", "ROUND", "ROW", "COLUMN",
    };

    private static readonly Regex RefRegex = new(@"(?<![A-Z0-9_])\$?[A-Z]{1,3}\$?[1-9][0-9]*#?", RegexOptions.IgnoreCase | RegexOptions.Compiled);
    private static readonly Regex FnRegex = new(@"(?<![A-Z0-9_])([A-Z_][A-Z0-9_]*)\s*\(", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private static FormulaFeatures ClassifyFormula(string formula)
    {
        var text = formula.ToUpperInvariant();
        var hasVol = text.Contains("RAND(") || text.Contains("RANDARRAY(") || text.Contains("NOW(");
        var hasStream = text.Contains("STREAM(");
        return new FormulaFeatures(hasVol, hasStream, hasStream);
    }

    private static bool TryParseFormula(string formula, out ExprNode ast)
    {
        var parser = new ExprParser(formula);
        return parser.TryParse(out ast);
    }

    private static ExprNode? ParseFormulaOrNull(string formula)
    {
        if (formula.Contains("#REF!", StringComparison.OrdinalIgnoreCase))
        {
            return null;
        }

        return TryParseFormula(formula, out var ast) ? ast : null;
    }

    private EvalValue EvaluateFormula(InputEntry input, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        var formula = input.Formula;
        if (formula.Contains("#REF!", StringComparison.OrdinalIgnoreCase))
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Formula contains invalid reference."));
        }

        var ast = input.ParsedFormula;
        if (ast is null && !TryParseFormula(formula, out ast))
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Formula parse failed."));
        }

        return EvalAst(ast!, self, visitingCells, visitingNames);
    }

    private EvalValue EvalAst(ExprNode node, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        switch (node.Kind)
        {
            case ExprKind.Number:
                return EvalValue.FromScalar(CellEval.NumberValue(node.Number));
            case ExprKind.Text:
                return EvalValue.FromScalar(CellEval.TextValue(node.Text));
            case ExprKind.Bool:
                return EvalValue.FromScalar(CellEval.BoolValue(node.Bool != 0));
            case ExprKind.Cell:
            {
                var addr = node.CellToken.ToAbsolute(self);
                if (!IsInBounds(addr))
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Cell reference out of bounds."));
                }

                return EvalValue.FromScalar(EvaluateCell(addr, visitingCells, visitingNames));
            }
            case ExprKind.SpillRef:
            {
                var anchor = node.CellToken.ToAbsolute(self);
                if (!IsInBounds(anchor))
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Spill reference out of bounds."));
                }

                if (!_spillAnchors.TryGetValue(anchor.Key, out var spill))
                {
                    _ = EvaluateCell(anchor, visitingCells, visitingNames);
                }

                if (!_spillAnchors.TryGetValue(anchor.Key, out spill))
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Spill reference not available."));
                }

                return EvalValue.FromMatrix(spill.Matrix);
            }
            case ExprKind.Name:
            {
                var norm = node.Name.ToUpperInvariant();
                if (!_names.ContainsKey(norm))
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.UnknownName, "Unknown name."));
                }

                return EvalValue.FromScalar(EvaluateName(norm, visitingCells, visitingNames));
            }
            case ExprKind.Unary:
            {
                var inner = EvalAst(node.Arguments[0], self, visitingCells, visitingNames).Scalar;
                if (!inner.TryAsNumber(out var n))
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Unary non-number."));
                }

                return EvalValue.FromScalar(CellEval.NumberValue(node.Operator == "-" ? -n : n));
            }
            case ExprKind.Binary:
            {
                var left = EvalAst(node.Arguments[0], self, visitingCells, visitingNames).Scalar;
                var right = EvalAst(node.Arguments[1], self, visitingCells, visitingNames).Scalar;
                if (left.Type == DvcValueType.Error)
                {
                    return EvalValue.FromScalar(left);
                }

                if (right.Type == DvcValueType.Error)
                {
                    return EvalValue.FromScalar(right);
                }

                if (!left.TryAsNumber(out var ln) || !right.TryAsNumber(out var rn))
                {
                    if (node.Operator == "&")
                    {
                        return EvalValue.FromScalar(CellEval.TextValue(left.AsText() + right.AsText()));
                    }

                    if (TryCompare(left, right, node.Operator, out var cmp))
                    {
                        return EvalValue.FromScalar(CellEval.BoolValue(cmp));
                    }

                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Binary non-number."));
                }

                return node.Operator switch
                {
                    "+" => EvalValue.FromScalar(CellEval.NumberValue(ln + rn)),
                    "-" => EvalValue.FromScalar(CellEval.NumberValue(ln - rn)),
                    "*" => EvalValue.FromScalar(CellEval.NumberValue(ln * rn)),
                    "/" => rn == 0 ? EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.DivZero, "Divide by zero.")) : EvalValue.FromScalar(CellEval.NumberValue(ln / rn)),
                    "=" => EvalValue.FromScalar(CellEval.BoolValue(Math.Abs(ln - rn) <= double.Epsilon)),
                    "<>" => EvalValue.FromScalar(CellEval.BoolValue(Math.Abs(ln - rn) > double.Epsilon)),
                    "<" => EvalValue.FromScalar(CellEval.BoolValue(ln < rn)),
                    "<=" => EvalValue.FromScalar(CellEval.BoolValue(ln <= rn)),
                    ">" => EvalValue.FromScalar(CellEval.BoolValue(ln > rn)),
                    ">=" => EvalValue.FromScalar(CellEval.BoolValue(ln >= rn)),
                    "&" => EvalValue.FromScalar(CellEval.TextValue(left.AsText() + right.AsText())),
                    _ => EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Unsupported operator.")),
                };
            }
            case ExprKind.Function:
                return EvalFunction(node, self, visitingCells, visitingNames);
            default:
                return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Unsupported expression."));
        }
    }

    private EvalValue EvalFunction(ExprNode node, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        var name = node.Name.ToUpperInvariant();
        switch (name)
        {
            case "SUM":
            {
                var total = 0.0;
                foreach (var arg in node.Arguments)
                {
                    if (arg.Kind == ExprKind.Range)
                    {
                        if (!arg.RangeStart.HasValue || !arg.RangeEnd.HasValue)
                        {
                            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Invalid range."));
                        }

                        var start = arg.RangeStart.Value.ToAbsolute(self);
                        var end = arg.RangeEnd.Value.ToAbsolute(self);
                        var rMin = Math.Min(start.Row, end.Row);
                        var rMax = Math.Max(start.Row, end.Row);
                        var cMin = Math.Min(start.Col, end.Col);
                        var cMax = Math.Max(start.Col, end.Col);
                        for (var r = rMin; r <= rMax; r++)
                        {
                            for (var c = cMin; c <= cMax; c++)
                            {
                                var v = EvaluateCell(new DvcCellAddr((ushort)c, (ushort)r), visitingCells, visitingNames);
                                if (v.TryAsNumber(out var n))
                                {
                                    total += n;
                                }
                            }
                        }

                        continue;
                    }

                    if (arg.Kind == ExprKind.SpillRef)
                    {
                        var spill = EvalAst(arg, self, visitingCells, visitingNames);
                        if (spill.Matrix is null)
                        {
                            if (spill.Scalar.Type == DvcValueType.Error)
                            {
                                return EvalValue.FromScalar(spill.Scalar);
                            }

                            if (spill.Scalar.TryAsNumber(out var scalarNumber))
                            {
                                total += scalarNumber;
                            }

                            continue;
                        }

                        for (var r = 0; r < spill.Matrix.GetLength(0); r++)
                        {
                            for (var c = 0; c < spill.Matrix.GetLength(1); c++)
                            {
                                var spillValue = spill.Matrix[r, c];
                                if (spillValue.TryAsNumber(out var matrixNumber))
                                {
                                    total += matrixNumber;
                                }
                            }
                        }

                        continue;
                    }

                    var value = EvalAst(arg, self, visitingCells, visitingNames).Scalar;
                    if (value.TryAsNumber(out var number))
                    {
                        total += number;
                    }
                }

                return EvalValue.FromScalar(CellEval.NumberValue(total));
            }
            case "MIN":
            case "MAX":
            {
                var numbers = new List<double>();
                foreach (var arg in node.Arguments)
                {
                    if (arg.Kind == ExprKind.Range)
                    {
                        if (!arg.RangeStart.HasValue || !arg.RangeEnd.HasValue)
                        {
                            continue;
                        }

                        var start = arg.RangeStart.Value.ToAbsolute(self);
                        var end = arg.RangeEnd.Value.ToAbsolute(self);
                        var rMin = Math.Min(start.Row, end.Row);
                        var rMax = Math.Max(start.Row, end.Row);
                        var cMin = Math.Min(start.Col, end.Col);
                        var cMax = Math.Max(start.Col, end.Col);
                        for (var r = rMin; r <= rMax; r++)
                        {
                            for (var c = cMin; c <= cMax; c++)
                            {
                                var value = EvaluateCell(new DvcCellAddr((ushort)c, (ushort)r), visitingCells, visitingNames);
                                if (value.TryAsNumber(out var n))
                                {
                                    numbers.Add(n);
                                }
                            }
                        }

                        continue;
                    }

                    var eval = EvalAst(arg, self, visitingCells, visitingNames);
                    if (eval.Matrix is not null)
                    {
                        for (var r = 0; r < eval.Matrix.GetLength(0); r++)
                        {
                            for (var c = 0; c < eval.Matrix.GetLength(1); c++)
                            {
                                var value = eval.Matrix[r, c];
                                if (value.TryAsNumber(out var n))
                                {
                                    numbers.Add(n);
                                }
                            }
                        }

                        continue;
                    }

                    if (eval.Scalar.TryAsNumber(out var scalar))
                    {
                        numbers.Add(scalar);
                    }
                }

                if (numbers.Count == 0)
                {
                    return EvalValue.FromScalar(CellEval.NumberValue(0.0));
                }

                return EvalValue.FromScalar(CellEval.NumberValue(name == "MIN" ? numbers.Min() : numbers.Max()));
            }
            case "ABS":
            {
                var value = EvalDoubleArg(node.Arguments, 0, 0.0, self, visitingCells, visitingNames);
                return EvalValue.FromScalar(CellEval.NumberValue(Math.Abs(value)));
            }
            case "ROUND":
            {
                var value = EvalDoubleArg(node.Arguments, 0, 0.0, self, visitingCells, visitingNames);
                var digits = EvalIntArg(node.Arguments, 1, 0, self, visitingCells, visitingNames);
                digits = Math.Clamp(digits, 0, 9);
                return EvalValue.FromScalar(CellEval.NumberValue(Math.Round(value, digits, MidpointRounding.AwayFromZero)));
            }
            case "IF":
            {
                if (node.Arguments.Count < 2)
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "IF requires 2 args."));
                }

                var cond = EvalAst(node.Arguments[0], self, visitingCells, visitingNames).Scalar;
                var truthy = cond.TryAsNumber(out var c) ? Math.Abs(c) > double.Epsilon : !string.IsNullOrEmpty(cond.AsText());
                if (truthy)
                {
                    return EvalAst(node.Arguments[1], self, visitingCells, visitingNames);
                }

                return node.Arguments.Count >= 3
                    ? EvalAst(node.Arguments[2], self, visitingCells, visitingNames)
                    : EvalValue.FromScalar(CellEval.BlankValue());
            }
            case "AND":
            {
                if (node.Arguments.Count == 0)
                {
                    return EvalValue.FromScalar(CellEval.BoolValue(true));
                }

                foreach (var arg in node.Arguments)
                {
                    var value = EvalAst(arg, self, visitingCells, visitingNames).Scalar;
                    if (value.Type == DvcValueType.Error)
                    {
                        return EvalValue.FromScalar(value);
                    }

                    if (!IsTruthy(value))
                    {
                        return EvalValue.FromScalar(CellEval.BoolValue(false));
                    }
                }

                return EvalValue.FromScalar(CellEval.BoolValue(true));
            }
            case "OR":
            {
                if (node.Arguments.Count == 0)
                {
                    return EvalValue.FromScalar(CellEval.BoolValue(false));
                }

                foreach (var arg in node.Arguments)
                {
                    var value = EvalAst(arg, self, visitingCells, visitingNames).Scalar;
                    if (value.Type == DvcValueType.Error)
                    {
                        return EvalValue.FromScalar(value);
                    }

                    if (IsTruthy(value))
                    {
                        return EvalValue.FromScalar(CellEval.BoolValue(true));
                    }
                }

                return EvalValue.FromScalar(CellEval.BoolValue(false));
            }
            case "NOT":
            {
                if (node.Arguments.Count != 1)
                {
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "NOT requires 1 arg."));
                }

                var value = EvalAst(node.Arguments[0], self, visitingCells, visitingNames).Scalar;
                if (value.Type == DvcValueType.Error)
                {
                    return EvalValue.FromScalar(value);
                }

                return EvalValue.FromScalar(CellEval.BoolValue(!IsTruthy(value)));
            }
            case "ROW":
            {
                var row = self.Row;
                if (node.Arguments.Count > 0)
                {
                    if (TryResolveReferenceCoord(node.Arguments[0], self, true, out var refRow))
                    {
                        row = refRow;
                    }
                }

                return EvalValue.FromScalar(CellEval.NumberValue(row));
            }
            case "COLUMN":
            {
                var col = self.Col;
                if (node.Arguments.Count > 0)
                {
                    if (TryResolveReferenceCoord(node.Arguments[0], self, false, out var refCol))
                    {
                        col = refCol;
                    }
                }

                return EvalValue.FromScalar(CellEval.NumberValue(col));
            }
            case "NOW":
                return EvalValue.FromScalar(CellEval.NumberValue(45000.0 + (_committedEpoch % 10000)));
            case "RAND":
                return EvalValue.FromScalar(CellEval.NumberValue(_rng.NextDouble()));
            case "RANDARRAY":
            {
                var rows = EvalIntArg(node.Arguments, 0, 1, self, visitingCells, visitingNames);
                var cols = EvalIntArg(node.Arguments, 1, 1, self, visitingCells, visitingNames);
                var min = EvalDoubleArg(node.Arguments, 2, 0.0, self, visitingCells, visitingNames);
                var max = EvalDoubleArg(node.Arguments, 3, 1.0, self, visitingCells, visitingNames);
                rows = Math.Max(1, rows);
                cols = Math.Max(1, cols);
                var matrix = new CellEval[rows, cols];
                for (var r = 0; r < rows; r++)
                {
                    for (var c = 0; c < cols; c++)
                    {
                        matrix[r, c] = CellEval.NumberValue(min + _rng.NextDouble() * (max - min));
                    }
                }

                return EvalValue.FromMatrix(matrix);
            }
            case "SEQUENCE":
            {
                var rows = EvalIntArg(node.Arguments, 0, 1, self, visitingCells, visitingNames);
                var cols = EvalIntArg(node.Arguments, 1, 1, self, visitingCells, visitingNames);
                var start = EvalDoubleArg(node.Arguments, 2, 1.0, self, visitingCells, visitingNames);
                var step = EvalDoubleArg(node.Arguments, 3, 1.0, self, visitingCells, visitingNames);
                rows = Math.Max(1, rows);
                cols = Math.Max(1, cols);
                var matrix = new CellEval[rows, cols];
                var current = start;
                for (var r = 0; r < rows; r++)
                {
                    for (var c = 0; c < cols; c++)
                    {
                        matrix[r, c] = CellEval.NumberValue(current);
                        current += step;
                    }
                }

                return EvalValue.FromMatrix(matrix);
            }
            case "STREAM":
            {
                var period = EvalDoubleArg(node.Arguments, 0, 1.0, self, visitingCells, visitingNames);
                var key = self.Key;
                if (!_streams.TryGetValue(key, out var stream))
                {
                    stream = new StreamState(Math.Max(0.1, period));
                    _streams[key] = stream;
                }
                else
                {
                    stream.Period = Math.Max(0.1, period);
                }

                return EvalValue.FromScalar(CellEval.NumberValue(stream.Counter));
            }
            default:
                return EvalUdfOrUnknown(name, node, self, visitingCells, visitingNames);
        }
    }

    private EvalValue EvalUdfOrUnknown(string name, ExprNode node, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (!_udfs.TryGetValue(name, out var udf))
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.UnknownName, "Unknown function."));
        }

        var args = new DvcCellValue[node.Arguments.Count];
        for (var i = 0; i < node.Arguments.Count; i++)
        {
            var arg = EvalAst(node.Arguments[i], self, visitingCells, visitingNames).Scalar;
            args[i] = arg.ToCellValue();
        }

        var status = udf.Callback(args, out var result);
        if (status != DvcStatus.Ok)
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, $"UDF failed: {(int)status}"));
        }

        var scalar = result.Type switch
        {
            DvcValueType.Number => CellEval.NumberValue(result.Number),
            DvcValueType.Text => CellEval.TextValue(string.Empty),
            DvcValueType.Bool => CellEval.BoolValue(result.BoolVal != 0),
            DvcValueType.Blank => CellEval.BlankValue(),
            DvcValueType.Error => CellEval.ErrorValue(result.ErrorKind, "UDF error."),
            _ => CellEval.BlankValue(),
        };
        return EvalValue.FromScalar(scalar);
    }

    private int EvalIntArg(IReadOnlyList<ExprNode> args, int index, int fallback, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (index >= args.Count)
        {
            return fallback;
        }

        var v = EvalAst(args[index], self, visitingCells, visitingNames).Scalar;
        return v.TryAsNumber(out var n) ? (int)Math.Round(n) : fallback;
    }

    private double EvalDoubleArg(IReadOnlyList<ExprNode> args, int index, double fallback, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (index >= args.Count)
        {
            return fallback;
        }

        var v = EvalAst(args[index], self, visitingCells, visitingNames).Scalar;
        return v.TryAsNumber(out var n) ? n : fallback;
    }

    private string RewriteFormulaRefs(string formula, DvcStructuralOpKind op, ushort at)
    {
        return RefRegex.Replace(formula, m =>
        {
            var rawRef = m.Value.EndsWith('#') ? m.Value[..^1] : m.Value;
            var hasSpill = rawRef.Length != m.Value.Length;
            if (!A1Ref.TryParseCellRef(rawRef, out var token))
            {
                return m.Value;
            }

            var row = token.Row;
            var col = token.Col;
            switch (op)
            {
                case DvcStructuralOpKind.InsertRow:
                    if (!token.RowAbsolute && row >= at)
                    {
                        row++;
                    }
                    break;
                case DvcStructuralOpKind.DeleteRow:
                    if (row == at)
                    {
                        return "#REF!";
                    }

                    if (!token.RowAbsolute && row > at)
                    {
                        row--;
                    }
                    break;
                case DvcStructuralOpKind.InsertCol:
                    if (!token.ColAbsolute && col >= at)
                    {
                        col++;
                    }
                    break;
                case DvcStructuralOpKind.DeleteCol:
                    if (col == at)
                    {
                        return "#REF!";
                    }

                    if (!token.ColAbsolute && col > at)
                    {
                        col--;
                    }
                    break;
            }

            if (col <= 0 || row <= 0 || col > _bounds.MaxColumns || row > _bounds.MaxRows)
            {
                return "#REF!";
            }

            var rewritten = new CellAddressToken(col, row, token.ColAbsolute, token.RowAbsolute).ToA1();
            return hasSpill ? $"{rewritten}#" : rewritten;
        });
    }

    private static bool IsTruthy(CellEval value)
    {
        if (value.Type == DvcValueType.Bool)
        {
            return value.Bool != 0;
        }

        if (value.TryAsNumber(out var number))
        {
            return Math.Abs(number) > double.Epsilon;
        }

        var text = value.AsText();
        if (text.Equals("TRUE", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (text.Equals("FALSE", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        return !string.IsNullOrEmpty(text);
    }

    private static bool TryCompare(CellEval left, CellEval right, string op, out bool result)
    {
        result = false;
        if (op is not ("=" or "<>" or "<" or "<=" or ">" or ">="))
        {
            return false;
        }

        if (left.TryAsNumber(out var ln) && right.TryAsNumber(out var rn))
        {
            result = op switch
            {
                "=" => Math.Abs(ln - rn) <= double.Epsilon,
                "<>" => Math.Abs(ln - rn) > double.Epsilon,
                "<" => ln < rn,
                "<=" => ln <= rn,
                ">" => ln > rn,
                ">=" => ln >= rn,
                _ => false,
            };
            return true;
        }

        var compare = string.Compare(left.AsText(), right.AsText(), StringComparison.OrdinalIgnoreCase);
        result = op switch
        {
            "=" => compare == 0,
            "<>" => compare != 0,
            "<" => compare < 0,
            "<=" => compare <= 0,
            ">" => compare > 0,
            ">=" => compare >= 0,
            _ => false,
        };
        return true;
    }

    private bool TryResolveReferenceCoord(ExprNode node, DvcCellAddr self, bool row, out ushort value)
    {
        value = row ? self.Row : self.Col;
        DvcCellAddr addr;
        if (node.Kind == ExprKind.Cell || node.Kind == ExprKind.SpillRef)
        {
            addr = node.CellToken.ToAbsolute(self);
            value = row ? addr.Row : addr.Col;
            return true;
        }

        if (node.Kind == ExprKind.Range && node.RangeStart.HasValue)
        {
            addr = node.RangeStart.Value.ToAbsolute(self);
            value = row ? addr.Row : addr.Col;
            return true;
        }

        return false;
    }

    internal static IEnumerable<string> GetFunctionCalls(string formula)
    {
        foreach (Match match in FnRegex.Matches(formula.ToUpperInvariant()))
        {
            if (match.Groups.Count >= 2)
            {
                yield return match.Groups[1].Value;
            }
        }
    }

    internal enum ExprKind
    {
        Number,
        Text,
        Bool,
        Cell,
        SpillRef,
        Name,
        Unary,
        Binary,
        Function,
        Range,
    }

    internal sealed record ExprNode(ExprKind Kind)
    {
        public double Number { get; init; }
        public string Text { get; init; } = string.Empty;
        public int Bool { get; init; }
        public string Name { get; init; } = string.Empty;
        public string Operator { get; init; } = string.Empty;
        public IReadOnlyList<ExprNode> Arguments { get; init; } = Array.Empty<ExprNode>();
        public CellAddressToken CellToken { get; init; }
        public CellAddressToken? RangeStart { get; init; }
        public CellAddressToken? RangeEnd { get; init; }
    }
}
