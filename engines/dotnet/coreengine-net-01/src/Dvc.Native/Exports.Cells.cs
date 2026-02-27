using System.Runtime.InteropServices;
using Dvc.Core;

namespace Dvc.Native;

public static unsafe partial class Exports
{
    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_number")]
    public static int CellSetNumber(IntPtr enginePtr, DvcCellAddr addr, double value)
    {
        return WithEngine(enginePtr, engine => (int)engine.SetCellNumber(addr, value));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_text")]
    public static int CellSetText(IntPtr enginePtr, DvcCellAddr addr, byte* text, uint textLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(text, textLen, out var value);
            return status == (int)DvcStatus.Ok ? (int)engine.SetCellText(addr, value) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_formula")]
    public static int CellSetFormula(IntPtr enginePtr, DvcCellAddr addr, byte* formula, uint formulaLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(formula, formulaLen, out var value);
            return status == (int)DvcStatus.Ok ? (int)engine.SetCellFormula(addr, value) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_clear")]
    public static int CellClear(IntPtr enginePtr, DvcCellAddr addr)
    {
        return WithEngine(enginePtr, engine => (int)engine.ClearCell(addr));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_state")]
    public static int CellGetState(IntPtr enginePtr, DvcCellAddr addr, DvcCellState* outState)
    {
        if (outState == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetCellState(addr, out var state);
            *outState = state;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_text")]
    public static int CellGetText(IntPtr enginePtr, DvcCellAddr addr, byte* buf, uint bufLen, uint* outLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetCellText(addr, out var text);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            return NativeHelpers.WriteUtf8(text, buf, bufLen, outLen);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_input_type")]
    public static int CellGetInputType(IntPtr enginePtr, DvcCellAddr addr, DvcInputType* outType)
    {
        if (outType == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetCellInputType(addr, out var inputType);
            *outType = inputType;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_input_text")]
    public static int CellGetInputText(IntPtr enginePtr, DvcCellAddr addr, byte* buf, uint bufLen, uint* outLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetCellInputText(addr, out var text);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            return NativeHelpers.WriteUtf8(text, buf, bufLen, outLen);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_number_a1")]
    public static int CellSetNumberA1(IntPtr enginePtr, byte* cellRef, uint refLen, double value) =>
        WithA1(enginePtr, cellRef, refLen, (engine, addr) => (int)engine.SetCellNumber(addr, value));

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_text_a1")]
    public static int CellSetTextA1(IntPtr enginePtr, byte* cellRef, uint refLen, byte* text, uint textLen)
    {
        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = NativeHelpers.ReadUtf8(text, textLen, out var value);
            return status == (int)DvcStatus.Ok ? (int)engine.SetCellText(addr, value) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_formula_a1")]
    public static int CellSetFormulaA1(IntPtr enginePtr, byte* cellRef, uint refLen, byte* formula, uint formulaLen)
    {
        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = NativeHelpers.ReadUtf8(formula, formulaLen, out var value);
            return status == (int)DvcStatus.Ok ? (int)engine.SetCellFormula(addr, value) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_clear_a1")]
    public static int CellClearA1(IntPtr enginePtr, byte* cellRef, uint refLen) =>
        WithA1(enginePtr, cellRef, refLen, (engine, addr) => (int)engine.ClearCell(addr));

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_state_a1")]
    public static int CellGetStateA1(IntPtr enginePtr, byte* cellRef, uint refLen, DvcCellState* outState)
    {
        if (outState == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = engine.GetCellState(addr, out var state);
            *outState = state;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_text_a1")]
    public static int CellGetTextA1(IntPtr enginePtr, byte* cellRef, uint refLen, byte* buf, uint bufLen, uint* outLen)
    {
        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = engine.GetCellText(addr, out var text);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            return NativeHelpers.WriteUtf8(text, buf, bufLen, outLen);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_input_type_a1")]
    public static int CellGetInputTypeA1(IntPtr enginePtr, byte* cellRef, uint refLen, DvcInputType* outType)
    {
        if (outType == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = engine.GetCellInputType(addr, out var inputType);
            *outType = inputType;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_input_text_a1")]
    public static int CellGetInputTextA1(IntPtr enginePtr, byte* cellRef, uint refLen, byte* buf, uint bufLen, uint* outLen)
    {
        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = engine.GetCellInputText(addr, out var text);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            return NativeHelpers.WriteUtf8(text, buf, bufLen, outLen);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_set_number")]
    public static int NameSetNumber(IntPtr enginePtr, byte* name, uint nameLen, double value)
    {
        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.SetNameNumber(text, value));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_set_text")]
    public static int NameSetText(IntPtr enginePtr, byte* name, uint nameLen, byte* text, uint textLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(name, nameLen, out var n);
            if (status != (int)DvcStatus.Ok)
            {
                return status;
            }

            status = NativeHelpers.ReadUtf8(text, textLen, out var t);
            return status == (int)DvcStatus.Ok ? (int)engine.SetNameText(n, t) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_set_formula")]
    public static int NameSetFormula(IntPtr enginePtr, byte* name, uint nameLen, byte* formula, uint formulaLen)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(name, nameLen, out var n);
            if (status != (int)DvcStatus.Ok)
            {
                return status;
            }

            status = NativeHelpers.ReadUtf8(formula, formulaLen, out var f);
            return status == (int)DvcStatus.Ok ? (int)engine.SetNameFormula(n, f) : status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_clear")]
    public static int NameClear(IntPtr enginePtr, byte* name, uint nameLen)
    {
        return WithEngineText(enginePtr, name, nameLen, (engine, text) => (int)engine.ClearName(text));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_get_input_type")]
    public static int NameGetInputType(IntPtr enginePtr, byte* name, uint nameLen, DvcInputType* outType)
    {
        if (outType == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var status = engine.GetNameInputType(text, out var inputType);
            *outType = inputType;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_name_get_input_text")]
    public static int NameGetInputText(IntPtr enginePtr, byte* name, uint nameLen, byte* buf, uint bufLen, uint* outLen)
    {
        return WithEngineText(enginePtr, name, nameLen, (engine, text) =>
        {
            var status = engine.GetNameInputText(text, out var inputText);
            if (status != DvcStatus.Ok)
            {
                return (int)status;
            }

            return NativeHelpers.WriteUtf8(inputText, buf, bufLen, outLen);
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_format")]
    public static int CellGetFormat(IntPtr enginePtr, DvcCellAddr addr, DvcCellFormat* outFormat)
    {
        if (outFormat == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetCellFormat(addr, out var format);
            *outFormat = format;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_format")]
    public static int CellSetFormat(IntPtr enginePtr, DvcCellAddr addr, DvcCellFormat* format)
    {
        if (format == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine => (int)engine.SetCellFormat(addr, *format));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_get_format_a1")]
    public static int CellGetFormatA1(IntPtr enginePtr, byte* cellRef, uint refLen, DvcCellFormat* outFormat)
    {
        if (outFormat == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithA1(enginePtr, cellRef, refLen, (engine, addr) =>
        {
            var status = engine.GetCellFormat(addr, out var format);
            *outFormat = format;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_set_format_a1")]
    public static int CellSetFormatA1(IntPtr enginePtr, byte* cellRef, uint refLen, DvcCellFormat* format)
    {
        if (format == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithA1(enginePtr, cellRef, refLen, (engine, addr) => (int)engine.SetCellFormat(addr, *format));
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_spill_role")]
    public static int CellSpillRole(IntPtr enginePtr, DvcCellAddr addr, DvcSpillRole* outRole)
    {
        if (outRole == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetSpillRole(addr, out var role);
            *outRole = role;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_spill_anchor")]
    public static int CellSpillAnchor(IntPtr enginePtr, DvcCellAddr addr, DvcCellAddr* outAnchor, int* found)
    {
        if (outAnchor == null || found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetSpillAnchor(addr, out var anchor, out var isFound);
            *outAnchor = anchor;
            *found = isFound;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_cell_spill_range")]
    public static int CellSpillRange(IntPtr enginePtr, DvcCellAddr addr, DvcCellRange* outRange, int* found)
    {
        if (outRange == null || found == null)
        {
            return NativeHelpers.NullPtr();
        }

        return WithEngine(enginePtr, engine =>
        {
            var status = engine.GetSpillRange(addr, out var range, out var isFound);
            *outRange = range;
            *found = isFound;
            return (int)status;
        });
    }

    [UnmanagedCallersOnly(EntryPoint = "dvc_insert_row")]
    public static int InsertRow(IntPtr enginePtr, ushort at) => WithEngine(enginePtr, engine => (int)engine.InsertRow(at));

    [UnmanagedCallersOnly(EntryPoint = "dvc_delete_row")]
    public static int DeleteRow(IntPtr enginePtr, ushort at) => WithEngine(enginePtr, engine => (int)engine.DeleteRow(at));

    [UnmanagedCallersOnly(EntryPoint = "dvc_insert_col")]
    public static int InsertCol(IntPtr enginePtr, ushort at) => WithEngine(enginePtr, engine => (int)engine.InsertCol(at));

    [UnmanagedCallersOnly(EntryPoint = "dvc_delete_col")]
    public static int DeleteCol(IntPtr enginePtr, ushort at) => WithEngine(enginePtr, engine => (int)engine.DeleteCol(at));

    private static int WithA1(IntPtr enginePtr, byte* refText, uint refLen, Func<DvcEngineCore, DvcCellAddr, int> action)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(refText, refLen, out var a1);
            if (status != (int)DvcStatus.Ok)
            {
                return status;
            }

            status = (int)engine.ParseCellRef(a1, out var addr);
            return status == (int)DvcStatus.Ok ? action(engine, addr) : status;
        });
    }

    private static int WithEngineText(IntPtr enginePtr, byte* text, uint len, Func<DvcEngineCore, string, int> action)
    {
        return WithEngine(enginePtr, engine =>
        {
            var status = NativeHelpers.ReadUtf8(text, len, out var value);
            return status == (int)DvcStatus.Ok ? action(engine, value) : status;
        });
    }

    private static int WithEngine(IntPtr enginePtr, Func<DvcEngineCore, int> action)
    {
        if (!NativeHelpers.TryGetEngine(enginePtr, out var engine, out var status))
        {
            return status;
        }

        return action(engine!);
    }
}
