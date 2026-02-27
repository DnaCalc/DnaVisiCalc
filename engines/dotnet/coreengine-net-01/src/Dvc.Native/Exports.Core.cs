using System.Runtime.InteropServices;
using Dvc.Core;

namespace Dvc.Native;

public static unsafe partial class Exports
{
    [UnmanagedCallersOnly(EntryPoint = "dvc_api_version")]
    public static uint ApiVersion() => DvcApiVersion.Packed;

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_create")]
    public static int EngineCreate(IntPtr* outEngine)
    {
        if (outEngine == null)
        {
            return (int)DvcStatus.ErrNullPointer;
        }

        try
        {
            var engine = new DvcEngineCore();
            *outEngine = HandleStore.Alloc(new EngineHandle(engine));
            return (int)DvcStatus.Ok;
        }
        catch (OutOfMemoryException)
        {
            return (int)DvcStatus.ErrOutOfMemory;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_create_with_bounds")]
    public static int EngineCreateWithBounds(DvcSheetBounds bounds, IntPtr* outEngine)
    {
        if (outEngine == null)
        {
            return (int)DvcStatus.ErrNullPointer;
        }

        if (bounds.MaxColumns == 0 || bounds.MaxRows == 0)
        {
            return (int)DvcStatus.ErrInvalidArgument;
        }

        try
        {
            var engine = new DvcEngineCore(bounds);
            *outEngine = HandleStore.Alloc(new EngineHandle(engine));
            return (int)DvcStatus.Ok;
        }
        catch (OutOfMemoryException)
        {
            return (int)DvcStatus.ErrOutOfMemory;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_destroy")]
    public static int EngineDestroy(IntPtr enginePtr)
    {
        HandleStore.Free(enginePtr);
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_clear")]
    public static int EngineClear(IntPtr enginePtr)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.ClearEngine();
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_bounds")]
    public static int EngineBounds(IntPtr enginePtr, DvcSheetBounds* outBounds)
    {
        if (outBounds == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        *outBounds = engine!.Bounds;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_get_recalc_mode")]
    public static int EngineGetRecalcMode(IntPtr enginePtr, DvcRecalcMode* outMode)
    {
        if (outMode == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        var s = engine!.GetRecalcMode(out var mode);
        *outMode = mode;
        return (int)s;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_set_recalc_mode")]
    public static int EngineSetRecalcMode(IntPtr enginePtr, DvcRecalcMode mode)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.SetRecalcMode(mode);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_committed_epoch")]
    public static int EngineCommittedEpoch(IntPtr enginePtr, ulong* outEpoch)
    {
        if (outEpoch == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        *outEpoch = engine!.CommittedEpoch;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_stabilized_epoch")]
    public static int EngineStabilizedEpoch(IntPtr enginePtr, ulong* outEpoch)
    {
        if (outEpoch == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        *outEpoch = engine!.StabilizedEpoch;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_is_stable")]
    public static int EngineIsStable(IntPtr enginePtr, int* outStable)
    {
        if (outStable == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.IsStable(out *outStable);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_recalculate")]
    public static int Recalculate(IntPtr enginePtr)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.Recalculate();
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_has_volatile_cells")]
    public static int HasVolatile(IntPtr enginePtr, int* outValue)
    {
        if (outValue == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.HasVolatileCells(out *outValue);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_has_externally_invalidated_cells")]
    public static int HasExternal(IntPtr enginePtr, int* outValue)
    {
        if (outValue == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.HasExternallyInvalidatedCells(out *outValue);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_invalidate_volatile")]
    public static int InvalidateVolatile(IntPtr enginePtr)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.InvalidateVolatile();
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_has_stream_cells")]
    public static int HasStreamCells(IntPtr enginePtr, int* outValue)
    {
        if (outValue == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.HasStreamCells(out *outValue);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_tick_streams")]
    public static int TickStreams(IntPtr enginePtr, double elapsedSecs, int* anyAdvanced)
    {
        if (anyAdvanced == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.TickStreams(elapsedSecs, out *anyAdvanced);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_invalidate_udf")]
    public static int InvalidateUdf(IntPtr enginePtr, byte* name, uint nameLen)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        status = NativeHelpers.ReadUtf8(name, nameLen, out var value);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        return (int)engine!.InvalidateUdf(value);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_get_iteration_config")]
    public static int GetIterationConfig(IntPtr enginePtr, DvcIterationConfig* outCfg)
    {
        if (outCfg == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.GetIterationConfig(out *outCfg);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_engine_set_iteration_config")]
    public static int SetIterationConfig(IntPtr enginePtr, DvcIterationConfig* cfg)
    {
        if (cfg == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return (int)engine!.SetIterationConfig(*cfg);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_last_error_message")]
    public static int LastErrorMessage(IntPtr enginePtr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        engine!.LastErrorMessage(out var message);
        return NativeHelpers.WriteUtf8(message, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_last_error_kind")]
    public static int LastErrorKind(IntPtr enginePtr, DvcStatus* outStatus)
    {
        if (outStatus == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        engine!.LastErrorKind(out var value);
        *outStatus = value;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_last_reject_kind")]
    public static int LastRejectKind(IntPtr enginePtr, DvcRejectKind* outKind)
    {
        if (outKind == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        engine!.LastRejectKind(out var value);
        *outKind = value;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_last_reject_context")]
    public static int LastRejectContext(IntPtr enginePtr, DvcLastRejectContext* outContext)
    {
        if (outContext == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        engine!.LastRejectContext(out var value);
        *outContext = value;
        return (int)DvcStatus.Ok;
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_error_message")]
    public static int CellErrorMessage(IntPtr enginePtr, DvcCellAddr addr, byte* buf, uint bufLen, uint* outLen)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        status = (int)engine!.CellErrorMessage(addr, out var message);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        return NativeHelpers.WriteUtf8(message, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_palette_color_name")]
    public static int PaletteColorName(DvcPaletteColor color, byte* buf, uint bufLen, uint* outLen)
    {
        if (!DvcEngineCore.TryPaletteColorName(color, out var name))
        {
            if (outLen != null)
            {
                *outLen = 0;
            }

            return (int)DvcStatus.ErrInvalidArgument;
        }

        return NativeHelpers.WriteUtf8(name, buf, bufLen, outLen);
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_parse_cell_ref")]
    public static int ParseCellRef(IntPtr enginePtr, byte* text, uint textLen, DvcCellAddr* outAddr)
    {
        if (outAddr == null)
        {
            return NativeHelpers.NullPtr();
        }

        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        status = NativeHelpers.ReadUtf8(text, textLen, out var a1);
        if (status != (int)DvcStatus.Ok)
        {
            return status;
        }

        status = (int)engine!.ParseCellRef(a1, out var addr);
        if (status == (int)DvcStatus.Ok)
        {
            *outAddr = addr;
        }

        return status;
    }
}
