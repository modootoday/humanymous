import hashing from 'crypto';

const allowed = ['content-type', 'authorization', 'cookie', 'accept', 'user-agent'];
const routes = ['/echo', '/deny', '/challenge', '/bad', '/timeout', '/cookies', '/stream'];
function project(raw) {
    const seen = {};
    const headers = [];
    if (raw.length > 100) throw Error('headers');
    let bytes = 0;
    for (let i = 0; i < raw.length; i++) {
        const pair = raw[i];
        const name = pair[0].toLowerCase(), value = pair[1];
        bytes += Buffer.byteLength(name + value);
        if (seen[name] || /[^\x20-\x7e]/.test(value)) throw Error('headers');
        seen[name] = true;
        if (['upgrade', 'trailer', 'transfer-encoding', 'content-encoding'].includes(name)) throw Error('stream');
        if (name === 'connection' && !/^(close|keep-alive)$/i.test(value)) throw Error('connection');
        if (allowed.includes(name)) headers.push([name, value]);
    }
    if (bytes > 32768) throw Error('headers');
    return headers.sort((a,b) => allowed.indexOf(a[0]) - allowed.indexOf(b[0]));
}
function targetOK(target) {
    if (Buffer.byteLength(target) > 8192 || /[\s#\\\x00]/.test(target) || /%(2f|5c|00|2e|25)/i.test(target)) return false;
    const path = target.split('?')[0];
    return routes.includes(path) && !path.split('/').some(x => x === '.' || x === '..');
}
function matches(reply, req, now) {
    const keys = ['protocol','id','tenant','origin','revision','expires','verdict'];
    return reply && Object.keys(reply).sort().join() === keys.sort().join()
        && ['protocol','id','tenant','origin','revision'].every(k => reply[k] === req[k])
        && Number.isSafeInteger(reply.expires) && reply.expires > now && reply.expires <= req.deadline
        && ['ALLOW','DENY','CHALLENGE'].includes(reply.verdict);
}
function parseReply(text) {
    // Flat ASCII identity schema: reject escapes, duplicate keys and non-integers
    // before JSON.parse can erase wire ambiguity.
    const grammar = /^\s*\{\s*"[a-z]+"\s*:\s*(?:"[A-Za-z0-9._:\/-]+"|[0-9]+)(?:\s*,\s*"[a-z]+"\s*:\s*(?:"[A-Za-z0-9._:\/-]+"|[0-9]+))*\s*\}\s*$/;
    if (!grammar.test(text)) throw Error('response grammar');
    const fields = text.match(/"[a-z]+"\s*:/g);
    const seen = {};
    fields.forEach(field => {
        const key = field.split('"')[1];
        if (seen[key]) throw Error('duplicate response field');
        seen[key] = true;
    });
    return JSON.parse(text);
}
async function gate(r) {
    r.headersOut['Cache-Control'] = 'no-store';
    try {
        const target = r.variables.request_uri;
        if (!targetOK(target) || !['GET','POST'].includes(r.method) || r.headersIn.host !== 'example.test') return r.return(400);
        const headers = project(r.rawHeadersIn);
        const body = Number(r.headersIn['Content-Length'] || 0) > 0 ? Buffer.from(r.requestBuffer) : Buffer.from('');
        if (r.variables.request_body_file || body.length > 65536 || (r.headersIn['Content-Length'] && Number(r.headersIn['Content-Length']) !== body.length)) return r.return(413);
        const req = {protocol:'humanymous.nginx-pilot/0.1',adapter:{id:'humanymous.nginx',version:'0.1.0-experimental.1',installation:'pilot'},connectionId:null,capture:null,stream:null,headerMode:'nginx-projected',id:Buffer.from(crypto.getRandomValues(new Uint8Array(16))).toString('hex'),tenant:'pilot',origin:'example',revision:'1',deadline:Date.now()+1500,method:r.method,target,authority:'example.test',headers,
            body:{layer:'transfer-decoded-content-encoded',complete:true,length:body.length,sha256:hashing.createHash('sha256').update(body.length ? body : '').digest('hex'),base64:body.toString('base64')},capabilities:{rawTls:false,originalH2:false,rawHeaderOrder:false,completeBody:true}};
        const result = await r.subrequest('/_ipc', {method:'POST',body:JSON.stringify(req)});
        if (result.status !== 200 || result.responseBuffer.length > 65536) return r.return(503);
        const reply = parseReply(result.responseText);
        if (!matches(reply,req,Date.now())) return r.return(503);
        if (reply.verdict === 'DENY') return r.return(403);
        if (reply.verdict === 'CHALLENGE') return r.return(409, '{"error":"challenge unavailable"}');
        r.internalRedirect('@origin');
    } catch (_) { r.return(503); }
}
export default {gate, project, targetOK, matches, parseReply};
