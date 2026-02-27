using System.Text;
using Dvc.Core;
using Dvc.Native;

namespace Dvc.Interop.Tests;

public sealed unsafe class InteropExportTests
{
    [Fact]
    public void EngineCreate_NullOutPointer_ReturnsNullPointerError()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        var status = create(null);
        Assert.Equal((int)DvcStatus.ErrNullPointer, status);
    }

    [Fact]
    public void CellSetText_NullTextPointer_ReturnsNullPointerError()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, DvcCellAddr, byte*, uint, int> setText = &Exports.CellSetText;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;

        IntPtr engine = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));
        var status = setText(engine, new DvcCellAddr(1, 1), null, 4);
        Assert.Equal((int)DvcStatus.ErrNullPointer, status);
        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }

    [Fact]
    public void CellGetText_BufferQueryPattern_Works()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, DvcCellAddr, byte*, uint, int> setText = &Exports.CellSetText;
        delegate* unmanaged<IntPtr, DvcCellAddr, byte*, uint, uint*, int> getText = &Exports.CellGetText;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;

        IntPtr engine = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));
        var expected = "interop-check";
        var bytes = Encoding.UTF8.GetBytes(expected);
        fixed (byte* ptr = bytes)
        {
            Assert.Equal((int)DvcStatus.Ok, setText(engine, new DvcCellAddr(1, 1), ptr, (uint)bytes.Length));
        }

        uint len = 0;
        Assert.Equal((int)DvcStatus.Ok, getText(engine, new DvcCellAddr(1, 1), null, 0, &len));
        Assert.Equal((uint)bytes.Length, len);

        var buf = new byte[len];
        fixed (byte* bufPtr = buf)
        {
            Assert.Equal((int)DvcStatus.Ok, getText(engine, new DvcCellAddr(1, 1), bufPtr, len, &len));
        }

        Assert.Equal(expected, Encoding.UTF8.GetString(buf));
        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }

    [Fact]
    public void CellGetInputText_NullOutLen_ReturnsNullPointerError()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, DvcCellAddr, byte*, uint, uint*, int> getInputText = &Exports.CellGetInputText;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;

        IntPtr engine = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));
        var status = getInputText(engine, new DvcCellAddr(1, 1), null, 0, null);
        Assert.Equal((int)DvcStatus.ErrNullPointer, status);
        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }

    [Fact]
    public void NameInputText_Utf8BufferProtocol_Works()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, byte*, uint, byte*, uint, int> setNameText = &Exports.NameSetText;
        delegate* unmanaged<IntPtr, byte*, uint, byte*, uint, uint*, int> getNameInputText = &Exports.NameGetInputText;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;

        IntPtr engine = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));

        var name = Encoding.UTF8.GetBytes("alpha_name");
        var expected = "m\u00FCnchen";
        var text = Encoding.UTF8.GetBytes(expected);
        fixed (byte* namePtr = name)
        fixed (byte* textPtr = text)
        {
            Assert.Equal((int)DvcStatus.Ok, setNameText(engine, namePtr, (uint)name.Length, textPtr, (uint)text.Length));
        }

        uint len = 0;
        fixed (byte* namePtr = name)
        {
            Assert.Equal((int)DvcStatus.Ok, getNameInputText(engine, namePtr, (uint)name.Length, null, 0, &len));
            Assert.Equal((uint)text.Length, len);
            var buf = new byte[len];
            fixed (byte* bufPtr = buf)
            {
                Assert.Equal((int)DvcStatus.Ok, getNameInputText(engine, namePtr, (uint)name.Length, bufPtr, len, &len));
            }

            Assert.Equal(expected, Encoding.UTF8.GetString(buf));
        }

        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }

    [Fact]
    public void ChangeGetDiagnostic_ExposesCircularReferenceDiagnostic()
    {
        delegate* unmanaged<IntPtr*, int> create = &Exports.EngineCreate;
        delegate* unmanaged<IntPtr, int> destroy = &Exports.EngineDestroy;
        delegate* unmanaged<IntPtr, DvcRecalcMode, int> setRecalcMode = &Exports.EngineSetRecalcMode;
        delegate* unmanaged<IntPtr, int> enableTracking = &Exports.ChangeTrackingEnable;
        delegate* unmanaged<IntPtr, DvcCellAddr, byte*, uint, int> setFormula = &Exports.CellSetFormula;
        delegate* unmanaged<IntPtr, int> recalc = &Exports.Recalculate;
        delegate* unmanaged<IntPtr, IntPtr*, int> changeIterate = &Exports.ChangeIterate;
        delegate* unmanaged<IntPtr, DvcChangeType*, ulong*, int*, int> changeNext = &Exports.ChangeIteratorNext;
        delegate* unmanaged<IntPtr, DvcDiagnosticCode*, byte*, uint, uint*, int> getDiagnostic = &Exports.ChangeGetDiagnostic;
        delegate* unmanaged<IntPtr, int> changeDestroy = &Exports.ChangeIteratorDestroy;

        IntPtr engine = IntPtr.Zero;
        IntPtr iter = IntPtr.Zero;
        Assert.Equal((int)DvcStatus.Ok, create(&engine));
        Assert.Equal((int)DvcStatus.Ok, setRecalcMode(engine, DvcRecalcMode.Manual));
        Assert.Equal((int)DvcStatus.Ok, enableTracking(engine));

        var a1Formula = Encoding.UTF8.GetBytes("=B1");
        var b1Formula = Encoding.UTF8.GetBytes("=A1");
        fixed (byte* a1Ptr = a1Formula)
        fixed (byte* b1Ptr = b1Formula)
        {
            Assert.Equal((int)DvcStatus.Ok, setFormula(engine, new DvcCellAddr(1, 1), a1Ptr, (uint)a1Formula.Length));
            Assert.Equal((int)DvcStatus.Ok, setFormula(engine, new DvcCellAddr(2, 1), b1Ptr, (uint)b1Formula.Length));
        }

        Assert.Equal((int)DvcStatus.Ok, recalc(engine));
        Assert.Equal((int)DvcStatus.Ok, changeIterate(engine, &iter));

        var sawDiagnostic = false;
        while (true)
        {
            DvcChangeType type = default;
            ulong epoch = 0;
            var done = 0;
            Assert.Equal((int)DvcStatus.Ok, changeNext(iter, &type, &epoch, &done));
            if (done == 1)
            {
                break;
            }

            if (type != DvcChangeType.Diagnostic)
            {
                continue;
            }

            sawDiagnostic = true;
            Assert.True(epoch > 0);

            DvcDiagnosticCode code = default;
            uint len = 0;
            Assert.Equal((int)DvcStatus.Ok, getDiagnostic(iter, &code, null, 0, &len));
            var buf = new byte[len];
            fixed (byte* bufPtr = buf)
            {
                Assert.Equal((int)DvcStatus.Ok, getDiagnostic(iter, &code, bufPtr, len, &len));
            }

            Assert.Equal(DvcDiagnosticCode.CircularReferenceDetected, code);
            Assert.Equal("Circular reference detected.", Encoding.UTF8.GetString(buf));
            break;
        }

        Assert.True(sawDiagnostic);
        Assert.Equal((int)DvcStatus.Ok, changeDestroy(iter));
        Assert.Equal((int)DvcStatus.Ok, destroy(engine));
    }
}
