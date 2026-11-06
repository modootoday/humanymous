"""Local Docker-only mock authority/origin and black-box compatibility pilot."""
import base64
import hashlib
import http.client
import http.server
import json
import os
import socketserver
import sys
import threading
import time

lock = threading.Lock()
effects = []
seen = set()

class Origin(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_GET(self): self.handle_request()
    def do_POST(self): self.handle_request()
    def handle_request(self):
        body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        if self.path == '/state':
            with lock: data = json.dumps(effects).encode()
        else:
            item = {'target': self.path, 'method': self.command, 'body': base64.b64encode(body).decode(), 'headers': list(self.headers.items())}
            with lock: effects.append(item)
            data = json.dumps(item).encode()
        if self.path == '/stream':
            self.send_response(200)
            self.send_header('Content-Type','text/event-stream')
            self.send_header('Transfer-Encoding','chunked')
            self.end_headers()
            self.wfile.write(b'9\r\ndata: a\n\n\r\n')
            self.wfile.flush()
            time.sleep(1)
            self.wfile.write(b'9\r\ndata: b\n\n\r\n0\r\n\r\n')
            self.wfile.flush()
            return
        self.send_response(200)
        self.send_header('Set-Cookie', 'a=1; HttpOnly')
        self.send_header('Set-Cookie', 'b=2; Secure')
        self.send_header('X-Repeat', 'one')
        self.send_header('X-Repeat', 'two')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

class UnixHTTP(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True

class Authority(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        req = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        assert self.path == '/adapter/v0.1/decision'
        assert self.headers['Authorization'] == 'Bearer ' + 'a' * 32
        assert req['protocol'] == 'humanymous.nginx-pilot/0.1'
        body = base64.b64decode(req['body']['base64'], validate=True)
        assert len(body) == req['body']['length']
        assert hashlib.sha256(body).hexdigest() == req['body']['sha256']
        assert not any(n.startswith(('x-forwarded-', 'x-humanymous-')) for n, _ in req['headers'])
        with lock:
            assert req['id'] not in seen
            seen.add(req['id'])
        path = req['target'].split('?')[0]
        if path == '/timeout': time.sleep(2)
        reply = {k: req[k] for k in ['protocol', 'id', 'tenant', 'origin', 'revision']}
        reply.update(expires=req['deadline'], verdict={'/deny':'DENY', '/challenge':'CHALLENGE'}.get(path, 'ALLOW'))
        if path == '/bad':
            mode = req['target'].partition('?')[2]
            if mode == 'expired': reply['expires'] = 0
            elif mode == 'extra': reply['extra'] = True
            elif mode == 'scope': reply['tenant'] = 'other'
            elif mode == 'verdict': reply['verdict'] = 'UNKNOWN'
            elif mode != 'duplicate': reply['id'] = 'mismatch'
        if req['target'] == '/bad?disconnect':
            self.close_connection = True
            return
        data = json.dumps(reply).encode()
        if req['target'] == '/bad?duplicate': data = data[:-1] + b',"id":"fake"}'
        self.send_response(200)
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        try: self.wfile.write(data)
        except BrokenPipeError: pass

def request(path, method='GET', body=None, headers=None, host='nginx', port=8080):
    c = http.client.HTTPConnection(host, port, timeout=5)
    c.request(method, path, body=body, headers={'Host':'example.test', **(headers or {})})
    r = c.getresponse()
    result = r.status, r.getheaders(), r.read()
    c.close()
    return result

def count(): return len(json.loads(request('/state', host='fixture', port=9000)[2]))

def tests():
    for _ in range(60):
        try:
            request('/unknown')
            break
        except OSError: time.sleep(.2)
    tests = 0
    for path, method, body, headers, status in [
        ('/deny','GET',None,{},403), ('/challenge','GET',None,{},409),
        ('/bad','GET',None,{},503), *[(f'/bad?{mode}','GET',None,{},503) for mode in ['expired','extra','scope','verdict','duplicate','disconnect']], ('/timeout','GET',None,{},503),
        ('/_humanymous/admin','GET',None,{},400), ('/_ipc','GET',None,{},404),
        ('/unknown','GET',None,{},400), ('/echo','CONNECT',None,{},400),
        ('/echo','GET',None,{'Upgrade':'websocket','Connection':'upgrade'},503),
        ('/echo','GET',None,{'Trailer':'digest'},503),
        ('/echo','POST',b'compressed',{'Content-Encoding':'gzip'},503),
        ('/echo','GET',None,{'Host':'evil.test'},400),
        ('/echo%2fadmin','GET',None,{},400),
        ('/echo','POST',b'x'*65537,{},413),
    ]:
        before = count()
        actual = request(path, method, body, headers)[0]
        assert actual == status, (path, actual, status)
        assert count() == before, ('origin effect',path)
        tests += 1
    for body in [b'', b'\x00\xff\x80binary\r\n', b'x'*65536]:
        before = count()
        status, headers, data = request('/echo?q=untouched%20query','POST',body,{'Content-Type':'application/octet-stream','X-Forwarded-For':'127.0.0.1','X-Humanymous-Allow':'true','Forwarded':'for=127.0.0.1','X-Arbitrary':'drop'})
        assert status == 200, (status,data)
        item = json.loads(data)
        assert base64.b64decode(item['body']) == body
        assert item['target'] == '/echo?q=untouched%20query'
        assert not any(n.lower().startswith(('x-', 'forwarded')) for n,v in item['headers'])
        assert count() == before + 1
        tests += 1
    status, headers, _ = request('/cookies')
    assert status == 200
    assert len([v for k,v in headers if k.lower() == 'set-cookie']) == 2
    assert len([v for k,v in headers if k.lower() == 'x-repeat']) == 2
    tests += 1
    # Raw duplicate request header must have no origin effect.
    import socket
    before = count()
    with socket.create_connection(('nginx',8080)) as sock:
        sock.sendall(b'GET /echo HTTP/1.1\r\nHost: example.test\r\nX-Dup: a\r\nX-Dup: b\r\nConnection: close\r\n\r\n')
        assert b'503' in sock.recv(1024).split(b'\r\n')[0]
    assert count() == before
    tests += 1
    # Missing IPC endpoint must fail closed with no TCP fallback or origin effect.
    before = count()
    os.rename('/ipc/authority.sock', '/ipc/authority.offline')
    try:
        assert request('/echo')[0] == 503
        assert count() == before
    finally:
        os.rename('/ipc/authority.offline', '/ipc/authority.sock')
    tests += 1
    # Ambiguous framing is rejected by nginx before JS or origin.
    before = count()
    with socket.create_connection(('nginx',8080)) as sock:
        sock.sendall(b'POST /echo HTTP/1.1\r\nHost: example.test\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n0\r\n\r\n')
        assert b'400' in sock.recv(1024).split(b'\r\n')[0]
    assert count() == before
    tests += 1
    # nginx proxy buffering is off: the first SSE event arrives before completion.
    c = http.client.HTTPConnection('nginx',8080,timeout=5)
    started = time.monotonic()
    c.request('GET','/stream',headers={'Host':'example.test'})
    stream = c.getresponse()
    assert stream.status == 200
    assert stream.read(9) == b'data: a\n\n'
    assert time.monotonic() - started < .8
    assert stream.read() == b'data: b\n\n'
    c.close()
    tests += 1
    print(json.dumps({'passed':tests,'authority':'mock Unix socket','rust_integration':False}))

if __name__ == '__main__':
    if sys.argv[1] == 'serve':
        try: os.unlink('/ipc/authority.sock')
        except FileNotFoundError: pass
        authority = UnixHTTP('/ipc/authority.sock', Authority)
        os.chmod('/ipc/authority.sock', 0o600)
        threading.Thread(target=authority.serve_forever,daemon=True).start()
        http.server.ThreadingHTTPServer(('0.0.0.0',9000), Origin).serve_forever()
    else: tests()
