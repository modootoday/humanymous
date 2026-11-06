// Actual host HTTP pilots. The authority remains an explicit mock fixture.
import assert from 'node:assert/strict';
import { once } from 'node:events';
import expressFramework from 'express';
import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import { express, hono, VERSION } from '../src/index.mjs';
const decide=async q=>({protocol:VERSION,id:q.id,tenant:q.tenant,origin:q.origin,revision:q.revision,expires:q.deadline,verdict:'ALLOW'});
const options={installation:'test',tenant:'pilot',origin:'pilot',revision:'1',routes:['/'],decide};
const app=expressFramework();app.use(express(options));app.use(expressFramework.json());app.post('/',(req,res)=>res.json(req.body));
const expressServer=app.listen(0,'127.0.0.1');await once(expressServer,'listening');
let honoServer;
try{
  let response=await fetch(`http://127.0.0.1:${expressServer.address().port}/`,{method:'POST',body:'{"pilot":true}',headers:{'content-type':'application/json'}});
  assert.equal(response.status,200);assert.deepEqual(await response.json(),{pilot:true});
  const h=new Hono();h.post('/',async c=>c.body(await c.req.arrayBuffer()));
  const handler=hono(options,h);
  let port;
  if(globalThis.Bun){honoServer=Bun.serve({hostname:'127.0.0.1',port:0,fetch:handler});port=honoServer.port;}
  else {honoServer=serve({hostname:'127.0.0.1',port:0,fetch:handler});if(!honoServer.listening)await once(honoServer,'listening');port=honoServer.address().port;}
  response=await fetch(`http://127.0.0.1:${port}/`,{method:'POST',body:new Uint8Array([0,255,42])});
  assert.equal(response.status,200);assert.deepEqual(new Uint8Array(await response.arrayBuffer()),new Uint8Array([0,255,42]));
  console.log(JSON.stringify({kind:'mock-conformance-host-pilot',runtime:globalThis.Bun?`Bun ${Bun.version}`:process.version,os:process.platform,arch:process.arch,express:'passed',hono:'passed',rustBackend:false}));
}finally{expressServer.closeAllConnections();await new Promise(r=>expressServer.close(r));if(globalThis.Bun)honoServer?.stop(true);else if(honoServer){honoServer.closeAllConnections();await new Promise(r=>honoServer.close(r));}}
