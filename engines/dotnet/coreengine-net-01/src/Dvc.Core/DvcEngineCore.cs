using System.Globalization;

namespace Dvc.Core;

public sealed partial class DvcEngineCore
{
    private const ushort DefaultCols = 63;
    private const ushort DefaultRows = 254;

    private readonly DvcSheetBounds _bounds;
    private readonly Dictionary<long, InputEntry> _cells = [];
    private readonly Dictionary<string, InputEntry> _names = new(StringComparer.Ordinal);
    private readonly Dictionary<long, CellEval> _cellComputed = [];
    private readonly Dictionary<string, CellEval> _nameComputed = new(StringComparer.Ordinal);
    private readonly Dictionary<long, DvcCellFormat> _formats = [];
    private readonly Dictionary<long, SpillInfo> _spillAnchors = [];
    private readonly Dictionary<long, long> _spillMembers = [];
    private readonly Dictionary<long, StreamState> _streams = [];
    private readonly Dictionary<string, UdfState> _udfs = new(StringComparer.Ordinal);
    private readonly Dictionary<string, ControlState> _controls = new(StringComparer.Ordinal);
    private readonly Dictionary<string, ChartState> _charts = new(StringComparer.Ordinal);
    private readonly List<ChangeItem> _changes = [];
    private readonly Random _rng = new(7331);

    private ulong _committedEpoch;
    private ulong _stabilizedEpoch;
    private DvcRecalcMode _recalcMode = DvcRecalcMode.Automatic;
    private DvcIterationConfig _iterationConfig = DvcIterationConfig.Default;
    private DvcStatus _lastErrorKind = DvcStatus.Ok;
    private string _lastErrorMessage = string.Empty;
    private DvcRejectKind _lastRejectKind = DvcRejectKind.None;
    private DvcLastRejectContext _lastRejectContext = default;
    private bool _changeTrackingEnabled;

    public DvcEngineCore()
    {
        _bounds = new DvcSheetBounds { MaxColumns = DefaultCols, MaxRows = DefaultRows };
    }

    public DvcEngineCore(DvcSheetBounds bounds)
    {
        _bounds = bounds;
    }

    public delegate DvcStatus UdfCallback(DvcCellValue[] args, out DvcCellValue result);

    public DvcSheetBounds Bounds => _bounds;
    public ulong CommittedEpoch => _committedEpoch;
    public ulong StabilizedEpoch => _stabilizedEpoch;

    public DvcStatus ClearEngine()
    {
        _cells.Clear();
        _names.Clear();
        _cellComputed.Clear();
        _nameComputed.Clear();
        _formats.Clear();
        _spillAnchors.Clear();
        _spillMembers.Clear();
        _streams.Clear();
        _controls.Clear();
        _charts.Clear();
        _changes.Clear();
        _committedEpoch++;
        _stabilizedEpoch = _committedEpoch;
        return MarkOk();
    }

    public DvcStatus GetRecalcMode(out DvcRecalcMode mode)
    {
        mode = _recalcMode;
        return MarkOk();
    }

    public DvcStatus SetRecalcMode(DvcRecalcMode mode)
    {
        if (mode is not (DvcRecalcMode.Automatic or DvcRecalcMode.Manual))
        {
            return MarkError(DvcStatus.ErrInvalidArgument, "Unknown recalc mode.");
        }

        _recalcMode = mode;
        return MarkOk();
    }

    public DvcStatus IsStable(out int stable)
    {
        stable = _stabilizedEpoch == _committedEpoch ? 1 : 0;
        return MarkOk();
    }

    public DvcStatus SetCellNumber(DvcCellAddr addr, double value)
    {
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        _cells[addr.Key] = InputEntry.NumberValue(value);
        _cellComputed[addr.Key] = CellEval.NumberValue(value, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateCell(addr, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus SetCellText(DvcCellAddr addr, string text)
    {
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        _cells[addr.Key] = InputEntry.TextValue(text);
        _cellComputed[addr.Key] = CellEval.TextValue(text, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateCell(addr, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus SetCellFormula(DvcCellAddr addr, string formula)
    {
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (!TryValidateFormulaSyntax(formula))
        {
            return MarkError(DvcStatus.ErrParse, "Formula parse failed.");
        }

        _cells[addr.Key] = InputEntry.FormulaValue(formula, ClassifyFormula(formula));
        _cellComputed.Remove(addr.Key);
        RecordChange(ChangeItem.CreateCell(addr, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus ClearCell(DvcCellAddr addr)
    {
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        _cells.Remove(addr.Key);
        _cellComputed.Remove(addr.Key);
        _formats.Remove(addr.Key);
        _spillAnchors.Remove(addr.Key);
        _spillMembers.Remove(addr.Key);
        RecordChange(ChangeItem.CreateCell(addr, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus GetCellState(DvcCellAddr addr, out DvcCellState state)
    {
        state = default;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (!_cellComputed.TryGetValue(addr.Key, out var value))
        {
            value = CellEval.BlankValue(_stabilizedEpoch);
        }

        state = new DvcCellState
        {
            Value = value.ToCellValue(),
            ValueEpoch = value.Epoch,
            Stale = value.Epoch < _committedEpoch ? 1 : 0,
        };
        return MarkOk();
    }

    public DvcStatus GetCellText(DvcCellAddr addr, out string text)
    {
        text = string.Empty;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_cellComputed.TryGetValue(addr.Key, out var value) && value.Type == DvcValueType.Text)
        {
            text = value.Text;
        }

        return MarkOk();
    }

    public DvcStatus GetCellInputType(DvcCellAddr addr, out DvcInputType inputType)
    {
        inputType = DvcInputType.Empty;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_cells.TryGetValue(addr.Key, out var input))
        {
            inputType = input.Kind;
        }

        return MarkOk();
    }

    public DvcStatus GetCellInputText(DvcCellAddr addr, out string text)
    {
        text = string.Empty;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_cells.TryGetValue(addr.Key, out var input))
        {
            text = InputToText(input);
        }

        return MarkOk();
    }

    public DvcStatus SetNameNumber(string name, double value)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        _names[norm] = InputEntry.NumberValue(value);
        _nameComputed[norm] = CellEval.NumberValue(value, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus SetNameText(string name, string text)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        _names[norm] = InputEntry.TextValue(text);
        _nameComputed[norm] = CellEval.TextValue(text, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus SetNameFormula(string name, string formula)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (!TryValidateFormulaSyntax(formula))
        {
            return MarkError(DvcStatus.ErrParse, "Formula parse failed.");
        }

        _names[norm] = InputEntry.FormulaValue(formula, ClassifyFormula(formula));
        _nameComputed.Remove(norm);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus ClearName(string name)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        _names.Remove(norm);
        _nameComputed.Remove(norm);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus GetNameInputType(string name, out DvcInputType inputType)
    {
        inputType = DvcInputType.Empty;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_names.TryGetValue(norm, out var input))
        {
            inputType = input.Kind;
        }

        return MarkOk();
    }

    public DvcStatus GetNameInputText(string name, out string text)
    {
        text = string.Empty;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_names.TryGetValue(norm, out var input))
        {
            text = InputToText(input);
        }

        return MarkOk();
    }

    public DvcStatus Recalculate()
    {
        _spillAnchors.Clear();
        _spillMembers.Clear();
        var nextCells = new Dictionary<long, CellEval>();
        var nextNames = new Dictionary<string, CellEval>(StringComparer.Ordinal);
        var cycleFound = false;
        var limit = _iterationConfig.Enabled == 1 ? Math.Max(1u, _iterationConfig.MaxIterations) : 1u;
        var prevCells = _cellComputed.ToDictionary(kv => kv.Key, kv => kv.Value);
        var prevNames = _nameComputed.ToDictionary(kv => kv.Key, kv => kv.Value, StringComparer.Ordinal);

        for (var iteration = 0u; iteration < limit; iteration++)
        {
            nextCells.Clear();
            nextNames.Clear();
            cycleFound = false;
            foreach (var name in _names.Keys.OrderBy(x => x, StringComparer.Ordinal))
            {
                var value = EvaluateName(name, [], []);
                if (value.ErrorKind == DvcCellErrorKind.Cycle)
                {
                    cycleFound = true;
                }

                nextNames[name] = value with { Epoch = _committedEpoch };
            }

            foreach (var cellKey in _cells.Keys.OrderBy(x => x))
            {
                var addr = KeyToAddr(cellKey);
                var value = EvaluateCell(addr, [], []);
                if (value.ErrorKind == DvcCellErrorKind.Cycle)
                {
                    cycleFound = true;
                }

                nextCells[cellKey] = value with { Epoch = _committedEpoch };
            }

            if (_iterationConfig.Enabled == 0)
            {
                break;
            }

            if (Converged(prevCells, nextCells, _iterationConfig.ConvergenceTolerance) &&
                Converged(prevNames, nextNames, _iterationConfig.ConvergenceTolerance))
            {
                break;
            }

            prevCells = nextCells.ToDictionary(kv => kv.Key, kv => kv.Value);
            prevNames = nextNames.ToDictionary(kv => kv.Key, kv => kv.Value, StringComparer.Ordinal);
        }

        _cellComputed.Clear();
        foreach (var kv in nextCells)
        {
            _cellComputed[kv.Key] = kv.Value;
        }

        _nameComputed.Clear();
        foreach (var kv in nextNames)
        {
            _nameComputed[kv.Key] = kv.Value;
        }

        _stabilizedEpoch = _committedEpoch;
        ComputeCharts();
        if (cycleFound && _iterationConfig.Enabled == 0)
        {
            return MarkError(DvcStatus.ErrDependency, "Dependency cycle detected.");
        }

        return MarkOk();
    }

    public DvcStatus HasVolatileCells(out int has)
    {
        has = _cells.Values.Any(x => x.Features.HasVolatile) || _names.Values.Any(x => x.Features.HasVolatile) ? 1 : 0;
        return MarkOk();
    }

    public DvcStatus HasExternallyInvalidatedCells(out int has)
    {
        has = _cells.Values.Any(x => x.Features.HasExternal) || _names.Values.Any(x => x.Features.HasExternal) ? 1 : 0;
        return MarkOk();
    }

    public DvcStatus InvalidateVolatile()
    {
        _committedEpoch++;
        return _recalcMode == DvcRecalcMode.Automatic ? Recalculate() : MarkOk();
    }

    public DvcStatus HasStreamCells(out int has)
    {
        has = _streams.Count > 0 || _cells.Values.Any(x => x.Features.HasStream) ? 1 : 0;
        return MarkOk();
    }

    public DvcStatus TickStreams(double elapsedSecs, out int anyAdvanced)
    {
        anyAdvanced = 0;
        if (elapsedSecs <= 0)
        {
            return MarkOk();
        }

        foreach (var stream in _streams.Values)
        {
            stream.Accumulator += elapsedSecs;
            while (stream.Accumulator >= stream.Period)
            {
                stream.Accumulator -= stream.Period;
                stream.Counter++;
                anyAdvanced = 1;
            }
        }

        if (anyAdvanced == 1)
        {
            _committedEpoch++;
            if (_recalcMode == DvcRecalcMode.Automatic)
            {
                return Recalculate();
            }
        }

        return MarkOk();
    }

    public DvcStatus InvalidateUdf(string name)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (!_udfs.TryGetValue(norm, out var udf))
        {
            return MarkError(DvcStatus.ErrInvalidName, "Unknown UDF.");
        }

        if (udf.Volatility == DvcVolatility.ExternallyInvalidated)
        {
            _committedEpoch++;
            if (_recalcMode == DvcRecalcMode.Automatic)
            {
                return Recalculate();
            }
        }

        return MarkOk();
    }

    public DvcStatus GetCellFormat(DvcCellAddr addr, out DvcCellFormat format)
    {
        format = DvcCellFormat.Default;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_formats.TryGetValue(addr.Key, out var f))
        {
            format = f;
        }

        return MarkOk();
    }

    public DvcStatus SetCellFormat(DvcCellAddr addr, DvcCellFormat format)
    {
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (!ValidateFormat(format))
        {
            return MarkError(DvcStatus.ErrInvalidArgument, "Invalid format.");
        }

        if (format.IsDefault)
        {
            _formats.Remove(addr.Key);
        }
        else
        {
            _formats[addr.Key] = format;
        }

        RecordChange(ChangeItem.CreateFormat(addr, _committedEpoch + 1));
        _committedEpoch++;
        return MarkOk();
    }

    public DvcStatus GetSpillRole(DvcCellAddr addr, out DvcSpillRole role)
    {
        role = DvcSpillRole.None;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_spillAnchors.ContainsKey(addr.Key))
        {
            role = DvcSpillRole.Anchor;
        }
        else if (_spillMembers.ContainsKey(addr.Key))
        {
            role = DvcSpillRole.Member;
        }

        return MarkOk();
    }

    public DvcStatus GetSpillAnchor(DvcCellAddr addr, out DvcCellAddr anchor, out int found)
    {
        anchor = default;
        found = 0;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_spillMembers.TryGetValue(addr.Key, out var anchorKey))
        {
            anchor = KeyToAddr(anchorKey);
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus GetSpillRange(DvcCellAddr addr, out DvcCellRange range, out int found)
    {
        range = default;
        found = 0;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_spillAnchors.TryGetValue(addr.Key, out var anchor))
        {
            range = anchor.Range;
            found = 1;
        }
        else if (_spillMembers.TryGetValue(addr.Key, out var anchorKey) && _spillAnchors.TryGetValue(anchorKey, out var spill))
        {
            range = spill.Range;
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus InsertRow(ushort at) => ApplyStructural(DvcStructuralOpKind.InsertRow, at);
    public DvcStatus DeleteRow(ushort at) => ApplyStructural(DvcStructuralOpKind.DeleteRow, at);
    public DvcStatus InsertCol(ushort at) => ApplyStructural(DvcStructuralOpKind.InsertCol, at);
    public DvcStatus DeleteCol(ushort at) => ApplyStructural(DvcStructuralOpKind.DeleteCol, at);

    public DvcStatus GetIterationConfig(out DvcIterationConfig config)
    {
        config = _iterationConfig;
        return MarkOk();
    }

    public DvcStatus SetIterationConfig(DvcIterationConfig config)
    {
        if (config.Enabled is not (0 or 1) || config.MaxIterations == 0 || config.ConvergenceTolerance < 0)
        {
            return MarkError(DvcStatus.ErrInvalidArgument, "Invalid iteration config.");
        }

        _iterationConfig = config;
        return MarkOk();
    }

    public DvcStatus LastErrorMessage(out string message)
    {
        message = _lastErrorMessage;
        return MarkOk();
    }

    public DvcStatus LastErrorKind(out DvcStatus kind)
    {
        kind = _lastErrorKind;
        return MarkOk();
    }

    public DvcStatus LastRejectKind(out DvcRejectKind kind)
    {
        kind = _lastRejectKind;
        return MarkOk();
    }

    public DvcStatus LastRejectContext(out DvcLastRejectContext context)
    {
        context = _lastRejectContext;
        return MarkOk();
    }

    public DvcStatus CellErrorMessage(DvcCellAddr addr, out string message)
    {
        message = string.Empty;
        if (!ValidateCell(addr, out var status))
        {
            return status;
        }

        if (_cellComputed.TryGetValue(addr.Key, out var state) && state.Type == DvcValueType.Error)
        {
            message = state.ErrorMessage;
        }

        return MarkOk();
    }

    public DvcStatus ParseCellRef(string a1, out DvcCellAddr addr)
    {
        addr = default;
        if (!A1Ref.TryParseCellRef(a1, out var token))
        {
            return MarkError(DvcStatus.ErrInvalidAddress, "Invalid A1 address.");
        }

        addr = new DvcCellAddr((ushort)token.Col, (ushort)token.Row);
        return ValidateCell(addr, out var status) ? MarkOk() : status;
    }

    public static bool TryPaletteColorName(DvcPaletteColor color, out string name)
    {
        name = color switch
        {
            DvcPaletteColor.Mist => "MIST",
            DvcPaletteColor.Sage => "SAGE",
            DvcPaletteColor.Fern => "FERN",
            DvcPaletteColor.Moss => "MOSS",
            DvcPaletteColor.Olive => "OLIVE",
            DvcPaletteColor.Seafoam => "SEAFOAM",
            DvcPaletteColor.Lagoon => "LAGOON",
            DvcPaletteColor.Teal => "TEAL",
            DvcPaletteColor.Sky => "SKY",
            DvcPaletteColor.Cloud => "CLOUD",
            DvcPaletteColor.Sand => "SAND",
            DvcPaletteColor.Clay => "CLAY",
            DvcPaletteColor.Peach => "PEACH",
            DvcPaletteColor.Rose => "ROSE",
            DvcPaletteColor.Lavender => "LAVENDER",
            DvcPaletteColor.Slate => "SLATE",
            _ => string.Empty,
        };
        return name.Length > 0;
    }

    private DvcStatus ApplyStructural(DvcStructuralOpKind kind, ushort at)
    {
        if (at == 0)
        {
            return MarkError(DvcStatus.ErrOutOfBounds, "Index out of bounds.");
        }

        if (kind is DvcStructuralOpKind.InsertRow or DvcStructuralOpKind.DeleteRow && at > _bounds.MaxRows)
        {
            return MarkError(DvcStatus.ErrOutOfBounds, "Row index out of bounds.");
        }

        if (kind is DvcStructuralOpKind.InsertCol or DvcStructuralOpKind.DeleteCol && at > _bounds.MaxColumns)
        {
            return MarkError(DvcStatus.ErrOutOfBounds, "Column index out of bounds.");
        }

        if (_spillAnchors.Values.Any(spill => Intersects(spill.Range, kind, at)))
        {
            return MarkReject(DvcStatus.RejectStructuralConstraint, DvcRejectKind.StructuralConstraint, new DvcLastRejectContext
            {
                RejectKind = DvcRejectKind.StructuralConstraint,
                OpKind = kind,
                OpIndex = at,
            });
        }

        var nextCells = new Dictionary<long, InputEntry>();
        foreach (var kv in _cells)
        {
            var old = KeyToAddr(kv.Key);
            var shifted = ShiftAddress(old, kind, at, out var keep);
            if (!keep || !IsInBounds(shifted))
            {
                continue;
            }

            var entry = kv.Value;
            if (entry.Kind == DvcInputType.Formula)
            {
                var rewritten = RewriteFormulaRefs(entry.Formula, kind, at);
                entry = InputEntry.FormulaValue(rewritten, ClassifyFormula(rewritten));
            }

            nextCells[shifted.Key] = entry;
        }

        _cells.Clear();
        foreach (var kv in nextCells)
        {
            _cells[kv.Key] = kv.Value;
        }

        var nextFormats = new Dictionary<long, DvcCellFormat>();
        foreach (var kv in _formats)
        {
            var old = KeyToAddr(kv.Key);
            var shifted = ShiftAddress(old, kind, at, out var keep);
            if (keep && IsInBounds(shifted))
            {
                nextFormats[shifted.Key] = kv.Value;
            }
        }

        _formats.Clear();
        foreach (var kv in nextFormats)
        {
            _formats[kv.Key] = kv.Value;
        }

        foreach (var name in _names.Keys.ToArray())
        {
            var value = _names[name];
            if (value.Kind == DvcInputType.Formula)
            {
                var rewritten = RewriteFormulaRefs(value.Formula, kind, at);
                _names[name] = InputEntry.FormulaValue(rewritten, ClassifyFormula(rewritten));
            }
        }

        return CommitMutation(true);
    }

    private static bool Intersects(DvcCellRange range, DvcStructuralOpKind kind, ushort at) =>
        kind switch
        {
            DvcStructuralOpKind.InsertRow or DvcStructuralOpKind.DeleteRow => at >= range.Start.Row && at <= range.End.Row,
            DvcStructuralOpKind.InsertCol or DvcStructuralOpKind.DeleteCol => at >= range.Start.Col && at <= range.End.Col,
            _ => false,
        };

    private static DvcCellAddr ShiftAddress(DvcCellAddr addr, DvcStructuralOpKind kind, ushort at, out bool keep)
    {
        keep = true;
        var col = addr.Col;
        var row = addr.Row;
        switch (kind)
        {
            case DvcStructuralOpKind.InsertRow:
                if (row >= at) row++;
                break;
            case DvcStructuralOpKind.DeleteRow:
                if (row == at) keep = false;
                else if (row > at) row--;
                break;
            case DvcStructuralOpKind.InsertCol:
                if (col >= at) col++;
                break;
            case DvcStructuralOpKind.DeleteCol:
                if (col == at) keep = false;
                else if (col > at) col--;
                break;
        }

        return new DvcCellAddr((ushort)col, (ushort)row);
    }

    private DvcStatus CommitMutation(bool forceRecalc)
    {
        _committedEpoch++;
        if (_recalcMode == DvcRecalcMode.Automatic || forceRecalc)
        {
            return Recalculate();
        }

        return MarkOk();
    }

    private CellEval EvaluateCell(DvcCellAddr addr, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (_spillMembers.TryGetValue(addr.Key, out var anchorKey) && _spillAnchors.TryGetValue(anchorKey, out var spill))
        {
            return spill.GetValue(addr);
        }

        if (visitingCells.Contains(addr.Key))
        {
            return _iterationConfig.Enabled == 1 ? CellEval.NumberValue(0.0) : CellEval.ErrorValue(DvcCellErrorKind.Cycle, "Circular reference.");
        }

        if (!_cells.TryGetValue(addr.Key, out var input))
        {
            return CellEval.BlankValue(_stabilizedEpoch);
        }

        if (input.Kind == DvcInputType.Number)
        {
            return CellEval.NumberValue(input.Number);
        }

        if (input.Kind == DvcInputType.Text)
        {
            return CellEval.TextValue(input.Text);
        }

        visitingCells.Add(addr.Key);
        var eval = EvaluateFormula(input.Formula, addr, visitingCells, visitingNames);
        visitingCells.Remove(addr.Key);
        if (eval.Matrix is null)
        {
            return eval.Scalar;
        }

        var applied = TryApplySpill(addr, eval.Matrix, out var top);
        return applied ? top : CellEval.ErrorValue(DvcCellErrorKind.Spill, "Spill blocked.");
    }

    private CellEval EvaluateName(string name, HashSet<long> visitingCells, HashSet<string> visitingNames)
    {
        if (visitingNames.Contains(name))
        {
            return _iterationConfig.Enabled == 1 ? CellEval.NumberValue(0.0) : CellEval.ErrorValue(DvcCellErrorKind.Cycle, "Circular name.");
        }

        if (!_names.TryGetValue(name, out var input))
        {
            return CellEval.BlankValue(_stabilizedEpoch);
        }

        if (input.Kind == DvcInputType.Number)
        {
            return CellEval.NumberValue(input.Number);
        }

        if (input.Kind == DvcInputType.Text)
        {
            return CellEval.TextValue(input.Text);
        }

        visitingNames.Add(name);
        var eval = EvaluateFormula(input.Formula, default, visitingCells, visitingNames);
        visitingNames.Remove(name);
        return eval.Scalar;
    }

    private bool TryApplySpill(DvcCellAddr anchor, CellEval[,] matrix, out CellEval top)
    {
        top = matrix[0, 0];
        var rows = matrix.GetLength(0);
        var cols = matrix.GetLength(1);
        var end = new DvcCellAddr((ushort)(anchor.Col + cols - 1), (ushort)(anchor.Row + rows - 1));
        if (!IsInBounds(end))
        {
            return false;
        }

        for (var r = 0; r < rows; r++)
        {
            for (var c = 0; c < cols; c++)
            {
                var addr = new DvcCellAddr((ushort)(anchor.Col + c), (ushort)(anchor.Row + r));
                if (addr.Key == anchor.Key)
                {
                    continue;
                }

                if (_cells.ContainsKey(addr.Key))
                {
                    return false;
                }
            }
        }

        var range = new DvcCellRange { Start = anchor, End = end };
        var spill = new SpillInfo(anchor, matrix, range);
        _spillAnchors[anchor.Key] = spill;
        for (var r = 0; r < rows; r++)
        {
            for (var c = 0; c < cols; c++)
            {
                var addr = new DvcCellAddr((ushort)(anchor.Col + c), (ushort)(anchor.Row + r));
                if (addr.Key == anchor.Key)
                {
                    continue;
                }

                _spillMembers[addr.Key] = anchor.Key;
            }
        }

        RecordChange(ChangeItem.CreateSpill(anchor, range, _committedEpoch));
        return true;
    }

    private static bool Converged<TKey>(Dictionary<TKey, CellEval> previous, Dictionary<TKey, CellEval> current, double tolerance) where TKey : notnull
    {
        foreach (var kv in current)
        {
            if (!previous.TryGetValue(kv.Key, out var oldValue))
            {
                return false;
            }

            var next = kv.Value;
            if (oldValue.Type != next.Type)
            {
                return false;
            }

            if (next.Type == DvcValueType.Number && Math.Abs(oldValue.Number - next.Number) > tolerance)
            {
                return false;
            }

            if (next.Type == DvcValueType.Text && oldValue.Text != next.Text)
            {
                return false;
            }
        }

        return true;
    }

    private bool ValidateCell(DvcCellAddr addr, out DvcStatus status)
    {
        if (!IsInBounds(addr))
        {
            status = MarkError(DvcStatus.ErrOutOfBounds, "Cell out of bounds.");
            return false;
        }

        status = DvcStatus.Ok;
        return true;
    }

    private bool IsInBounds(DvcCellAddr addr) =>
        addr.Col > 0 && addr.Col <= _bounds.MaxColumns && addr.Row > 0 && addr.Row <= _bounds.MaxRows;

    private bool NormalizeName(string name, out string normalized, out DvcStatus status)
    {
        normalized = name.Trim().ToUpperInvariant();
        if (normalized.Length == 0)
        {
            status = MarkError(DvcStatus.ErrInvalidName, "Name cannot be empty.");
            return false;
        }

        if (!System.Text.RegularExpressions.Regex.IsMatch(normalized, "^[A-Z_][A-Z0-9_]*$"))
        {
            status = MarkError(DvcStatus.ErrInvalidName, "Invalid name format.");
            return false;
        }

        if (A1Ref.TryParseCellRef(normalized, out _))
        {
            status = MarkError(DvcStatus.ErrInvalidName, "Name conflicts with cell ref.");
            return false;
        }

        if (Builtins.Contains(normalized) || normalized is "TRUE" or "FALSE")
        {
            status = MarkError(DvcStatus.ErrInvalidName, "Name conflicts with reserved literal/function.");
            return false;
        }

        status = DvcStatus.Ok;
        return true;
    }

    private static string InputToText(InputEntry input) =>
        input.Kind switch
        {
            DvcInputType.Number => input.Number.ToString("G17", CultureInfo.InvariantCulture),
            DvcInputType.Text => input.Text,
            DvcInputType.Formula => input.Formula,
            _ => string.Empty,
        };

    private static DvcCellAddr KeyToAddr(long key) => new((ushort)(key & 0xFFFF), (ushort)(key >> 32));

    private void RecordChange(ChangeItem change)
    {
        if (_changeTrackingEnabled)
        {
            _changes.Add(change);
        }
    }

    private static bool ValidateFormat(DvcCellFormat format)
    {
        if (format.HasDecimals is not (0 or 1))
        {
            return false;
        }

        if (format.HasDecimals == 1 && format.Decimals > 9)
        {
            return false;
        }

        if (format.Bold is not (0 or 1) || format.Italic is not (0 or 1))
        {
            return false;
        }

        return IsPalette(format.Fg) && IsPalette(format.Bg);
    }

    private static bool IsPalette(DvcPaletteColor color) => color == DvcPaletteColor.None || (int)color is >= 0 and <= 15;

    private DvcStatus MarkOk()
    {
        _lastErrorKind = DvcStatus.Ok;
        _lastErrorMessage = string.Empty;
        _lastRejectKind = DvcRejectKind.None;
        _lastRejectContext = default;
        return DvcStatus.Ok;
    }

    private DvcStatus MarkError(DvcStatus status, string message)
    {
        _lastErrorKind = status;
        _lastErrorMessage = message;
        _lastRejectKind = DvcRejectKind.None;
        _lastRejectContext = default;
        return status;
    }

    private DvcStatus MarkReject(DvcStatus status, DvcRejectKind rejectKind, DvcLastRejectContext context)
    {
        _lastErrorKind = DvcStatus.Ok;
        _lastErrorMessage = string.Empty;
        _lastRejectKind = rejectKind;
        _lastRejectContext = context;
        return status;
    }
}
