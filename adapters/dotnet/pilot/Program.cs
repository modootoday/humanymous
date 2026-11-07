using System.Net;
using System.Text.Json;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using Humanymous;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Hosting.Server;
using Microsoft.AspNetCore.Hosting.Server.Features;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

// Test host only. All HTTP clients, authority provisioning and assertions belong to the Rust runner.
internal static class Program
{
    private static readonly object OutputLock = new();
    private static long contacts, mutations;
    private static int collecting;
    private static Task? gcWorker;
    private static string handoffMode = "normal";
    private static void Emit(Action<Utf8JsonWriter> fields)
    {
        using var buffer = new MemoryStream();
        using (var writer = new Utf8JsonWriter(buffer))
        { writer.WriteStartObject(); fields(writer); writer.WriteEndObject(); }
        lock (OutputLock)
        {
            Console.OpenStandardOutput().Write(buffer.GetBuffer(), 0, checked((int)buffer.Length));
            Console.WriteLine(); Console.Out.Flush();
        }
    }
    internal static async Task<int> Main(string[] args)
    {
        if (args.Length != 0 || !Console.IsInputRedirected || !Console.IsOutputRedirected) return 64;
        Emit(json => { json.WriteString("kind", "starting"); json.WriteNumber("pid", Environment.ProcessId); });
        string? input = await Console.In.ReadLineAsync();
        if (input is null || input.Length > 65_536) return 65;
        using var startup = JsonDocument.Parse(input);
        if (startup.RootElement.TryGetProperty("generic_probe", out var genericProbe) && genericProbe.GetBoolean())
        {
            GenericAbi.Probe();
            Emit(json => { json.WriteString("kind", "generic-native-abi"); json.WriteNumber("passed", 1);
                json.WriteString("case", "pointer-free-roundtrip-and-negative-ownership"); });
        }
        using var host = new HumanymousHost(startup.RootElement.GetProperty("configuration").GetRawText());
        HumanymousMiddleware.BeforeHandoff = async context =>
        {
            switch (Volatile.Read(ref handoffMode))
            {
                case "rewrite": context.Request.Path = "/mutate"; break;
                case "response-started": await context.Response.StartAsync(); break;
                case "expire": await Task.Delay(host.TimeoutMilliseconds + 100); break;
            }
        };
        var builder = WebApplication.CreateSlimBuilder();
        builder.Logging.ClearProviders();
        builder.WebHost.ConfigureKestrel(server =>
        {
            server.Listen(IPAddress.Loopback, 0);
            server.Limits.MaxRequestBodySize = 2_097_152;
            server.Limits.MinRequestBodyDataRate = null;
        });
        await using var app = builder.Build();
        app.Lifetime.ApplicationStopping.Register(host.Dispose);
        app.UseMiddleware<HumanymousMiddleware>(host);
        app.Run(async context =>
        {
            long contact = Interlocked.Increment(ref contacts);
            using var body = new MemoryStream(); await context.Request.Body.CopyToAsync(body, context.RequestAborted);
            bool mutation = context.Request.Path == "/mutate";
            long changed = mutation ? Interlocked.Increment(ref mutations) : Interlocked.Read(ref mutations);
            Emit(json => { json.WriteString("kind", "application-contact"); json.WriteNumber("contacts", contact);
                json.WriteNumber("mutations", changed); json.WriteNumber("body_length", body.Length); });
            if (context.Request.Path == "/app-error") throw new IOException("pilot application failure");
            context.Response.Headers.Append("Set-Cookie", "a=1; Path=/");
            context.Response.Headers.Append("Set-Cookie", "b=2; Path=/");
            context.Response.Headers["X-Pilot-Forwarded"] = context.Request.Headers.ContainsKey("X-Forwarded-For") ? "present" : "absent";
            if (context.Request.Path == "/no-content") { context.Response.StatusCode = 204; return; }
            if (context.Request.Path == "/not-modified") { context.Response.StatusCode = 304; return; }
            if (HttpMethods.IsHead(context.Request.Method)) return;
            if (context.Request.Path == "/stream")
            {
                await context.Response.Body.WriteAsync(new byte[] { 1 }, context.RequestAborted);
                await context.Response.Body.FlushAsync(context.RequestAborted);
                await Task.Delay(20, context.RequestAborted);
                await context.Response.Body.WriteAsync(new byte[] { 2 }, context.RequestAborted); return;
            }
            context.Response.ContentType = "application/octet-stream";
            context.Response.ContentLength = body.Length;
            await context.Response.Body.WriteAsync(body.GetBuffer().AsMemory(0, checked((int)body.Length)), context.RequestAborted);
        });
        await app.StartAsync();
        string address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>()!.Addresses.Single();
        Emit(json => { json.WriteString("kind", "ready"); json.WriteNumber("pid", Environment.ProcessId);
            json.WriteString("address", address); json.WriteString("runtime", System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription);
            json.WriteBoolean("dynamic_code", System.Runtime.CompilerServices.RuntimeFeature.IsDynamicCodeSupported); });

        var control = Task.Run(async () =>
        {
            while (await Console.In.ReadLineAsync() is { } line)
            {
                if (line.Length > 1024) { app.Lifetime.StopApplication(); break; }
                using var command = JsonDocument.Parse(line);
                switch (command.RootElement.GetProperty("operation").GetString())
                {
                    case "gc-start":
                        if (Interlocked.Exchange(ref collecting, 1) == 0)
                            gcWorker = Task.Run(async () =>
                            {
                                while (Volatile.Read(ref collecting) == 1)
                                {
                                    var pressure = new byte[131_072]; GC.Collect(); GC.WaitForPendingFinalizers(); GC.KeepAlive(pressure); await Task.Delay(5);
                                }
                            });
                        break;
                    case "gc-stop":
                        Interlocked.Exchange(ref collecting, 0);
                        if (gcWorker is { } finishedGc) await finishedGc.WaitAsync(TimeSpan.FromSeconds(2));
                        gcWorker = null; break;
                    case "dispose-host": host.Dispose(); break;
                    case "handoff-mode":
                        var mode = command.RootElement.GetProperty("mode").GetString();
                        if (mode is not ("normal" or "rewrite" or "response-started" or "expire")) throw new InvalidDataException();
                        Volatile.Write(ref handoffMode, mode); break;
                    case "stop": Interlocked.Exchange(ref collecting, 0); app.Lifetime.StopApplication(); return;
                    case "counters":
                        Emit(json => { json.WriteString("kind", "counters"); json.WriteNumber("contacts", Interlocked.Read(ref contacts));
                            json.WriteNumber("mutations", Interlocked.Read(ref mutations)); json.WriteNumber("collections", GC.CollectionCount(2));
                            try { json.WriteNumber("native_resources", host.Metrics()); } catch (ObjectDisposedException) { json.WriteNull("native_resources"); } });
                        break;
                    default: app.Lifetime.StopApplication(); return;
                }
            }
            app.Lifetime.StopApplication();
        });
        await app.WaitForShutdownAsync();
        Interlocked.Exchange(ref collecting, 0);
        if (gcWorker is { } remainingGc) await remainingGc.WaitAsync(TimeSpan.FromSeconds(2));
        try { await control.WaitAsync(TimeSpan.FromSeconds(2)); }
        catch (TimeoutException) { return 70; }
        Emit(json => { json.WriteString("kind", "stopped"); json.WriteNumber("contacts", Interlocked.Read(ref contacts));
            json.WriteNumber("mutations", Interlocked.Read(ref mutations)); json.WriteNumber("collections", GC.CollectionCount(2)); });
        return 0;
    }
}

// Separate optional actual entrypoint check, not middleware or C++ qualification.
internal static partial class GenericAbi
{
    [LibraryImport("humanymous_native_abi_export", EntryPoint = "humanymous_native_abi_v1")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    private static partial Native.Response Invoke(Native.Command command);
    private static Native.Response Ok(Native.Command command)
    {
        var result = Invoke(command);
        if (result.Status != 0 || result.Reserved != 0) throw new InvalidDataException("Generic ABI failure");
        return result;
    }
    internal static void Probe()
    {
        Native.ValidateAbi();
        var context = Ok(Native.New(1)).Value;
        try
        {
            var buffer = Ok(Native.New(3, context, length: 64)).Value;
            var write = Native.New(6, context, buffer, 3); write.Offset = 31;
            Native.SetData(ref write, new byte[] { 0, 255, 42 }); Ok(write);
            GC.Collect(); GC.WaitForPendingFinalizers();
            var read = Native.New(7, context, buffer, 5); read.Offset = 30;
            Span<byte> data = stackalloc byte[5]; Native.ReadData(Ok(read), data);
            if (!data.SequenceEqual(new byte[] { 0, 0, 255, 42, 0 })) throw new InvalidDataException("Generic ABI copy mismatch");
            if (Ok(Native.New(5, context, buffer)).Value != 64) throw new InvalidDataException("Generic ABI length mismatch");
            Ok(Native.New(4, context, buffer));
            var stale = Invoke(Native.New(5, context, buffer));
            if (stale.Status != 3 || stale.Value != 0 || stale.Data0 != 0 || stale.Data1 != 0 || stale.Data2 != 0 || stale.Data3 != 0)
                throw new InvalidDataException("Generic ABI stale handle accepted");
            var version = Native.New(1); version.Major = 2;
            if (Invoke(version).Status != 1) throw new InvalidDataException("Generic ABI version accepted");
        }
        finally { Ok(Native.New(2, context)); }
    }
}
