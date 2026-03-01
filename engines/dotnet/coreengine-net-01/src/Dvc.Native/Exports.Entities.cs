using System.Runtime.InteropServices;
using System.Text;
using Dvc.Core;

namespace Dvc.Native;

public static unsafe partial class Exports
{
    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_iterate")]
    public static int CellIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.CreateCellIterator(out var iterator);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new CellIterHandle(iterator));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_iterator_next")]
    public static int CellIteratorNext(IntPtr iterPtr, [DNNE.C99Type("DvcCellAddr*")] DvcCellAddr* outAddr, [DNNE.C99Type("int32_t*")] DvcInputType* outType, int* done)
    {
        if (done == null || outAddr == null || outType == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<CellIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!handle.Iterator.Next(out var entry))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        handle.Current = entry;
        *outAddr = entry.Addr;
        *outType = entry.InputType;
        *done = 0;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_iterator_get_text")]
    public static int CellIteratorGetText(IntPtr iterPtr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<CellIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        return NativeHelpers.WriteUtf8(handle.Current.Text, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_iterator_destroy")]
    public static int CellIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_iterate")]
    public static int NameIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.CreateNameIterator(out var iterator);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new NameIterHandle(iterator));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_iterator_next")]
    public static int NameIteratorNext(IntPtr iterPtr, byte* nameBuf, uint nameBufLen, uint* nameLen, [DNNE.C99Type("int32_t*")] DvcInputType* inputType, int* done)
    {
        if (done == null || nameLen == null || inputType == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<NameIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        NameIterEntry entry;
        if (handle.HasPending)
        {
            entry = handle.Pending;
        }
        else if (!handle.Iterator.Next(out entry))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        handle.Current = entry;
        *inputType = entry.InputType;
        *done = 0;
        var status = NativeHelpers.WriteUtf8(entry.Name, nameBuf, nameBufLen, nameLen);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        var requiredLen = (uint)Encoding.UTF8.GetByteCount(entry.Name);
        if (nameBuf != null && nameBufLen >= requiredLen)
        {
            handle.HasPending = false;
        }
        else
        {
            handle.Pending = entry;
            handle.HasPending = true;
        }

        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_iterator_get_text")]
    public static int NameIteratorGetText(IntPtr iterPtr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<NameIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        return NativeHelpers.WriteUtf8(handle.Current.Text, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_iterator_destroy")]
    public static int NameIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_format_iterate")]
    public static int FormatIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.CreateFormatIterator(out var iterator);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new FormatIterHandle(iterator));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_format_iterator_next")]
    public static int FormatIteratorNext(IntPtr iterPtr, [DNNE.C99Type("DvcCellAddr*")] DvcCellAddr* outAddr, [DNNE.C99Type("DvcCellFormat*")] DvcCellFormat* outFormat, int* done)
    {
        if (done == null || outAddr == null || outFormat == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<FormatIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!handle.Iterator.Next(out var entry))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        *outAddr = entry.Addr;
        *outFormat = entry.Format;
        *done = 0;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_format_iterator_destroy")]
    public static int FormatIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_define")]
    public static int ControlDefine(IntPtr enginePtr, byte* name, uint nameLen, [DNNE.C99Type("DvcControlDef*")] DvcControlDef* def)
    {
        if (def == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ControlDefine(text, *def));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_remove")]
    public static int ControlRemove(IntPtr enginePtr, byte* name, uint nameLen, int* found)
    {
        if (found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ControlRemove(text, out *found));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_set_value")]
    public static int ControlSetValue(IntPtr enginePtr, byte* name, uint nameLen, double value)
    {
        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ControlSetValue(text, value));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_get_value")]
    public static int ControlGetValue(IntPtr enginePtr, byte* name, uint nameLen, double* outValue, int* found)
    {
        if (outValue == null || found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var status = engine.ControlGetValue(text, out var value, out var isFound);
            *outValue = value;
            *found = isFound;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_get_def")]
    public static int ControlGetDef(IntPtr enginePtr, byte* name, uint nameLen, [DNNE.C99Type("DvcControlDef*")] DvcControlDef* outDef, int* found)
    {
        if (outDef == null || found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var status = engine.ControlGetDef(text, out var def, out var isFound);
            *outDef = def;
            *found = isFound;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_iterate")]
    public static int ControlIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.CreateControlIterator(out var iterator);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new ControlIterHandle(iterator));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_iterator_next")]
    public static int ControlIteratorNext(IntPtr iterPtr, byte* nameBuf, uint nameBufLen, uint* nameLen, [DNNE.C99Type("DvcControlDef*")] DvcControlDef* def, double* value, int* done)
    {
        if (done == null || nameLen == null || def == null || value == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ControlIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        ControlIterEntry entry;
        if (handle.HasPending)
        {
            entry = handle.Pending;
        }
        else if (!handle.Iterator.Next(out entry))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        *def = entry.Def;
        *value = entry.Value;
        *done = 0;
        var status = NativeHelpers.WriteUtf8(entry.Name, nameBuf, nameBufLen, nameLen);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        var requiredLen = (uint)Encoding.UTF8.GetByteCount(entry.Name);
        if (nameBuf != null && nameBufLen >= requiredLen)
        {
            handle.HasPending = false;
        }
        else
        {
            handle.Pending = entry;
            handle.HasPending = true;
        }

        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_control_iterator_destroy")]
    public static int ControlIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_define")]
    public static int ChartDefine(IntPtr enginePtr, byte* name, uint nameLen, [DNNE.C99Type("DvcChartDef*")] DvcChartDef* def)
    {
        if (def == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ChartDefine(text, *def));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_remove")]
    public static int ChartRemove(IntPtr enginePtr, byte* name, uint nameLen, int* found)
    {
        if (found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ChartRemove(text, out *found));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_get_output")]
    public static int ChartGetOutput(IntPtr enginePtr, byte* name, uint nameLen, IntPtr* outOutput, int* found)
    {
        if (outOutput == null || found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var status = engine.ChartGetOutput(text, out var output, out var isFound);
            *found = isFound;
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            if (isFound == 0 || output is null)
            {
                *outOutput = IntPtr.Zero;
                return (int)DvcStatus.Ok;
            }

            *outOutput = HandleStore.Alloc(new ChartOutputHandle(output));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_output_series_count")]
    public static int ChartOutputSeriesCount(IntPtr outputPtr, uint* outCount)
    {
        if (outCount == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChartOutputHandle>(outputPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *outCount = (uint)handle.Output.Series.Count;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_output_label_count")]
    public static int ChartOutputLabelCount(IntPtr outputPtr, uint* outCount)
    {
        if (outCount == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChartOutputHandle>(outputPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *outCount = (uint)handle.Output.Labels.Count;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_output_label")]
    public static int ChartOutputLabel(IntPtr outputPtr, uint index, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<ChartOutputHandle>(outputPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (index >= handle.Output.Labels.Count)
        {
            return (int)DvcStatus.ErrOutOfBounds;
        }

        return NativeHelpers.WriteUtf8(handle.Output.Labels[(int)index], buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_output_series_name")]
    public static int ChartOutputSeriesName(IntPtr outputPtr, uint seriesIndex, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<ChartOutputHandle>(outputPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (seriesIndex >= handle.Output.Series.Count)
        {
            return (int)DvcStatus.ErrOutOfBounds;
        }

        return NativeHelpers.WriteUtf8(handle.Output.Series[(int)seriesIndex].Name, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_output_series_values")]
    public static int ChartOutputSeriesValues(IntPtr outputPtr, uint seriesIndex, double* buf, uint bufLen, uint* outCount)
    {
        if (outCount == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChartOutputHandle>(outputPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (seriesIndex >= handle.Output.Series.Count)
        {
            return (int)DvcStatus.ErrOutOfBounds;
        }

        var values = handle.Output.Series[(int)seriesIndex].Values;
        *outCount = (uint)values.Count;
        if (buf != null && bufLen > 0)
        {
            var count = Math.Min((int)bufLen, values.Count);
            for (var i = 0; i < count; i++)
            {
                buf[i] = values[i];
            }
        }

        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_iterate")]
    public static int ChartIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.CreateChartIterator(out var iterator);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new ChartIterHandle(iterator));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_iterator_next")]
    public static int ChartIteratorNext(IntPtr iterPtr, byte* nameBuf, uint nameBufLen, uint* nameLen, [DNNE.C99Type("DvcChartDef*")] DvcChartDef* def, int* done)
    {
        if (done == null || nameLen == null || def == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChartIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        ChartIterEntry entry;
        if (handle.HasPending)
        {
            entry = handle.Pending;
        }
        else if (!handle.Iterator.Next(out entry))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        *def = entry.Def;
        *done = 0;
        var status = NativeHelpers.WriteUtf8(entry.Name, nameBuf, nameBufLen, nameLen);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        var requiredLen = (uint)Encoding.UTF8.GetByteCount(entry.Name);
        if (nameBuf != null && nameBufLen >= requiredLen)
        {
            handle.HasPending = false;
        }
        else
        {
            handle.Pending = entry;
            handle.HasPending = true;
        }

        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_chart_iterator_destroy")]
    public static int ChartIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_udf_register")]
    public static int UdfRegister(IntPtr enginePtr, byte* name, uint nameLen, IntPtr callback, void* userData, [DNNE.C99Type("int32_t")] DvcVolatility volatility)
    {
        if (callback == IntPtr.Zero)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var fn = (delegate* unmanaged[Cdecl]<void*, DvcCellValue*, uint, DvcCellValue*, int>)callback;
            DvcEngineCore.UdfCallback callbackBridge = (DvcCellValue[] args, out DvcCellValue result) =>
            {
                fixed (DvcCellValue* argPtr = args)
                {
                    DvcCellValue outVal = default;
                    var status = fn(userData, argPtr, (uint)args.Length, &outVal);
                    result = outVal;
                    return (DvcStatus)status;
                }
            };
            return (int)engine.UdfRegister(text, volatility, callbackBridge);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_udf_unregister")]
    public static int UdfUnregister(IntPtr enginePtr, byte* name, uint nameLen, int* found)
    {
        if (found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.UdfUnregister(text, out *found));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_tracking_enable")]
    public static int ChangeTrackingEnable(IntPtr enginePtr) => WithEngine(enginePtr, engine => (int)engine.ChangeTrackingEnable());

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_tracking_disable")]
    public static int ChangeTrackingDisable(IntPtr enginePtr) => WithEngine(enginePtr, engine => (int)engine.ChangeTrackingDisable());

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_tracking_is_enabled")]
    public static int ChangeTrackingIsEnabled(IntPtr enginePtr, int* outEnabled)
    {
        if (outEnabled == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine => (int)engine.ChangeTrackingIsEnabled(out *outEnabled));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_iterate")]
    public static int ChangeIterate(IntPtr enginePtr, IntPtr* outIter)
    {
        if (outIter == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.DrainChanges(out var iter);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            *outIter = HandleStore.Alloc(new ChangeIterHandle(iter));
            return (int)DvcStatus.Ok;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_iterator_next")]
    public static int ChangeIteratorNext(IntPtr iterPtr, [DNNE.C99Type("int32_t*")] DvcChangeType* outType, ulong* outEpoch, int* done)
    {
        if (done == null || outType == null || outEpoch == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!handle.Iterator.Next(out var item))
        {
            *done = 1;
            return (int)DvcStatus.Ok;
        }

        handle.Current = item;
        *outType = item.Type;
        *outEpoch = item.Epoch;
        *done = 0;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_cell")]
    public static int ChangeGetCell(IntPtr iterPtr, [DNNE.C99Type("DvcCellAddr*")] DvcCellAddr* outAddr)
    {
        if (outAddr == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *outAddr = handle.Current.Cell;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_name")]
    public static int ChangeGetName(IntPtr iterPtr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        return NativeHelpers.WriteUtf8(handle.Current.Name, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_chart_name")]
    public static int ChangeGetChartName(IntPtr iterPtr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        return NativeHelpers.WriteUtf8(handle.Current.Name, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_spill")]
    public static int ChangeGetSpill(IntPtr iterPtr, [DNNE.C99Type("DvcCellAddr*")] DvcCellAddr* anchor, [DNNE.C99Type("DvcCellRange*")] DvcCellRange* oldRange, int* hadOld, [DNNE.C99Type("DvcCellRange*")] DvcCellRange* newRange, int* hasNew)
    {
        if (anchor == null || oldRange == null || hadOld == null || newRange == null || hasNew == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *anchor = handle.Current.Cell;
        *oldRange = handle.Current.OldSpill;
        *hadOld = handle.Current.HadOldSpill;
        *newRange = handle.Current.NewSpill;
        *hasNew = handle.Current.HasNewSpill;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_format")]
    public static int ChangeGetFormat(IntPtr iterPtr, [DNNE.C99Type("DvcCellAddr*")] DvcCellAddr* addr, [DNNE.C99Type("DvcCellFormat*")] DvcCellFormat* oldFmt, [DNNE.C99Type("DvcCellFormat*")] DvcCellFormat* newFmt)
    {
        if (addr == null || oldFmt == null || newFmt == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *addr = handle.Current.Cell;
        *oldFmt = handle.Current.OldFormat;
        *newFmt = handle.Current.NewFormat;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_get_diagnostic")]
    public static int ChangeGetDiagnostic(IntPtr iterPtr, [DNNE.C99Type("int32_t*")] DvcDiagnosticCode* code, byte* buf, uint bufLen, uint* outLen)
    {
        if (code == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!HandleStore.TryGet<ChangeIterHandle>(iterPtr, out var handle) || handle is null)
        {
            return NativeHelpers.NullPtr();
        }

        *code = handle.Current.DiagnosticCode;
        return NativeHelpers.WriteUtf8(handle.Current.DiagnosticMessage, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_change_iterator_destroy")]
    public static int ChangeIteratorDestroy(IntPtr iterPtr)
    {
        HandleStore.Free(iterPtr);
        return (int)DvcStatus.Ok;
    }
}

