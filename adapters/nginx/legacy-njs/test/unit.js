import gate from '/pilot/gate.js';
let passed = 0;
function check(value) { if (!value) throw Error('assertion ' + passed); passed++; }
function rejects(fn) { let rejected = false; try { fn(); } catch (_) { rejected = true; } check(rejected); }
check(gate.targetOK('/echo?a=1'));
['/unknown','/echo#x','/echo%2fadmin','/echo%252fadmin','/echo?x=%00','/_humanymous','/echo\\x'].forEach(x => check(!gate.targetOK(x)));
check(gate.project([['X-Forwarded-For','local'],['Cookie','a=1']]).length === 1);
rejects(() => gate.project([['Cookie','a'],['cookie','b']]));
rejects(() => gate.project([['Trailer','digest']]));
rejects(() => gate.project([['Content-Encoding','gzip']]));
rejects(() => gate.project([['Upgrade','websocket']]));
rejects(() => gate.project([['X-Test','a\r\nb']]));
rejects(() => gate.parseReply('{"id":"a","id":"b"}'));
rejects(() => gate.parseReply('{"id":"a","\\u0069d":"b"}'));
rejects(() => gate.parseReply('{"expires":1e9}'));
const req = {protocol:'humanymous.nginx-pilot/0.1',id:'abc',tenant:'pilot',origin:'example',revision:'1',deadline:100};
const reply = Object.assign({},req,{expires:90,verdict:'ALLOW'});
delete reply.deadline;
check(gate.matches(reply,req,80));
check(!gate.matches(reply,req,90));
check(!gate.matches(Object.assign({},reply,{id:'other'}),req,80));
check(!gate.matches(Object.assign({},reply,{extra:true}),req,80));
check(!gate.matches(Object.assign({},reply,{expires:101}),req,80));
check(!gate.matches(Object.assign({},reply,{verdict:'allow'}),req,80));
check(gate.parseReply(JSON.stringify(reply)).id === 'abc');
console.log(JSON.stringify({unitPassed:passed}));
