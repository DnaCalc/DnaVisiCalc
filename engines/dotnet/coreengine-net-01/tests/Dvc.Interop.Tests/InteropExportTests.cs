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
}
