using System.Buffers;
using System.Text.Json;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.Http.Features;

namespace Humanymous;

// Install first, before Body/BodyReader consumers, response writers and protected effects.
public sealed class HumanymousMiddleware(RequestDelegate next, HumanymousHost host)
{
    // Friend pilot only; never populated by the production package.
    internal static Func<HttpContext, ValueTask>? BeforeHandoff = null;
    public async Task InvokeAsync(HttpContext context)
    {
        var original = context.Request.Body;
        MemoryStream? replay = null;
        bool authorized = false;
        try
        {
            if (context.Response.HasStarted || context.Request.Method == "CONNECT" || IsUpgrade(context)
                || (original.CanSeek && original.Position != 0)) throw new NativeFailureException();
            foreach (var name in context.Request.Headers.Keys.ToArray())
                if (host.IsInternalHeader(name)) context.Request.Headers.Remove(name);

            using (var request = host.Begin(context.Request.ContentLength))
            using (var deadline = CancellationTokenSource.CreateLinkedTokenSource(context.RequestAborted))
            {
                deadline.CancelAfter(host.TimeoutMilliseconds);
                using var registration = deadline.Token.Register(static state => ((NativeRequest)state!).Cancel(), request);
                byte[] input = ArrayPool<byte>.Shared.Rent(16_384);
                try
                {
                    while (true)
                    {
                        int length = await original.ReadAsync(input.AsMemory(), deadline.Token);
                        if (length == 0) break;
                        request.Append(input.AsSpan(0, length));
                    }
                }
                finally { Array.Clear(input); ArrayPool<byte>.Shared.Return(input); }
                deadline.Token.ThrowIfCancellationRequested(); request.Eof();
                var verdict = await request.DecideAsync(Observe(context));
                deadline.Token.ThrowIfCancellationRequested();
                if (verdict != 1)
                {
                    context.Response.StatusCode = verdict == 2 ? 403 : 503;
                    return;
                }
                replay = new MemoryStream(request.Replay(), writable: false);
                if (BeforeHandoff is { } probe) await probe(context);
                // The final native check includes original deadline and authority expiry,
                // after the entire replay copy, with no subsequent await before next.
                request.Handoff(Observe(context), deadline.IsCancellationRequested, context.Response.HasStarted, IsUpgrade(context));
            }
            context.Request.Body = replay;
            authorized = true;
        }
        catch (Exception error) when (error is NativeFailureException or OperationCanceledException or IOException or ObjectDisposedException)
        {
            if (context.Response.HasStarted || context.RequestAborted.IsCancellationRequested) context.Abort();
            else context.Response.StatusCode = 503;
        }
        finally
        {
            if (!authorized) { context.Request.Body = original; replay?.Dispose(); }
        }
        if (!authorized) return;
        // Application exceptions retain the host application's normal handling.
        try { await next(context); }
        finally { context.Request.Body = original; replay?.Dispose(); }
    }
    private static bool IsUpgrade(HttpContext context) =>
        context.Request.Headers.ContainsKey("Upgrade") || context.Features.Get<IHttpUpgradeFeature>()?.IsUpgradableRequest == true;
    private static byte[] Observe(HttpContext context)
    {
        using var output = new MemoryStream();
        using (var json = new Utf8JsonWriter(output))
        {
            json.WriteStartObject(); json.WriteString("method", context.Request.Method);
            json.WriteString("target", context.Features.Get<IHttpRequestFeature>()?.RawTarget ?? "");
            json.WriteString("authority", context.Request.Host.Value);
            json.WriteStartArray("route"); json.WriteStringValue(context.Request.PathBase.Value ?? "");
            json.WriteStringValue(context.Request.Path.Value ?? ""); json.WriteStringValue(context.Request.QueryString.Value ?? ""); json.WriteEndArray();
            json.WriteStartArray("headers");
            foreach (var header in context.Request.Headers)
                foreach (var value in header.Value)
                {
                    json.WriteStartArray(); json.WriteStringValue(header.Key); json.WriteStringValue(value ?? ""); json.WriteEndArray();
                }
            json.WriteEndArray(); json.WriteEndObject();
        }
        return output.ToArray();
    }
}
