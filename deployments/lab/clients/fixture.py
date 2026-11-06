"""Owned synthetic origin/canary. Never an authority or Gate replacement."""
import http.client
import json
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = json.dumps({"fixture": sys.argv[1]}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_):
        pass


if sys.argv[1] in ("health", "control"):
    host = "127.0.0.1" if sys.argv[1] == "health" else sys.argv[2]
    client = http.client.HTTPConnection(host, 8080, timeout=2)
    client.request("GET", "/")
    response = client.getresponse()
    assert response.status == 200
    assert json.loads(response.read())["fixture"] == "origin" or sys.argv[1] == "health"
    print(json.dumps({"control": "fixture-reachable", "host": host, "status": "passed"}))
    client.close()
else:
    HTTPServer(("0.0.0.0", 8080), Handler).serve_forever()
