import { test } from 'node:test';
import assert from 'node:assert/strict';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { readFileSync } from 'node:fs';
import Ajv from 'ajv/dist/2020.js';
import expressFramework from 'express';
import { Hono } from 'hono';
import { express, fetchAdapter, hono, htmlHook, localClient, VERSION } from '../src/index.mjs';
export const options = {installation:'test',tenant:'tenant',origin:'origin',revision:'rev',routes:['/','/sse','/redirect','/cookies','/empty']};
export const allow = async q => ({protocol:VERSION,id:q.id,tenant:q.tenant,origin:q.origin,revision:q.revision,expires:q.deadline,verdict:'ALLOW'});
test('shared structural schemas accept captured requests and reject negatives',async()=>{
  const ajv=new Ajv({strict:true});
  const requestSchema=ajv.compile(JSON.parse(readFileSync(new URL('../../contracts/adapter-0.1-request.schema.json',import.meta.url))));
  const responseSchema=ajv.compile(JSON.parse(readFileSync(new URL('../../contracts/adapter-0.1-response.schema.json',import.meta.url))));
  let captured;
  const adapter=fetchAdapter({...options,decide:async q=>{captured=q;return allow(q);}},()=>new Response('ok'));
  assert.equal((await adapter(new Request('http://example/'))).status,200);
  assert.ok(requestSchema(captured),JSON.stringify(requestSchema.errors));assert.ok(responseSchema(await allow(captured)));
  for(const mutation of [{stream:{}},{protocol:'2'},{peer:42},{unexpected:true},{body:{...captured.body,complete:false}},{headers:[['bad name','value']]}])assert.equal(requestSchema({...captured,...mutation}),false);
  for(const mutation of [{verdict:'ERROR'},{expires:null},{extra:true}])assert.equal(responseSchema({...await allow(captured),...mutation}),false);
});
test('Fetch preserves binary body; strips asserted trust; body bound before effect', async () => {
  let observed; let effects=0;
  const adapter = fetchAdapter({...options,decide:async q=>{observed=q;assert.equal(effects,0);return allow(q);}},async req=>{effects++; return new Response(await req.arrayBuffer());});
  const bytes = new Uint8Array([0,255,13,10]);
  const response = await adapter(new Request('http://example/',{method:'POST',body:bytes,headers:{'x-humanymous-verdict':'ALLOW','x-forwarded-for':'127.0.0.1',authorization:'Bearer app'}}));
  assert.equal(response.status,200);assert.deepEqual(new Uint8Array(await response.arrayBuffer()),bytes);
  assert.equal(observed.peer,null);assert.equal(observed.capabilities.rawTls,false);
  assert.deepEqual(observed.headers,[['authorization','Bearer app']]);assert.equal(observed.body.base64,Buffer.from(bytes).toString('base64'));
});
test('all invalid/missing/stale/cross-scope decisions fail closed', async () => {
  for (const patch of [{id:'other'},{tenant:'other'},{origin:'other'},{revision:'other'},{expires:0},{expires:Infinity},{protocol:'new'},{verdict:'allow'},{admin:'oops'}]) {
    let effects=0;const adapter=fetchAdapter({...options,decide:async q=>({...await allow(q),...patch})},()=>{effects++;return new Response();});
    assert.equal((await adapter(new Request('http://example/'))).status,503);assert.equal(effects,0);
  }
});
test('deny/challenge/errors never invoke application',async()=>{
  for(const verdict of ['DENY','CHALLENGE','ERROR']){
    const adapter=fetchAdapter({...options,decide:async q=>({...await allow(q),verdict})},()=>assert.fail('origin effect'));
    assert.equal((await adapter(new Request('http://example/'))).status,verdict==='DENY'?403:verdict==='CHALLENGE'?409:503);
  }
});
test('namespace collision, unknown route, body limits and unsupported protocols',async()=>{
  assert.throws(()=>fetchAdapter({...options,routes:['/_humanymous/collect'],decide:allow},()=>new Response()));
  const collision=new Hono();collision.get('/_humanymous/collect',()=>new Response());assert.throws(()=>hono({...options,decide:allow},collision));
  for(const [path,init] of [['/_humanymous/admin',{}],['/unknown',{}],['/',{method:'POST',body:'too big'}],['/',{headers:{upgrade:'websocket'}}],['/',{headers:{'content-type':'application/grpc'}}]]){
    const adapter=fetchAdapter({...options,maxBody:2,decide:()=>assert.fail('must not decide')},()=>assert.fail('must not effect'));
    assert.equal((await adapter(new Request('http://example'+path,init))).status,503);
  }
});
test('real Express body parser consumes exact replay after authorization',async()=>{
  const app=expressFramework();let calls=0;
  app.use(express({...options,decide:async q=>{calls++;assert.equal(q.headerMode,'node-raw');return allow(q);}}));
  app.use(expressFramework.raw({type:'*/*'}));app.post('/',(req,res)=>{assert.equal(req.headers['x-forwarded-for'],undefined);res.send(req.body);});
  const server=app.listen(0,'127.0.0.1');await once(server,'listening');
  try {const response=await fetch(`http://127.0.0.1:${server.address().port}/`,{method:'POST',headers:{'content-type':'application/octet-stream','x-forwarded-for':'bad'},body:Buffer.from([0,255,1])});assert.equal(response.status,200);assert.deepEqual(Buffer.from(await response.arrayBuffer()),Buffer.from([0,255,1]));assert.equal(calls,1);}finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
});
test('real Hono wrapper preserves redirects, cookies, SSE and empty response',async()=>{
  const app=new Hono();app.get('/redirect',c=>c.redirect('/'));app.get('/cookies',()=>new Response('ok',{headers:[['set-cookie','a=1; Path=/'],['set-cookie','b=2; Path=/']]}));
  app.get('/sse',()=>new Response(new ReadableStream({start(c){c.enqueue(new TextEncoder().encode('data: first\n\n'));c.close();}}),{headers:{'content-type':'text/event-stream'}}));app.get('/empty',()=>new Response(null,{status:204}));
  const adapter=hono({...options,decide:allow},app);
  assert.equal((await adapter(new Request('http://example/redirect'))).headers.get('location'),'/');
  assert.equal((await adapter(new Request('http://example/cookies'))).headers.getSetCookie().length,2);
  assert.equal(await(await adapter(new Request('http://example/sse'))).text(),'data: first\n\n');
  assert.equal((await adapter(new Request('http://example/empty'))).status,204);
});
test('cancellation and deadline prevent origin effects',async()=>{
  const controller=new AbortController();controller.abort();
  const adapter=fetchAdapter({...options,timeoutMs:10,decide:async(q,signal)=>{await new Promise(r=>setTimeout(r,20));return allow(q);}},()=>assert.fail('origin effect'));
  assert.equal((await adapter(new Request('http://example/',{signal:controller.signal}))).status,503);
  assert.equal((await adapter(new Request('http://example/'))).status,503);
});
test('never-resolving authority and input streams are bounded by cancellation',async()=>{
  const adapter=fetchAdapter({...options,timeoutMs:15,decide:()=>new Promise(()=>{})},()=>assert.fail('origin effect'));
  assert.equal((await adapter(new Request('http://example/'))).status,503);
  let cancelled=false;
  const stream=new ReadableStream({pull(){return new Promise(()=>{});},cancel(){cancelled=true;}});
  assert.equal((await adapter(new Request('http://example/',{method:'POST',body:stream,duplex:'half'}))).status,503);
  assert.equal(cancelled,true);
});
test('response streams retain identity and downstream backpressure',async()=>{
  let pulls=0;let cancelled=false;
  const stream=new ReadableStream({pull(c){pulls++;c.enqueue(new Uint8Array([pulls]));},cancel(){cancelled=true;}},{highWaterMark:0});
  const original=new Response(stream,{headers:{'content-type':'text/event-stream','etag':'"original"'}});
  const adapter=fetchAdapter({...options,decide:allow},()=>original);
  const result=await adapter(new Request('http://example/sse'));
  assert.equal(result,original);assert.equal(pulls,0);const reader=result.body.getReader();await reader.read();assert.equal(pulls,1);await reader.cancel();assert.equal(cancelled,true);
});
test('normalization ambiguity and framing disagreement never reach authority',async()=>{
  for(const [path,headers] of [['/%2fadmin',{}],['/',{'host':'other'}],['/',{'content-length':'2'}],['/',{'trailer':'x-test'}]]){
    const adapter=fetchAdapter({...options,decide:()=>assert.fail('must not decide')},()=>assert.fail('origin effect'));
    assert.equal((await adapter(new Request('http://example'+path,{headers}))).status,503);
  }
});
test('IPC client refuses network URLs and malformed credentials; HTML hook is explicit',()=>{
  assert.throws(()=>localClient({socketPath:'http://localhost',credential:'a'.repeat(32)}));assert.throws(()=>localClient({socketPath:'/run/test',credential:'bad'}));
  assert.throws(()=>htmlHook({nonce:'" onload="x'}));assert.match(htmlHook({nonce:'a'.repeat(24)}).markup,/defer/);
});
test('IPC framing and authorization roundtrip (mock conformance)',async()=>{
  const path=process.platform==='win32'?`\\\\.\\pipe\\humanymous-test-${process.pid}`:`/tmp/humanymous-test-${process.pid}.sock`;
  const server=createServer(async(req,res)=>{assert.equal(req.url,'/adapter/v0.1/decision');assert.equal(req.headers.authorization,'Bearer '+'a'.repeat(32));const chunks=[];for await(const chunk of req)chunks.push(chunk);res.end(JSON.stringify(await allow(JSON.parse(Buffer.concat(chunks)))));});
  server.listen(path);await once(server,'listening');
  try {const adapter=fetchAdapter({...options,socketPath:path,credential:'a'.repeat(32)},()=>new Response('ok'));assert.equal((await adapter(new Request('http://example/'))).status,200);}finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
});
test('IPC malformed, duplicate, oversized and non-200 responses fail closed',async()=>{
  const path=process.platform==='win32'?`\\\\.\\pipe\\humanymous-negative-${process.pid}`:`/tmp/humanymous-negative-${process.pid}.sock`;
  let mode='duplicate';
  const server=createServer(async(req,res)=>{
    const chunks=[];for await(const chunk of req)chunks.push(chunk);const q=JSON.parse(Buffer.concat(chunks));
    if(mode==='duplicate')res.end(JSON.stringify(await allow(q)).replace('"verdict":"ALLOW"','"verdict":"DENY","verdict":"ALLOW"'));
    else if(mode==='malformed')res.end(Buffer.from([0xff]));
    else if(mode==='oversized')res.end('x'.repeat(65537));
    else{res.statusCode=401;res.end('{}');}
  });server.listen(path);await once(server,'listening');
  try{const adapter=fetchAdapter({...options,socketPath:path,credential:'a'.repeat(32)},()=>assert.fail('origin effect'));
    for(mode of ['duplicate','malformed','oversized','unauthorized'])assert.equal((await adapter(new Request('http://example/'))).status,503);
  }finally{server.closeAllConnections();await new Promise(r=>server.close(r));}
});
