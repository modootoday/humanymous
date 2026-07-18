// pass_wargame.mjs — the humanymous Pass red/blue wargame runner (SoT-36 §8).
// A red team attacks Pass with several strategies: it SOLVES the physics (a script
// can run the same simulation) and submits with different forged interaction proofs.
// Every attempt self-labels via X-HM-Redteam so it lands in the wargame KPIs. Then
// it reads /api/pass/kpi to print the blue scoreboard (bypass-rate per strategy).
//
// The point of the loop: solving the puzzle is NOT enough — the attacker must also
// forge convincing REAL-EVENT signals. Naive synthetic input is caught; the strategy
// that slips through (if any) is the blue team's next hardening target.
const BASE = process.env.BASE || 'https://127.0.0.1:8443';
process.env.NODE_TLS_REJECT_UNAUTHORIZED = '0';

// --- shared ballistics (identical to internal/pass) ---
const DT=0.10,MAXSTEPS=900,BOUNDS=100,RB=0.86,DB=0.90;
const d=(a,b)=>Math.hypot(a.x-b.x,a.y-b.y),sub=(a,b)=>({x:a.x-b.x,y:a.y-b.y}),cr=(a,b)=>a.x*b.y-a.y*b.x;
const segI=(p1,p2,p3,p4)=>{const d1=cr(sub(p3,p4),sub(p3,p1)),d2=cr(sub(p3,p4),sub(p3,p2)),d3=cr(sub(p1,p2),sub(p1,p3)),d4=cr(sub(p1,p2),sub(p1,p4));return((d1>0)!==(d2>0))&&((d3>0)!==(d4>0));};
const cross=(p,q,w)=>{if(!segI(p,q,w.a,w.b))return null;const wx=w.b.x-w.a.x,wy=w.b.y-w.a.y,l=Math.hypot(wx,wy);if(!l)return null;return{x:-wy/l,y:wx/l};};
const refl=(v,n,r)=>{const dot=v.x*n.x+v.y*n.y;return{x:(v.x-2*dot*n.x)*r,y:(v.y-2*dot*n.y)*r};};
const mk=(c,a,len)=>{const dx=Math.cos(a)*len/2,dy=Math.sin(a)*len/2;return{a:{x:c.x-dx,y:c.y-dy},b:{x:c.x+dx,y:c.y+dy}};};
function sim(sc,ramp){const segs=[...sc.deflectors,ramp];let p={...sc.ball},v={x:0,y:0};for(let i=0;i<MAXSTEPS;i++){v.y+=sc.gravity*DT;let np={x:p.x+v.x*DT,y:p.y+v.y*DT};if(d(np,sc.cup)<=sc.cupR||d(p,sc.cup)<=sc.cupR)return true;for(let si=0;si<segs.length;si++){const bo=si<sc.deflectors.length?DB:RB;const n=cross(p,np,segs[si]);if(n){v=refl(v,n,bo);np={x:p.x+v.x*DT*0.5,y:p.y+v.y*DT*0.5};break;}}p=np;if(p.x<0||p.x>BOUNDS||p.y>BOUNDS)return false;}return false;}
function solve(sc){for(let cx=18;cx<=82;cx+=1.5)for(let cy=24;cy<=82;cy+=1.5)for(let a=-1.15;a<=1.15;a+=0.1){if(cx>2&&cx<98&&d({x:cx,y:cy},sc.ball)>=4&&sim(sc,mk({x:cx,y:cy},a,sc.rampLen)))return{cx,cy,a};}return null;}

// --- forged interaction-proof strategies (the red team's variants) ---
const strategies={
  'A: script (uniform timing)':()=>({moves:24,coalesced:70,trusted:true,pathLen:340,durations:Array(12).fill(16)}),
  'B: dispatchEvent (untrusted)':()=>({moves:24,coalesced:70,trusted:false,pathLen:340,durations:[12,18,9,22,14]}),
  'C: minimal interaction':()=>({moves:2,coalesced:2,trusted:true,pathLen:6,durations:[15,15]}),
  'D: forged plausible stats':()=>{const dur=[];for(let i=0;i<14;i++)dur.push(8+Math.random()*30);return {moves:22,coalesced:64,trusted:true,pathLen:300+Math.random()*80,durations:dur};},
  'E: forged raw stream':()=>{const dur=[],rawT=[];let t=1000;for(let i=0;i<30;i++){t+=6+Math.random()*10;rawT.push(+t.toFixed(3));}for(let i=0;i<14;i++)dur.push(8+Math.random()*30);return {moves:22,coalesced:64,trusted:true,pathLen:300+Math.random()*80,durations:dur,rawT};},
};

let cookie='';
async function api(path,opts={}){const h={...(opts.headers||{}),'X-HM-Redteam':'1'};if(cookie)h.Cookie=cookie;const r=await fetch(BASE+path,{...opts,headers:h});const sc=r.headers.get('set-cookie');if(sc)cookie=sc.split(';')[0];return r.json();}

async function attack(name,forge,rounds=6){let bypass=0,tried=0;
  for(let i=0;i<rounds;i++){cookie='';const nw=await api('/api/pass/new');if(!nw.scene)continue;const s=solve(nw.scene);if(!s)continue;tried++;
    const r=await api('/api/pass/solve',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({bucket:nw.bucket,rampX:s.cx,rampY:s.cy,rampAngle:s.a,...forge()})});
    if(r.ok)bypass++;}
  return {name,tried,bypass};}

console.log('=== humanymous Pass — red/blue wargame ===\nRed solves the physics every time; the question is whether its forged input survives the real-event filter.\n');
const results=[];
for(const [name,forge] of Object.entries(strategies)){const r=await attack(name,forge);results.push(r);
  console.log(`  ${r.bypass>0?'⚠ BYPASS':'✓ blocked'}  ${name.padEnd(30)} ${r.bypass}/${r.tried} passed`);}

// blue scoreboard
const kpi=await (await fetch(BASE+'/api/pass/kpi',{headers:cookie?{Cookie:cookie}:{}})).json();
console.log(`\n=== blue KPI scoreboard ===`);
console.log(`  bypass-rate (bots/red that passed): ${(kpi.bypassRate*100).toFixed(1)}%  over ${kpi.botAttempts} attempts`);
console.log(`  per-difficulty pass-rate:`,kpi.perDifficulty);
const anyBypass=results.some(r=>r.bypass>0);
console.log(`\n${anyBypass?'⚠ A strategy slipped through — the blue team\'s next hardening target (motor-model the forged-stats path, bind to nonce, replay-registry).':'✓ All red strategies blocked at this build. Rotate mechanics + re-run as models improve.'}`);
