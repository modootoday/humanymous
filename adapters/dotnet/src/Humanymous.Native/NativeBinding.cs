using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

[assembly: DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory | DllImportSearchPath.SafeDirectories)]
[assembly: InternalsVisibleTo("Humanymous.HostPilot")]

namespace Humanymous;

// The only managed ABI declarations. No pointer, delegate or managed buffer is retained natively.
internal static partial class Native
{
    [StructLayout(LayoutKind.Sequential)]
    internal struct Command
    {
        internal uint Major, Minor, Size, Reserved, Operation, Flags;
        internal ulong Context, Buffer, Offset, Length, Data0, Data1, Data2, Data3;
    }
    [StructLayout(LayoutKind.Sequential)]
    internal struct Response
    {
        internal uint Status, Reserved;
        internal ulong Value, Data0, Data1, Data2, Data3;
    }
    [LibraryImport("humanymous_dotnet_adapter_export", EntryPoint = "humanymous_dotnet_v1")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    private static partial Response Invoke(Command command);

    internal static Command New(uint operation, ulong context = 0, ulong buffer = 0, ulong length = 0) =>
        new() { Major = 1, Size = 16, Operation = operation, Context = context, Buffer = buffer, Length = length };
    internal static Response Call(Command command)
    {
        if ((!OperatingSystem.IsWindows() && !OperatingSystem.IsLinux()) || RuntimeInformation.ProcessArchitecture != Architecture.X64
            || !BitConverter.IsLittleEndian || IntPtr.Size != 8 || Marshal.SizeOf<Command>() != 88 || Marshal.SizeOf<Response>() != 48)
            throw new PlatformNotSupportedException("Unqualified native ABI layout");
        var response = Invoke(command);
        if (response.Reserved != 0 || response.Status != 0) throw new NativeFailureException();
        return response;
    }
    internal static void SetData(ref Command command, ReadOnlySpan<byte> bytes)
    {
        Span<byte> data = stackalloc byte[32];
        data.Clear(); bytes.CopyTo(data);
        command.Data0 = BinaryPrimitives.ReadUInt64LittleEndian(data);
        command.Data1 = BinaryPrimitives.ReadUInt64LittleEndian(data[8..]);
        command.Data2 = BinaryPrimitives.ReadUInt64LittleEndian(data[16..]);
        command.Data3 = BinaryPrimitives.ReadUInt64LittleEndian(data[24..]);
    }
    internal static void ReadData(Response response, Span<byte> destination)
    {
        Span<byte> data = stackalloc byte[32];
        BinaryPrimitives.WriteUInt64LittleEndian(data, response.Data0);
        BinaryPrimitives.WriteUInt64LittleEndian(data[8..], response.Data1);
        BinaryPrimitives.WriteUInt64LittleEndian(data[16..], response.Data2);
        BinaryPrimitives.WriteUInt64LittleEndian(data[24..], response.Data3);
        data[..destination.Length].CopyTo(destination);
    }
}

public sealed class NativeFailureException : Exception
{
    public NativeFailureException() : base("Native adapter unavailable") { }
}

// The native integer is an opaque registry context, never an address.
internal sealed class ContextHandle : SafeHandle
{
    internal ContextHandle(ulong value) : base(IntPtr.Zero, true) => SetHandle(unchecked((nint)value));
    public override bool IsInvalid => handle == IntPtr.Zero;
    protected override bool ReleaseHandle()
    {
        try { Native.Call(Native.New(2, unchecked((ulong)handle))); return true; }
        catch { return false; }
    }
    internal Lease Acquire() => new(this);
    internal sealed class Lease : IDisposable
    {
        private ContextHandle? owner;
        internal ulong Id { get; }
        internal Lease(ContextHandle handle)
        {
            bool added = false;
            handle.DangerousAddRef(ref added);
            if (!added) throw new ObjectDisposedException(nameof(ContextHandle));
            owner = handle; Id = unchecked((ulong)handle.DangerousGetHandle());
        }
        public void Dispose() => Interlocked.Exchange(ref owner, null)?.DangerousRelease();
    }
}

public sealed class HumanymousHost : IDisposable
{
    private readonly ContextHandle context;
    public int TimeoutMilliseconds { get; }
    public HumanymousHost(string supervisorConfiguration)
    {
        context = new ContextHandle(Native.Call(Native.New(1)).Value);
        try
        {
            using var lease = context.Acquire();
            var buffer = Upload(lease.Id, Encoding.UTF8.GetBytes(supervisorConfiguration));
            try { Native.Call(Native.New(100, lease.Id, buffer)); }
            finally { Native.Call(Native.New(4, lease.Id, buffer)); }
            // Native Configuration::parse already validated the complete strict schema.
            using var json = JsonDocument.Parse(supervisorConfiguration);
            TimeoutMilliseconds = json.RootElement.GetProperty("timeoutMs").GetInt32();
        }
        catch { context.Dispose(); throw; }
    }
    internal NativeRequest Begin(long? length)
    {
        using var lease = context.Acquire();
        var id = Native.Call(Native.New(101, lease.Id, length: length.HasValue ? checked((ulong)length.Value) : ulong.MaxValue)).Value;
        return new NativeRequest(id);
    }
    internal bool IsInternalHeader(string name)
    {
        using var lease = context.Acquire();
        var buffer = Upload(lease.Id, Encoding.UTF8.GetBytes(name));
        try { return Native.Call(Native.New(107, lease.Id, buffer)).Value == 1; }
        finally { Native.Call(Native.New(4, lease.Id, buffer)); }
    }
    internal ulong Metrics() { using var lease = context.Acquire(); return Native.Call(Native.New(109, lease.Id)).Value; }
    internal static ulong Upload(ulong context, ReadOnlySpan<byte> bytes)
    {
        var id = Native.Call(Native.New(3, context, length: checked((ulong)bytes.Length))).Value;
        try
        {
            for (int offset = 0; offset < bytes.Length; offset += 32)
            {
                int count = Math.Min(32, bytes.Length - offset);
                var command = Native.New(6, context, id, (ulong)count);
                command.Offset = (ulong)offset;
                Native.SetData(ref command, bytes.Slice(offset, count)); Native.Call(command);
            }
            return id;
        }
        catch { Native.Call(Native.New(4, context, id)); throw; }
    }
    public void Dispose()
    {
        // Closing the installation cancels child requests before their contexts are released.
        context.Dispose();
    }
}

internal sealed class NativeRequest(ulong id) : IDisposable
{
    private readonly ContextHandle context = new(id);
    internal void Append(ReadOnlySpan<byte> bytes)
    {
        using var lease = context.Acquire();
        for (int offset = 0; offset < bytes.Length; offset += 32)
        {
            int count = Math.Min(32, bytes.Length - offset);
            var command = Native.New(102, lease.Id, length: (ulong)count);
            Native.SetData(ref command, bytes.Slice(offset, count)); Native.Call(command);
        }
    }
    internal void Eof() { using var lease = context.Acquire(); Native.Call(Native.New(103, lease.Id)); }
    internal void Cancel()
    {
        try { using var lease = context.Acquire(); Native.Call(Native.New(105, lease.Id)); }
        catch (ObjectDisposedException) { }
        catch (NativeFailureException) { }
    }
    internal async Task<ulong> DecideAsync(byte[] observation)
    {
        // Acquire before scheduling: Dispose/finalization cannot release a pending call.
        using var lease = context.Acquire();
        var buffer = HumanymousHost.Upload(lease.Id, observation);
        try { return await Task.Run(() => Native.Call(Native.New(104, lease.Id, buffer)).Value).ConfigureAwait(false); }
        finally
        {
            try { Native.Call(Native.New(4, lease.Id, buffer)); }
            catch (NativeFailureException) { /* Request failure may already have revoked its context. */ }
        }
    }
    internal byte[] Replay()
    {
        using var lease = context.Acquire();
        var buffer = Native.Call(Native.New(108, lease.Id)).Value;
        try
        {
            int length = checked((int)Native.Call(Native.New(5, lease.Id, buffer)).Value);
            if (length > 1_048_576) throw new NativeFailureException();
            byte[] body = new byte[length];
            for (int offset = 0; offset < length; offset += 32)
            {
                int count = Math.Min(32, length - offset);
                var command = Native.New(7, lease.Id, buffer, (ulong)count); command.Offset = (ulong)offset;
                Native.ReadData(Native.Call(command), body.AsSpan(offset, count));
            }
            return body;
        }
        finally { Native.Call(Native.New(4, lease.Id, buffer)); }
    }
    internal void Handoff(byte[] observation, bool aborted, bool responseStarted, bool upgrade)
    {
        using var lease = context.Acquire();
        var buffer = HumanymousHost.Upload(lease.Id, observation);
        try { Native.Call(Native.New(106, lease.Id, buffer, (aborted ? 1UL : 0) | (responseStarted ? 2UL : 0) | (upgrade ? 4UL : 0))); }
        finally { Native.Call(Native.New(4, lease.Id, buffer)); }
    }
    public void Dispose() { Cancel(); context.Dispose(); }
}
