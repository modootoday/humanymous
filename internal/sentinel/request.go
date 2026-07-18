package sentinel

import (
	"context"
	"net"
	"net/http"
	"strings"
)

// request.go holds small request-derivation helpers (SRP: keep sentinel.go focused
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
