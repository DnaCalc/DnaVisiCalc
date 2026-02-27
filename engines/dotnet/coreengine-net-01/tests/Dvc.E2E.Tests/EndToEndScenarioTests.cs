using System.Text;
using Dvc.Core;
using Dvc.Native;

namespace Dvc.E2E.Tests;

public sealed unsafe class EndToEndScenarioTests
{
    [Fact]
    public void FormulaStructuralEpochScenario_WorksAcrossExportBoundary()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, byte*, uint, double, int> setNumberA1 = &Exports.CellSetNumberA1;
        delegate* unmanaged<IntPtr, byte*, uint, byte*, uint, int> setFormulaA1 = &Exports.CellSetFormulaA1;
        delegate* unmanaged<IntPtr, byte*, uint, DvcCellState*, int> getStateA1 = &Exports.CellGetStateA1;
        delegate* unmanaged<IntPtr, ushort, int> insertRow = &Exports.InsertRow;
        delegate* unmanaged<IntPtr, byte*, uint, byte*, uint, uint*, int> getInputTextA1 = &Exports.CellGetInputTextA1;
        delegate* unmanaged<IntPtr, ulong*, int> committedEpoch = &Exports.EngineCommittedEpoch;
        delegate* unmanaged<IntPtr, ulong*, int> stabilizedEpoch = &Exports.EngineStabilizedEpoch;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;

        IntPtr engine = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));

        var a1 = Encoding.UTF8.GetBytes("A1");
        fixed (byte* a1Ptr = a1)
        {
            Assert.Equal((int)DvcStatus.Ok, setNumberA1(engine, a1Ptr, (uint)a1.Length, 2));
        }

        var b1 = Encoding.UTF8.GetBytes("B1");
        var formula = Encoding.UTF8.GetBytes("=A1+1");
        fixed (byte* b1Ptr = b1)
        fixed (byte* fPtr = formula)
        {
            Assert.Equal((int)DvcStatus.Ok, setFormulaA1(engine, b1Ptr, (uint)b1.Length, fPtr, (uint)formula.Length));
        }

        DvcCellState state = default;
        fixed (byte* b1Ptr = b1)
        {
            Assert.Equal((int)DvcStatus.Ok, getStateA1(engine, b1Ptr, (uint)b1.Length, &state));
        }

        Assert.Equal(DvcValueType.Number, state.Value.Type);
        Assert.Equal(3, state.Value.Number);

        Assert.Equal((int)DvcStatus.Ok, insertRow(engine, 1));

        var b2 = Encoding.UTF8.GetBytes("B2");
        uint required = 0;
        fixed (byte* b2Ptr = b2)
        {
            Assert.Equal((int)DvcStatus.Ok, getInputTextA1(engine, b2Ptr, (uint)b2.Length, null, 0, &required));
            var buf = new byte[required];
            fixed (byte* bufPtr = buf)
            {
                Assert.Equal((int)DvcStatus.Ok, getInputTextA1(engine, b2Ptr, (uint)b2.Length, bufPtr, required, &required));
                Assert.Equal("=A2+1", Encoding.UTF8.GetString(buf));
            }
        }

        ulong committed = 0;
        ulong stabilized = 0;
        Assert.Equal((int)DvcStatus.Ok, committedEpoch(engine, &committed));
        Assert.Equal((int)DvcStatus.Ok, stabilizedEpoch(engine, &stabilized));
        Assert.True(committed >= 3);
        Assert.Equal(committed, stabilized);

        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }
}
