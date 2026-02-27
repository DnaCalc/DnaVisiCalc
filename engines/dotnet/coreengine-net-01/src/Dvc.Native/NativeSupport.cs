using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using Dvc.Core;

namespace Dvc.Native;

internal sealed class EngineHandle(DvcEngineCore engine)
{
    public DvcEngineCore Engine { get; } = engine;
}

internal sealed class CellIterHandle(CellIterator iterator)
{
    public CellIterator Iterator { get; } = iterator;
    public CellIterEntry Current { get; set; }
}

internal sealed class NameIterHandle(NameIterator iterator)
{
    public NameIterator Iterator { get; } = iterator;
    public NameIterEntry Current { get; set; }
}

internal sealed class FormatIterHandle(FormatIterator iterator)
{
    public FormatIterator Iterator { get; } = iterator;
}

internal sealed class ControlIterHandle(ControlIterator iterator)
{
    public ControlIterator Iterator { get; } = iterator;
}

internal sealed class ChartIterHandle(ChartIterator iterator)
{
    public ChartIterator Iterator { get; } = iterator;
}

internal sealed class ChangeIterHandle(ChangeIterator iterator)
{
    public ChangeIterator Iterator { get; } = iterator;
    public ChangeItem Current { get; set; }
}

internal sealed class ChartOutputHandle(ChartOutput output)
{
    public ChartOutput Output { get; } = output;
}

internal static class HandleStore
{
    public static IntPtr Alloc(object value)
    {
        var handle = GCHandle.Alloc(value);
        return GCHandle.ToIntPtr(handle);
    }

    public static bool TryGet<T>(IntPtr ptr, out T? value) where T : class
    {
        value = null;
        if (ptr == IntPtr.Zero)
        {
            return false;
        }

        try
        {
            var handle = GCHandle.FromIntPtr(ptr);
            value = handle.Target as T;
            return value is not null;
        }
        catch
        {
            return false;
        }
    }

    public static void Free(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
        {
            return;
        }

        try
        {
            var handle = GCHandle.FromIntPtr(ptr);
            if (handle.IsAllocated)
            {
                handle.Free();
            }
        }
        catch
        {
            // ignore invalid pointer; caller gets safe no-op cleanup.
        }
    }
}

internal static unsafe class NativeHelpers
{
    public static bool TryGetEngine(IntPtr ptr, out DvcEngineCore? engine, out int status)
    {
        engine = null;
        if (!HandleStore.TryGet<EngineHandle>(ptr, out var wrapper) || wrapper is null)
        {
            status = (int)DvcStatus.ErrNullPointer;
            return false;
        }

        engine = wrapper.Engine;
        status = (int)DvcStatus.Ok;
        return true;
    }

    public static int ReadUtf8(byte* ptr, uint len, out string text)
    {
        text = string.Empty;
        if (len == 0)
        {
            if (ptr == null)
            {
                return (int)DvcStatus.ErrNullPointer;
            }

            return (int)DvcStatus.Ok;
        }

        if (ptr == null)
        {
            return (int)DvcStatus.ErrNullPointer;
        }

        try
        {
            text = Encoding.UTF8.GetString(ptr, checked((int)len));
            return (int)DvcStatus.Ok;
        }
        catch
        {
            return (int)DvcStatus.ErrInvalidArgument;
        }
    }

    public static int WriteUtf8(string value, byte* buf, uint bufLen, uint* outLen)
    {
        if (outLen == null)
        {
            return (int)DvcStatus.ErrNullPointer;
        }

        var bytes = Encoding.UTF8.GetBytes(value);
        *outLen = (uint)bytes.Length;
        if (buf == null || bufLen == 0)
        {
            return (int)DvcStatus.Ok;
        }

        var count = Math.Min(bytes.Length, (int)bufLen);
        Marshal.Copy(bytes, 0, (IntPtr)buf, count);
        return (int)DvcStatus.Ok;
    }

    public static int SetOut<T>(T* ptr, T value) where T : unmanaged
    {
        if (ptr == null)
        {
            return (int)DvcStatus.ErrNullPointer;
        }

        *ptr = value;
        return (int)DvcStatus.Ok;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public static int NullPtr() => (int)DvcStatus.ErrNullPointer;
}
