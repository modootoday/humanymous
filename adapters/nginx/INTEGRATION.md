# Authority-owner integration request

This pilot intentionally does not use shared humanymous.adapter/0.1 because that
schema authenticates humanymous.node and node-raw/fetch-normalized only.

Required joint decision before integration:
- approve adapter id humanymous.nginx and nginx-projected headerMode;
- explicitly bind fixed Host plus ordered five-header projection, generated
  Content-Length/Connection framing, exact request_uri and entity body digest;
- 64 KiB body bound, connectionId/capture/stream null, rawTls/originalH2 false;
- reuse authority credential scope, verified actual Unix peer/ACL, capability
  enforcement, nonce replay, revision/deadline and durable audit requirements;
- unsupported required raw capture must produce failure, never synthetic ALLOW;
- retain ALLOW/DENY/CHALLENGE exact identity/expiry response and zero origin
  effects before successful binding; recovery endpoints remain unavailable.

No shared schema/service files were modified. nginx-pilot/0.1 is an isolated
mock protocol, not a claim of compatibility with the real Rust service.
No new Rust crate or workspace integration is requested by this njs slice.
