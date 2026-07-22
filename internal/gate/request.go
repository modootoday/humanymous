package gate

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"net"
	"net/http"
	"strings"
	"time"
)

// request.go holds small request-derivation helpers (SRP: keep gate.go focused
// on the pipeline).

// sessionCookie is the session id cookie shared with the control plane.
const sessionCookie = "hsid"

func sessionID(r *http.Request) string {
	if c, err := r.Cookie(sessionCookie); err == nil {
		return c.Value
	}
	return ""
}

// clientIP returns the socket peer IP. Trust headers are stripped at ingress
// (HR-27b), so we derive from RemoteAddr, never from XFF (SoT-23 §4).
func clientIP(r *http.Request) string {
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return host
}

// clientSubnet returns the /24 (IPv4) or /48 (IPv6) of the socket peer — a
// coarse, non-forgeable-from-a-different-machine network anchor used for verdict-
// token binding and rate metering (SoT-28 WS6/WS7).
func clientSubnet(r *http.Request) string {
	ip := net.ParseIP(clientIP(r))
	if ip == nil {
		return clientIP(r)
	}
	if v4 := ip.To4(); v4 != nil {
		return net.IP{v4[0], v4[1], v4[2], 0}.String() + "/24"
	}
	return ip.Mask(net.CIDRMask(48, 128)).String() + "/48"
}

// routeClass classifies the request for policy + audit (SoT-19 §7).
func routeClass(r *http.Request) string {
	if strings.EqualFold(r.Header.Get("Upgrade"), "websocket") || r.Header.Get("Sec-WebSocket-Key") != "" {
		return "upgrade"
	}
	accept := r.Header.Get("Accept")
	dest := r.Header.Get("Sec-Fetch-Dest")
	if dest == "document" || strings.Contains(accept, "text/html") {
		return "html"
	}
	if strings.Contains(accept, "application/json") || dest == "empty" {
		return "api"
	}
	return "static"
}

// route context plumbing so ModifyResponse knows the matched policy.
type routeCtxKey struct{}

func withRoute(ctx context.Context, route routePolicy) context.Context {
	return context.WithValue(ctx, routeCtxKey{}, route)
}

func routeFrom(ctx context.Context) routePolicy {
	if v, ok := ctx.Value(routeCtxKey{}).(routePolicy); ok {
		return v
	}
	return presetBalanced
}

// reqMeta is per-request audit metadata (PLAN-07 R15): one correlation id ties
// together every audit record a single request produces, and start enables latency
// stamping. It rides the request context so downstream gates, the proxy, and the
// upstream ErrorHandler all read the same values.
type reqMetaKey struct{}

type reqMeta struct {
	corr  string
	start time.Time
}

func withReqMeta(ctx context.Context, m reqMeta) context.Context {
	return context.WithValue(ctx, reqMetaKey{}, m)
}

func reqMetaFrom(ctx context.Context) reqMeta {
	if v, ok := ctx.Value(reqMetaKey{}).(reqMeta); ok {
		return v
	}
	return reqMeta{}
}

// latencyUS returns microseconds elapsed since the request started, saturating into
// uint32 (~71 minutes) and never going negative.
func (m reqMeta) latencyUS(now time.Time) uint32 {
	if m.start.IsZero() {
		return 0
	}
	us := now.Sub(m.start).Microseconds()
	if us <= 0 {
		return 0
	}
	if us > int64(^uint32(0)) {
		return ^uint32(0)
	}
	return uint32(us)
}

// newCorrelationID mints a short random per-request id. A CSPRNG failure fails
// closed (panic) rather than emitting correlated records under a predictable id.
func newCorrelationID() string {
	var b [8]byte
	if _, err := rand.Read(b[:]); err != nil {
		panic("gate: CSPRNG failed minting correlation id (fail-closed): " + err.Error())
	}
	return hex.EncodeToString(b[:])
}

// upstreamErrorClass buckets a reverse-proxy transport error into a short, stable
// class for the audit record (PLAN-07 R16) — never the raw error string as a field.
func upstreamErrorClass(err error) string {
	switch {
	case err == nil:
		return ""
	case errors.Is(err, context.Canceled):
		return "canceled"
	case errors.Is(err, context.DeadlineExceeded):
		return "timeout"
	}
	var ne net.Error
	if errors.As(err, &ne) && ne.Timeout() {
		return "timeout"
	}
	msg := err.Error()
	switch {
	case strings.Contains(msg, "connection refused"):
		return "connection_refused"
	case strings.Contains(msg, "no such host"):
		return "dns"
	case strings.Contains(msg, "EOF"):
		return "eof"
	default:
		return "unreachable"
	}
}
