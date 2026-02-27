namespace Dvc.Core;

public sealed partial class DvcEngineCore
{
    public DvcStatus UdfRegister(string name, DvcVolatility volatility, UdfCallback callback)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        _udfs[norm] = new UdfState(volatility, callback);
        return MarkOk();
    }

    public DvcStatus UdfUnregister(string name, out int found)
    {
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_udfs.Remove(norm))
        {
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ControlDefine(string name, DvcControlDef def)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (!ValidateControlDef(def))
        {
            return MarkError(DvcStatus.ErrInvalidArgument, "Invalid control definition.");
        }

        _controls[norm] = new ControlState(def);
        var initial = def.Kind == DvcControlKind.Slider ? def.Min : 0.0;
        _names[norm] = InputEntry.NumberValue(initial);
        _nameComputed[norm] = CellEval.NumberValue(initial, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus ControlRemove(string name, out int found)
    {
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_controls.Remove(norm))
        {
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ControlSetValue(string name, double value)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (!_controls.TryGetValue(norm, out var control))
        {
            return MarkError(DvcStatus.ErrInvalidName, "Control not found.");
        }

        if (!NormalizeControlValue(control.Def, value, out var normalized))
        {
            return MarkError(DvcStatus.ErrInvalidArgument, "Invalid control value.");
        }

        _names[norm] = InputEntry.NumberValue(normalized);
        _nameComputed[norm] = CellEval.NumberValue(normalized, _committedEpoch + 1);
        RecordChange(ChangeItem.CreateName(norm, _committedEpoch + 1));
        return CommitMutation(false);
    }

    public DvcStatus ControlGetValue(string name, out double value, out int found)
    {
        value = 0.0;
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_controls.ContainsKey(norm) && _nameComputed.TryGetValue(norm, out var computed) && computed.Type == DvcValueType.Number)
        {
            value = computed.Number;
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ControlGetDef(string name, out DvcControlDef def, out int found)
    {
        def = default;
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_controls.TryGetValue(norm, out var control))
        {
            def = control.Def;
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ChartDefine(string name, DvcChartDef def)
    {
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (!IsInBounds(def.SourceRange.Start) || !IsInBounds(def.SourceRange.End))
        {
            return MarkError(DvcStatus.ErrOutOfBounds, "Chart source range out of bounds.");
        }

        _charts[norm] = new ChartState(def);
        return MarkOk();
    }

    public DvcStatus ChartRemove(string name, out int found)
    {
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_charts.Remove(norm))
        {
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ChartGetOutput(string name, out ChartOutput? output, out int found)
    {
        output = null;
        found = 0;
        if (!NormalizeName(name, out var norm, out var status))
        {
            return status;
        }

        if (_charts.TryGetValue(norm, out var chart) && chart.Output is not null)
        {
            output = chart.Output;
            found = 1;
        }

        return MarkOk();
    }

    public DvcStatus ChangeTrackingEnable()
    {
        _changeTrackingEnabled = true;
        return MarkOk();
    }

    public DvcStatus ChangeTrackingDisable()
    {
        _changeTrackingEnabled = false;
        _changes.Clear();
        return MarkOk();
    }

    public DvcStatus ChangeTrackingIsEnabled(out int enabled)
    {
        enabled = _changeTrackingEnabled ? 1 : 0;
        return MarkOk();
    }

    public DvcStatus DrainChanges(out ChangeIterator iter)
    {
        iter = new ChangeIterator(_changes.ToArray());
        _changes.Clear();
        return MarkOk();
    }

    public DvcStatus CreateCellIterator(out CellIterator iter)
    {
        var entries = _cells.OrderBy(kv => kv.Key)
            .Select(kv => new CellIterEntry(KeyToAddr(kv.Key), kv.Value.Kind, InputToText(kv.Value)))
            .ToArray();
        iter = new CellIterator(entries);
        return MarkOk();
    }

    public DvcStatus CreateNameIterator(out NameIterator iter)
    {
        var entries = _names.OrderBy(kv => kv.Key, StringComparer.Ordinal)
            .Select(kv => new NameIterEntry(kv.Key, kv.Value.Kind, InputToText(kv.Value)))
            .ToArray();
        iter = new NameIterator(entries);
        return MarkOk();
    }

    public DvcStatus CreateFormatIterator(out FormatIterator iter)
    {
        var entries = _formats.OrderBy(kv => kv.Key)
            .Select(kv => new FormatIterEntry(KeyToAddr(kv.Key), kv.Value))
            .ToArray();
        iter = new FormatIterator(entries);
        return MarkOk();
    }

    public DvcStatus CreateControlIterator(out ControlIterator iter)
    {
        var entries = _controls.OrderBy(kv => kv.Key, StringComparer.Ordinal)
            .Select(kv =>
            {
                var value = _nameComputed.TryGetValue(kv.Key, out var eval) && eval.Type == DvcValueType.Number ? eval.Number : 0.0;
                return new ControlIterEntry(kv.Key, kv.Value.Def, value);
            })
            .ToArray();
        iter = new ControlIterator(entries);
        return MarkOk();
    }

    public DvcStatus CreateChartIterator(out ChartIterator iter)
    {
        var entries = _charts.OrderBy(kv => kv.Key, StringComparer.Ordinal)
            .Select(kv => new ChartIterEntry(kv.Key, kv.Value.Def))
            .ToArray();
        iter = new ChartIterator(entries);
        return MarkOk();
    }

    private void ComputeCharts()
    {
        foreach (var kv in _charts)
        {
            var range = kv.Value.Def.SourceRange;
            var labels = new List<string>();
            var values = new List<double>();
            var rowStart = Math.Min(range.Start.Row, range.End.Row);
            var rowEnd = Math.Max(range.Start.Row, range.End.Row);
            var colStart = Math.Min(range.Start.Col, range.End.Col);
            var colEnd = Math.Max(range.Start.Col, range.End.Col);
            for (var r = rowStart; r <= rowEnd; r++)
            {
                var labelCell = new DvcCellAddr(colStart, r);
                if (_cellComputed.TryGetValue(labelCell.Key, out var labelEval))
                {
                    labels.Add(labelEval.AsText());
                }
                else
                {
                    labels.Add(string.Empty);
                }

                for (var c = colStart + 1; c <= colEnd; c++)
                {
                    var valueCell = new DvcCellAddr((ushort)c, (ushort)r);
                    if (_cellComputed.TryGetValue(valueCell.Key, out var valueEval) && valueEval.TryAsNumber(out var number))
                    {
                        values.Add(number);
                    }
                }
            }

            kv.Value.Output = new ChartOutput("SERIES1", labels, values);
            RecordChange(ChangeItem.CreateChart(kv.Key, _committedEpoch));
        }
    }

    private static bool ValidateControlDef(DvcControlDef def)
    {
        if (def.Kind == DvcControlKind.Slider)
        {
            return def.Step > 0 && def.Min <= def.Max;
        }

        return def.Kind is DvcControlKind.Checkbox or DvcControlKind.Button;
    }

    private static bool NormalizeControlValue(DvcControlDef def, double input, out double output)
    {
        output = input;
        switch (def.Kind)
        {
            case DvcControlKind.Slider:
                output = Math.Clamp(input, def.Min, def.Max);
                return true;
            case DvcControlKind.Checkbox:
                if (input is not (0.0 or 1.0))
                {
                    return false;
                }

                output = input;
                return true;
            case DvcControlKind.Button:
                output = 0.0;
                return true;
            default:
                return false;
        }
    }
}

public readonly record struct ChangeItem(DvcChangeType Type, ulong Epoch)
{
    public DvcCellAddr Cell { get; init; }
    public string Name { get; init; } = string.Empty;
    public DvcCellRange Spill { get; init; }

    public static ChangeItem CreateCell(DvcCellAddr addr, ulong epoch) => new(DvcChangeType.CellValue, epoch) { Cell = addr };
    public static ChangeItem CreateName(string name, ulong epoch) => new(DvcChangeType.NameValue, epoch) { Name = name };
    public static ChangeItem CreateFormat(DvcCellAddr addr, ulong epoch) => new(DvcChangeType.CellFormat, epoch) { Cell = addr };
    public static ChangeItem CreateSpill(DvcCellAddr addr, DvcCellRange range, ulong epoch) => new(DvcChangeType.SpillRegion, epoch) { Cell = addr, Spill = range };
    public static ChangeItem CreateChart(string name, ulong epoch) => new(DvcChangeType.ChartOutput, epoch) { Name = name };
}

public sealed class CellIterator(CellIterEntry[] entries)
{
    private int _index = -1;
    public bool Next(out CellIterEntry entry)
    {
        if (_index + 1 >= entries.Length)
        {
            entry = default;
            return false;
        }

        _index++;
        entry = entries[_index];
        return true;
    }

    public bool Current(out CellIterEntry entry)
    {
        if (_index < 0 || _index >= entries.Length)
        {
            entry = default;
            return false;
        }

        entry = entries[_index];
        return true;
    }
}

public readonly record struct CellIterEntry(DvcCellAddr Addr, DvcInputType InputType, string Text);

public sealed class NameIterator(NameIterEntry[] entries)
{
    private int _index = -1;
    public bool Next(out NameIterEntry entry)
    {
        if (_index + 1 >= entries.Length)
        {
            entry = default;
            return false;
        }

        _index++;
        entry = entries[_index];
        return true;
    }

    public bool Current(out NameIterEntry entry)
    {
        if (_index < 0 || _index >= entries.Length)
        {
            entry = default;
            return false;
        }

        entry = entries[_index];
        return true;
    }
}

public readonly record struct NameIterEntry(string Name, DvcInputType InputType, string Text);

public sealed class FormatIterator(FormatIterEntry[] entries)
{
    private int _index = -1;
    public bool Next(out FormatIterEntry entry)
    {
        if (_index + 1 >= entries.Length)
        {
            entry = default;
            return false;
        }

        _index++;
        entry = entries[_index];
        return true;
    }
}

public readonly record struct FormatIterEntry(DvcCellAddr Addr, DvcCellFormat Format);

public sealed class ControlIterator(ControlIterEntry[] entries)
{
    private int _index = -1;
    public bool Next(out ControlIterEntry entry)
    {
        if (_index + 1 >= entries.Length)
        {
            entry = default;
            return false;
        }

        _index++;
        entry = entries[_index];
        return true;
    }
}

public readonly record struct ControlIterEntry(string Name, DvcControlDef Def, double Value);

public sealed class ChartIterator(ChartIterEntry[] entries)
{
    private int _index = -1;
    public bool Next(out ChartIterEntry entry)
    {
        if (_index + 1 >= entries.Length)
        {
            entry = default;
            return false;
        }

        _index++;
        entry = entries[_index];
        return true;
    }
}

public readonly record struct ChartIterEntry(string Name, DvcChartDef Def);

public sealed class ChangeIterator(ChangeItem[] entries)
{
    private int _index = -1;
    public bool Next(out ChangeItem item)
    {
        if (_index + 1 >= entries.Length)
        {
            item = default;
            return false;
        }

        _index++;
        item = entries[_index];
        return true;
    }

    public bool Current(out ChangeItem item)
    {
        if (_index < 0 || _index >= entries.Length)
        {
            item = default;
            return false;
        }

        item = entries[_index];
        return true;
    }
}
