// pass-e2e.mjs — validates humanymous Pass end to end: fetch a scene, SOLVE it with
// the same ballistics the client/server share (proving sim parity), submit, expect a
// pass; then confirm the real-event filter rejects synthetic/degenerate submissions.
const BASE = process.env.BASE || 'https://127.0.0.1:8443';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

const DT=0.10,MAXSTEPS=900,BOUNDS=100,RAMP_B=0.86,DEFL_B=0.90;
const d=(a,b)=>Math.hypot(a.x-b.x,a.y-b.y),sub=(a,b)=>({x:a.x-b.x,y:a.y-b.y}),cr=(a,b)=>a.x*b.y-a.y*b.x;
function segInt(p1,p2,p3,p4){const d1=cr(sub(p3,p4),sub(p3,p1)),d2=cr(sub(p3,p4),sub(p3,p2)),d3=cr(sub(p1,p2),sub(p1,p3)),d4=cr(sub(p1,p2),sub(p1,p4));return((d1>0)!==(d2>0))&&((d3>0)!==(d4>0));}
function crosses(p,q,w){if(!segInt(p,q,w.a,w.b))return null;const wx=w.b.x-w.a.x,wy=w.b.y-w.a.y,l=Math.hypot(wx,wy);if(!l)return null;return{x:-wy/l,y:wx/l};}
function refl(v,n,r){const dot=v.x*n.x+v.y*n.y;return{x:(v.x-2*dot*n.x)*r,y:(v.y-2*dot*n.y)*r};}
function makeRamp(c,a,len){const dx=Math.cos(a)*len/2,dy=Math.sin(a)*len/2;return{a:{x:c.x-dx,y:c.y-dy},b:{x:c.x+dx,y:c.y+dy}};}
function sim(sc,ramp){const segs=[...sc.deflectors,ramp];let p={...sc.ball},v={x:0,y:0};
  for(let i=0;i<MAXSTEPS;i++){v.y+=sc.gravity*DT;let np={x:p.x+v.x*DT,y:p.y+v.y*DT};
    if(d(np,sc.cup)<=sc.cupR||d(p,sc.cup)<=sc.cupR)return true;
    for(let si=0;si<segs.length;si++){const bo=si<sc.deflectors.length?DEFL_B:RAMP_B;const n=crosses(p,np,segs[si]);if(n){v=refl(v,n,bo);np={x:p.x+v.x*DT*0.5,y:p.y+v.y*DT*0.5};break;}}
    p=np;if(p.x<0||p.x>BOUNDS||p.y>BOUNDS)return false;}
  return false;}
function solve(sc){for(let cx=20;cx<=80;cx+=2)for(let cy=25;cy<=80;cy+=2)for(let a=-1.1;a<=1.1;a+=0.15){
  if(cx<98&&cx>2&&d({x:cx,y:cy},sc.ball)>=4&&sim(sc,makeRamp({x:cx,y:cy},a,sc.rampLen)))return{cx,cy,a};}return null;}

let cookie='';
async function api(path,opts={}){const h={...(opts.headers||{})};if(cookie)h.Cookie=cookie;
  const r=await fetch(BASE+path,{...opts,headers:h});const sc=r.headers.get('set-cookie');if(sc)cookie=sc.split(';')[0];
  return r.json();}

const goodProof={moves:24,coalesced:70,trusted:true,pathLen:340,durations:[12,18,9,22,14,31,11,19,8,27],rawT:[1007.344,1021.818,1035.456,1044.006,1054.961,1065.456,1077.972,1091.859,1098.797,1105.081,1119.439,1129.766,1143.389,1149.41,1159.864,1173.079,1181.367,1196.82,1211.834,1218.14,1224.394,1235.808,1251.2,1261.012,1269.178,1279.399,1285.69,1293.906,1304.285,1315.243]};

const nw=await api('/api/pass/new');
if(!nw.scene){console.error('FAIL: no scene',nw);process.exit(1);}
const sol=solve(nw.scene);
if(!sol){console.error('FAIL: client could not solve scene',JSON.stringify(nw.scene));process.exit(1);}
console.log('scene solved client-side at',sol);

// 1. valid placement + valid proof -> pass (proves client/server sim parity)
const r1=await api('/api/pass/solve',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({bucket:nw.bucket,rampX:sol.cx,rampY:sol.cy,rampAngle:sol.a,...goodProof})});
console.log('[1] valid solve ->',JSON.stringify(r1));
if(!r1.ok){console.error('FAIL: valid solve rejected — sim parity broken');process.exit(1);}

// 2. fresh session: untrusted events must be rejected
cookie='';const nw2=await api('/api/pass/new');const s2=solve(nw2.scene);
const r2=await api('/api/pass/solve',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({bucket:nw2.bucket,rampX:s2.cx,rampY:s2.cy,rampAngle:s2.a,...goodProof,trusted:false})});
console.log('[2] untrusted ->',JSON.stringify(r2));
if(r2.ok){console.error('FAIL: untrusted events accepted');process.exit(1);}

// 3. fresh session: uniform timing (CDP tell) must be rejected
cookie='';const nw3=await api('/api/pass/new');const s3=solve(nw3.scene);
const r3=await api('/api/pass/solve',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({bucket:nw3.bucket,rampX:s3.cx,rampY:s3.cy,rampAngle:s3.a,moves:24,coalesced:70,trusted:true,pathLen:340,durations:[16,16,16,16,16,16,16,16]})});
console.log('[3] uniform timing ->',JSON.stringify(r3));
if(r3.ok){console.error('FAIL: uniform-timing (CDP) accepted');process.exit(1);}

// 4. fresh session: a wrong placement must be rejected even with a good proof
cookie='';const nw4=await api('/api/pass/new');
const r4=await api('/api/pass/solve',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({bucket:nw4.bucket,rampX:96,rampY:4,rampAngle:0,...goodProof})});
console.log('[4] wrong placement ->',JSON.stringify(r4));
if(r4.ok){console.error('FAIL: wrong placement accepted');process.exit(1);}

console.log('\n=== ALL PASS-E2E CHECKS OK: sim parity + real-event filter + placement verify ===');
