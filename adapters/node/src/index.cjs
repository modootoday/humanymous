'use strict';
const http = require('node:http');
const { randomUUID, createHash } = require('node:crypto');
const { Readable } = require('node:stream');
const VERSION = 'humanymous.adapter/0.1';
const connections = new WeakMap();
const INTERNAL = /^(x-humanymous-|x-forwarded-|forwarded$|cf-connecting-ip$|true-client-ip$|x-real-ip$)/i;
const TOKEN = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
function clean(headers) {
  return headers.filter(([k, v]) => {
    if (!TOKEN.test(k) || /[\r\n\0]/.test(v)) throw Error('invalid header');
    return !INTERNAL.test(k);
  });
}
function sha(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
function localClient({ socketPath, credential, timeoutMs = 2000 }) {
  if (typeof socketPath !== 'string' || !(socketPath.startsWith('/') || /^\\\\\.\\pipe\\[^\\]+$/.test(socketPath)) || socketPath.startsWith('/\0') || !/^[A-Za-z0-9_-]{32,256}$/.test(credential)) throw Error('local IPC path and scoped credential required');
  return (message, signal) => new Promise((resolve, reject) => {
    const body = JSON.stringify(message);
    if (Buffer.byteLength(body) > 1600000) return reject(Error('envelope limit'));
    const req = http.request({ socketPath, path: '/adapter/v0.1/decision', method: 'POST', headers: { authorization: `Bearer ${credential}`, 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) }, signal }, res => {
      let size = 0; const chunks = [];
      res.on('data', chunk => { size += chunk.length; if (size > 65536) res.destroy(Error('response limit')); else chunks.push(chunk); });
      res.on('error', reject);
      res.on('end', () => { try {
        if (res.statusCode !== 200) throw Error('authority transport error');
        const wire = new TextDecoder('utf-8',{fatal:true}).decode(Buffer.concat(chunks));
        const parsed = JSON.parse(wire);
        // Responses are flat scalar objects. Count lexical members before a
        // permissive JSON parser can conceal duplicate names.
        if (!parsed || Array.isArray(parsed) || (wire.match(/"(?:[^"\\]|\\.)*"\s*:/g)||[]).length !== Object.keys(parsed).length) throw Error('duplicate response member');
        resolve(parsed);
      } catch (e) { reject(e); } });
    });
    req.setTimeout(timeoutMs, () => req.destroy(Error('authority timeout')));
    req.on('error', reject); req.end(body);
  });
}
function configure(options) {
  const { installation, tenant, origin, revision, routes, reserved = '/_humanymous', maxBody = 1048576, timeoutMs = 3000 } = options;
  if (![installation, tenant, origin, revision].every(x => typeof x === 'string' && /^[A-Za-z0-9._:-]{1,128}$/.test(x))) throw Error('identity required');
  if (!/^\/[A-Za-z0-9_-]+$/.test(reserved) || !Array.isArray(routes) || routes.some(x => typeof x !== 'string' || !x.startsWith('/') || x === reserved || x.startsWith(reserved + '/'))) throw Error('explicit route inventory collides with reserved namespace');
  if (!Number.isSafeInteger(maxBody) || maxBody < 0 || maxBody > 1048576 || !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 30000) throw Error('invalid limits');
  const decide = options.decide || localClient(options);
  return { installation, tenant, origin, revision, routes, reserved, maxBody, timeoutMs, decide };
}
async function readBody(stream, limit, signal) {
  const chunks = []; let length = 0;
  signal.throwIfAborted();
  const abort=()=>stream.destroy(Error('body cancelled'));
  signal.addEventListener('abort',abort,{once:true});
  try {
    for await (const value of stream) { signal.throwIfAborted(); const chunk = Buffer.from(value); length += chunk.length; if (length > limit) throw Error('body limit'); chunks.push(chunk); }
    signal.throwIfAborted(); return Buffer.concat(chunks);
  } finally { signal.removeEventListener('abort',abort); }
}
async function authorize(c, input, body, signal) {
  if (!TOKEN.test(input.method) || input.method.length>32 || input.method!==input.method.toUpperCase() || Buffer.byteLength(input.target)>8192 || !/^[\x21-\x7e]{1,1024}$/.test(input.authority) || /[@/#?\\]/.test(input.authority)) throw Error('invalid request identity');
  if (!input.target.startsWith('/') || /[\0-\x20\x7f#]/.test(input.target) || /%2f|%5c|%00|\\/i.test(input.target)) throw Error('ambiguous target');
  const path = input.target.split('?')[0];
  if (path.split('/').some(s => { try { return ['.', '..'].includes(decodeURIComponent(s)); } catch { return true; } })) throw Error('ambiguous path');
  if (path === c.reserved || path.startsWith(c.reserved + '/')) throw Error('reserved endpoint unavailable');
  if (!c.routes.includes(path)) throw Error('route absent from inventory');
  if (input.method === 'CONNECT' || input.headers.some(([k,v]) => k.toLowerCase() === 'upgrade' || (k.toLowerCase() === 'content-type' && v.startsWith('application/grpc')))) throw Error('unsupported stream protocol');
  const headers = clean(input.headers);
  const values=name=>headers.filter(([k])=>k.toLowerCase()===name).map(([,v])=>v);
  const lengths=values('content-length'), transfers=values('transfer-encoding'), hosts=values('host');
  if(lengths.length>1 || (lengths.length && transfers.length) || (lengths.length && (!/^(0|[1-9][0-9]*)$/.test(lengths[0]) || Number(lengths[0])!==body.length)) || transfers.length>1 || (transfers.length && transfers[0].toLowerCase()!=='chunked') || hosts.length>1 || (hosts.length && hosts[0]!==input.authority)) throw Error('ambiguous framing or authority');
  if (values('trailer').length) throw Error('trailers unsupported');
  if (headers.length > 100 || headers.reduce((n,[k,v])=>n+Buffer.byteLength(k)+Buffer.byteLength(v),0)>32768) throw Error('header limit');
  const request = { protocol: VERSION, id: randomUUID(), tenant: c.tenant, origin: c.origin, revision: c.revision, deadline: Date.now() + c.timeoutMs, method: input.method, target: input.target, authority: input.authority, peer: input.peer, headers, headerMode: input.headerMode, body: { layer: 'transfer-decoded-content-encoded', complete: true, length: body.length, sha256: sha(body), base64: body.toString('base64') }, capabilities: { rawTls: false, originalH2: false, rawHeaderOrder: input.headerMode === 'node-raw', completeBody: true }, stream: null };
  request.adapter = { id:'humanymous.node', version:'0.1.0-experimental.1', installation:c.installation };
  request.connectionId = input.connectionId;
  request.capture = null;
  signal.throwIfAborted();
  let aborted;
  const cancellation = new Promise((_,reject)=>{aborted=()=>reject(Error('decision cancelled'));signal.addEventListener('abort',aborted,{once:true});});
  let result;
  try { result = await Promise.race([c.decide(request, signal),cancellation]); }
  finally { signal.removeEventListener('abort',aborted); }
  signal.throwIfAborted();
  if (!result || Object.keys(result).sort().join(',') !== 'expires,id,origin,protocol,revision,tenant,verdict' || result.protocol !== VERSION || result.id !== request.id || result.tenant !== c.tenant || result.origin !== c.origin || result.revision !== c.revision || !Number.isSafeInteger(result.expires) || result.expires <= Date.now() || result.expires > request.deadline || !['ALLOW', 'DENY', 'CHALLENGE'].includes(result.verdict)) throw Error('invalid decision binding');
  return { allowed: result.verdict === 'ALLOW', verdict: result.verdict, headers };
}
function express(options) {
  const c = configure(options);
  return async (req, res, next) => {
    const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), c.timeoutMs);
    const removeOriginal = req.removeListener.bind(req);
    const abort = () => controller.abort(); req.once('aborted', abort); res.once('close', abort);
    try {
      if (req.readableEnded || req.body !== undefined || req.readableEncoding || req.httpVersionMajor !== 1) throw Error('install before parsers; HTTP/1 only');
      const headers = []; for (let i=0;i<req.rawHeaders.length;i+=2) headers.push([req.rawHeaders[i],req.rawHeaders[i+1]]);
      const body = await readBody(req, c.maxBody, controller.signal);
      if (req.rawTrailers.length) throw Error('request trailers unsupported');
      if (!connections.has(req.socket)) connections.set(req.socket,randomUUID());
      const decision = await authorize(c, { method:req.method, target:req.url, authority:req.headers.host || '', peer:req.socket.remoteAddress || null, connectionId:connections.get(req.socket), headers, headerMode:'node-raw' }, body, controller.signal);
      if (!decision.allowed) { res.writeHead(decision.verdict === 'DENY' ? 403 : 409, {'content-type':'application/json','cache-control':'no-store'}); res.end(JSON.stringify({verdict:decision.verdict})); return; }
      for (const key of Object.keys(req.headers)) if (INTERNAL.test(key)) delete req.headers[key];
      req.rawHeaders = decision.headers.flat();
      // Delegate subsequent body reads to a bounded replay without mutating Node's
      // private stream state. Request/socket/routing identity remains unchanged.
      const replay = Readable.from(body.length ? [body] : []);
      for (const method of ['on','once','addListener','removeListener','off','pipe','unpipe','read','resume','pause','setEncoding','isPaused']) req[method] = replay[method].bind(replay);
      for (const key of ['readable','readableEnded','readableFlowing','readableLength']) Object.defineProperty(req,key,{configurable:true,get:()=>replay[key]});
      next();
    } catch { if (!res.headersSent && !res.destroyed) { res.writeHead(503, {'cache-control':'no-store'}); res.end('authority unavailable'); } }
    finally { clearTimeout(timer); removeOriginal('aborted',abort); res.removeListener('close',abort); }
  };
}
function fetchAdapter(options, handler) {
  const c = configure(options);
  return async (request, context) => {
    const signal = AbortSignal.any([request.signal, AbortSignal.timeout(c.timeoutMs)]);
    try {
      if (request.bodyUsed) throw Error('install before parsers');
      const url = new URL(request.url);
      const body = request.body ? await readBody(Readable.fromWeb(request.body), c.maxBody, signal) : Buffer.alloc(0);
      const headers = [...request.headers];
      const result = await authorize(c, { method:request.method, target:url.pathname+url.search, authority:url.host, peer:null, connectionId:null, headers, headerMode:'fetch-normalized' }, body, signal);
      if (!result.allowed) return Response.json({verdict:result.verdict}, {status:result.verdict === 'DENY'?403:409,headers:{'cache-control':'no-store'}});
      const init = { method:request.method, headers:result.headers, signal:request.signal, redirect:request.redirect };
      if (!['GET','HEAD'].includes(request.method)) init.body = body;
      return await handler(new Request(request.url,init),context);
    } catch { return new Response('authority unavailable',{status:503,headers:{'cache-control':'no-store'}}); }
  };
}
function hono(options, app) {
  const reserved=options.reserved || '/_humanymous';
  if (!Array.isArray(app.routes) || app.routes.some(route=>route.path===reserved || route.path.startsWith(reserved+'/'))) throw Error('Hono reserved route collision');
  return fetchAdapter(options, (request, env) => app.fetch(request, env));
}
function htmlHook({nonce, scriptPath = '/_humanymous/collector.js'}) {
  if (!/^[A-Za-z0-9+/_=-]{16,128}$/.test(nonce) || !/^\/[A-Za-z0-9/_-]+\.js$/.test(scriptPath)) throw Error('invalid explicit HTML hook');
  return { markup:`<script nonce="${nonce}" src="${scriptPath}" defer></script>`, cspSource:`'nonce-${nonce}'` };
}
module.exports = { VERSION, localClient, express, fetchAdapter, hono, htmlHook };
