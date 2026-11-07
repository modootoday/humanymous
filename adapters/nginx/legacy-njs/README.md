# nginx adapter compatibility pilot

Experimental Linux nginx+njs bounded-request adapter, **mock authority only**.
This is not full humanymous protection or a shared adapter/0.1 implementation.
No Rust authority integration, collector/challenge recovery or capture claims.
Production deployment is blocked; do not deploy the fixture token/root worker.

Run with Docker Compose from this repository:

```
docker compose -p humanymous-nginx-pilot -f adapters/nginx/compose.yaml up -d
docker compose -p humanymous-nginx-pilot -f adapters/nginx/compose.yaml exec -T nginx njs -m /pilot/test/unit.js
docker compose -p humanymous-nginx-pilot -f adapters/nginx/compose.yaml exec -T fixture python /pilot/test/pilot.py test
docker compose -p humanymous-nginx-pilot -f adapters/nginx/compose.yaml down -v
```

The network is internal, has no published port and targets only the fixture.
Only GET/POST on the listed pilot routes, max 64 KiB complete entity body,
fixed example.test authority and an explicit five-header projection are supported.
All other application headers are dropped; never use this on arbitrary apps
without an agreed header/route inventory. Upgrades, gRPC, trailers, chunked
uploads, Content-Encoding uploads and streaming uploads are rejected. Normal response SSE is tested;
long-running cancellation, backpressure and native OS matrices remain open.
Set-Cookie and repeatable response headers are handled directly by nginx proxy.
No response HTML/CSP rewrite. Internal origin routing cannot be requested directly.

IPC uses a fixed Unix socket and independent fixture Bearer token. Socket 0600
and root worker exist solely for this isolated pilot. Real deployment requires
non-root worker identity, protected credential injection, verified socket parent
ownership and peer credentials, bounded authority concurrency, replay state and
audit integration. No TCP authority fallback or management endpoint exists.

See the governing nginx-adapter SoT for binding and fail-closed rules. No Cargo
changes, npm package changes or Go code are needed for this njs slice.
