# Experimental humanymous Node adapter

Thin Express and Hono/Fetch bridge to the single Rust authority. No scoring,
RIT crypto or administrative API is implemented here. This package is not yet
release-qualified: the Rust decision service, verified IPC ACL/peer setup and
full Docker/browser deployment acceptance remain prerequisites.

```js
import { express as guard } from '@humanymous/node';
app.use(guard({ installation: 'node-1', tenant: 'site', origin: 'app', revision: 'config-1',
  routes: ['/checkout'], socketPath: '/run/humanymous/decision.sock',
  credential: process.env.HUMANYMOUS_ADAPTER_CREDENTIAL }));
app.use(express.json()); // guard must precede parsers and origin effects
```

Mount at root with an exhaustive exact pathname inventory. No subsequent route,
method, body or identity rewrite may change the authorized request. Unknown and
reserved routes fail closed. Fetch/Hono use `hono(options, app)` as the outer
server Fetch handler; normalized Fetch headers/targets cannot establish raw
client fidelity. Express body methods delegate to a bounded replay after the
decision. Compressed entity bytes remain compressed. Limits default to 1 MiB
input and 3 seconds including body and authority call; no upload streaming.

`localClient` supports pathname Unix sockets and local Windows named pipes only.
Use independently scoped adapter credentials. The service must validate actual
peer identity, socket ownership/ACLs and replay state; client headers never grant
trust. Public application routers never mount or proxy administration.

ALLOW invokes the application once. DENY returns 403, CHALLENGE returns 409 JSON
(browser recovery not yet connected), dependency/input errors return 503. The
reserved `/_humanymous` subtree currently returns 503; it does not serve scripts.
Configure a separately qualified Rust edge for raw TLS/H2, browser collection,
challenge recovery, large streaming uploads, WebSocket, gRPC and trailers.
Response SSE/download streams pass through without rewriting or buffering.

`htmlHook({nonce})` returns template markup and a CSP nonce source for explicit
application integration only. It does not serve a script, alter CSP, rewrite
HTML or modify cache/signature/compression headers. Application code must supply
the matching nonce and a qualified browser data-plane deployment.

Runtime dependencies: none. Node >=22 is the install floor, not a promise that
every version/OS is qualified. The capability descriptor records boundaries;
release-qualified tuples remain empty pending Rust/Docker/native acceptance.
`decide` injection is for controlled embedding/conformance; untrusted callers
must never choose it or the configured identity/IPC credentials.

CI interface: `npm ci`, `npm test`, `npm run test:pilot`, `npm run test:bun`,
`npm run typecheck`, `npm audit --audit-level=low`, `npm pack --dry-run --json`.
Pack scripts validate ESM/CJS surface and declarations. Publishing requires npm
OIDC workflow, matching reviewed version and complete migration acceptance.

The tests use a mock authority and local host pilots; they are not Rust e2e or
Docker completion evidence. Exact wire requirements are maintained in the
repository Specfold adapter-service-contract source of truth.
