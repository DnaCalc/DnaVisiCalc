using System.Globalization;
using System.Text.RegularExpressions;

namespace Dvc.Core;

public sealed partial class DvcEngineCore
{
    private static readonly HashSet<string> Builtins = new(StringComparer.Ordinal)
    {
        "SUM", "IF", "SEQUENCE", "RAND", "RANDARRAY", "NOW", "STREAM",
    };

    private static readonly Regex RefRegex = new(@"(?<![A-Z0-9_])\$?[A-Z]{1,3}\$?[1-9][0-9]*", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private static FormulaFeatures ClassifyFormula(string formula)
    {
        var text = formula.ToUpperInvariant();
        var hasVol = text.Contains("RAND(") || text.Contains("RANDARRAY(") || text.Contains("NOW(");
        var hasStream = text.Contains("STREAM(");
        return new FormulaFeatures(hasVol, hasStream, hasStream);
    }

    private bool TryValidateFormulaSyntax(string formula)
    {
        var parser = new ExprParser(formula, _ => EvalValue.FromScalar(CellEval.NumberValue(0.0)), _ => EvalValue.FromScalar(CellEval.NumberValue(0.0)));
        return parser.TryParse(out _);
    }

    private EvalValue EvaluateFormula(string formula, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (formula.Contains("#REF!", StringComparison.OrdinalIgnoreCase))
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Formula contains invalid reference."));
        }

        EvalValue ResolveCell(CellAddressToken token)
        {
            var addr = token.ToAbsolute(self);
            if (!IsInBounds(addr))
            {
                return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Ref, "Cell reference out of bounds."));
            }

            return EvalValue.FromScalar(EvaluateCell(addr, visitingCells, visitingNames));
        }

        EvalValue ResolveName(string name)
        {
            var norm = name.ToUpperInvariant();
            if (!_names.ContainsKey(norm))
            {
                return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.UnknownName, "Unknown name."));
            }

            return EvalValue.FromScalar(EvaluateName(norm, visitingCells, visitingNames));
        }

        var parser = new ExprParser(formula, ResolveCell, ResolveName);
        if (!parser.TryParse(out var ast))
        {
            return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Formula parse failed."));
        }

        return EvalAst(ast, self, visitingCells, visitingNames);
    }

    private EvalValue EvalAst(ExprNode node, DvcCellAddr self, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        switch (node.Kind)
        {
            case ExprKind.Number:
                return EvalValue.FromScalar(CellEval.NumberValue(node.Number));
            case ExprKind.Text:
                return EvalValue.FromScalar(CellEval.TextValue(node.Text));
            case ExprKind.Cell:
                return node.CellResolver!(node.CellToken);
            case ExprKind.Name:
                return node.NameResolver!(node.Name);
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
                    return EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.Value, "Binary non-number."));
                }

                return node.Operator switch
                {
                    "+" => EvalValue.FromScalar(CellEval.NumberValue(ln + rn)),
                    "-" => EvalValue.FromScalar(CellEval.NumberValue(ln - rn)),
                    "*" => EvalValue.FromScalar(CellEval.NumberValue(ln * rn)),
                    "/" => rn == 0 ? EvalValue.FromScalar(CellEval.ErrorValue(DvcCellErrorKind.DivZero, "Divide by zero.")) : EvalValue.FromScalar(CellEval.NumberValue(ln / rn)),
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

                    var value = EvalAst(arg, self, visitingCells, visitingNames).Scalar;
                    if (value.TryAsNumber(out var number))
                    {
                        total += number;
                    }
                }

                return EvalValue.FromScalar(CellEval.NumberValue(total));
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

    private static string RewriteFormulaRefs(string formula, DvcStructuralOpKind op, ushort at)
    {
        return RefRegex.Replace(formula, m =>
        {
            if (!A1Ref.TryParseCellRef(m.Value, out var token))
            {
                return m.Value;
            }

            var row = token.Row;
            var col = token.Col;
            switch (op)
            {
                case DvcStructuralOpKind.InsertRow:
                    if (row >= at) row++;
                    break;
                case DvcStructuralOpKind.DeleteRow:
                    if (row == at) return "#REF!";
                    if (row > at) row--;
                    break;
                case DvcStructuralOpKind.InsertCol:
                    if (col >= at) col++;
                    break;
                case DvcStructuralOpKind.DeleteCol:
                    if (col == at) return "#REF!";
                    if (col > at) col--;
                    break;
            }

            return new CellAddressToken(col, row, token.ColAbsolute, token.RowAbsolute).ToA1();
        });
    }

    internal enum ExprKind
    {
        Number,
        Text,
        Cell,
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
        public string Name { get; init; } = string.Empty;
        public string Operator { get; init; } = string.Empty;
        public IReadOnlyList<ExprNode> Arguments { get; init; } = Array.Empty<ExprNode>();
        public CellAddressToken CellToken { get; init; }
        public Func<CellAddressToken, EvalValue>? CellResolver { get; init; }
        public Func<string, EvalValue>? NameResolver { get; init; }
        public CellAddressToken? RangeStart { get; init; }
        public CellAddressToken? RangeEnd { get; init; }
    }
}
