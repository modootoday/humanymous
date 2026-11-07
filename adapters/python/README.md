# Experimental Python native adapter

Rust-native `humanymous_python.Wsgi(app,options_json)` and
`humanymous_python.Asgi(app,options_json)` are source implementations awaiting
compilation and actual framework pilots. No wheel or runtime tuple is qualified.
The candidate PyPI distribution name is unverified and no publish is authorized.

Install at the outer application boundary before parsing/effects. WSGI requires
an actual raw target server extension, exact CONTENT_LENGTH, HTTP_HOST and a
server-enforced input deadline. ASGI requires raw_path and HTTP1 scope and rejects
other protocols. A route not in the explicit configured inventory fails closed.
Default missing browser/capture planes do not become ALLOW.

The required configuration fields and host lifecycle are specified in
`.spec/sot/python-ruby-adapters.sot.md`. Private configuration must come from a
trusted supervisor; it includes the authority credential and actual authority
UID or SID/PID. Never place configuration JSON in a public header or log it.
The middleware exposes no management route or browser recovery flow.

Host response iterables/send callbacks remain framework-owned. Native callback,
streaming/cancellation, contextvars, GIL, CPython ABI, package clean installation
and actual FastAPI/Starlette/Django/Flask tests remain NOT_RUN. Free-threaded
CPython, subinterpreters, PyPy, ABI3, WebSocket and request streaming are not
qualified. Future support is based on direct runtime evidence.
