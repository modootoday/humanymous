# Experimental Go HTTP origin compatibility

External Go applications consume the shared Rust HTTP Gate. There is no new Go
SDK. Rust deployment contracts live in crates/go-adapter; all authorization,
request parsing, durable audit and forwarding stay in the existing runtime.

net/http, chi, Gin and Echo qualify as separate hosts. Fiber/fasthttp require
separate pooled-context/buffer lifecycle tests. HTTP proxy compatibility does
not imply an in-process middleware binding or upgrade support.

No tuple is currently qualified. Hosted external fixtures and isolated Docker
product results remain NOT_RUN. Unit receipts exercise rejection rules only.
