package gate

import (
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"net"
	"strconv"
	"strings"
	"time"
)

// proxyproto.go recovers the true client address when the gate sits behind an L4 /
// TCP-passthrough load balancer (PLAN-08 R4). In that deployment the gate still
// terminates TLS itself — so the real ClientHello (and thus the JA3/JA4 signals in
// internal/network) is unaffected — but the TCP peer is the balancer, not the client,
// which would break every ban / rate-limit / correlation keyed on the client IP.
//
// The balancer prepends a PROXY protocol v2 header (HAProxy send-proxy-v2, nginx
// stream proxy_protocol) declaring the real source. This wrapper parses that header
// BEFORE the TLS handshake and overrides RemoteAddr — but ONLY when the TCP peer is
// in a configured trusted-proxy CIDR allowlist, so an attacker connecting directly
// can never inject a spoofed real IP (their header is not parsed; their socket IP
// stands). A LOCAL command (health checks) or a non-trusted peer passes through
// unchanged.

// proxyV2Sig is the 12-byte PROXY protocol v2 signature.
var proxyV2Sig = []byte{0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A}

var errNotProxyV2 = errors.New("gate: not a PROXY v2 header")

// proxyListener wraps a listener, recovering the real client address from a PROXY v2
// header sent by a trusted balancer.
type proxyListener struct {
	net.Listener
	trusted []*net.IPNet
	timeout time.Duration
}

// WrapProxyProto wraps ln so connections from a trusted-proxy CIDR are read for a
// PROXY v2 header. trusted must be non-empty; with no trusted CIDRs the caller should
// not wrap at all (every peer would be untrusted and the header never parsed).
func WrapProxyProto(ln net.Listener, trusted []*net.IPNet) net.Listener {
	return &proxyListener{Listener: ln, trusted: trusted, timeout: 5 * time.Second}
}

// ParseCIDRs parses a comma/space-separated list of CIDRs (and bare IPs) into nets.
func ParseCIDRs(list string) ([]*net.IPNet, error) {
	var out []*net.IPNet
	for _, f := range strings.FieldsFunc(list, func(r rune) bool { return r == ',' || r == ' ' }) {
		if !strings.Contains(f, "/") {
			if ip := net.ParseIP(f); ip != nil {
				bits := 32
				if ip.To4() == nil {
					bits = 128
				}
				f += "/" + strconv.Itoa(bits)
			}
		}
		_, n, err := net.ParseCIDR(f)
		if err != nil {
			return nil, err
		}
		// Reject a dangerously broad trusted-proxy range (PLAN-08 backlog): a /0 (or a
		// near-/0) would let ANY client spoof ANY source IP via a self-supplied PROXY-v2
		// header — the exact abuse the trust gate exists to prevent. Fail closed on the
		// misconfig rather than silently trusting the whole internet.
		if ones, bits := n.Mask.Size(); ones < 8 {
			return nil, fmt.Errorf("gate: trusted-proxy CIDR %s is too broad (/%d of /%d); "+
				"list your balancers' addresses, never a /0", f, ones, bits)
		}
		out = append(out, n)
	}
	return out, nil
}

func (l *proxyListener) trusts(addr net.Addr) bool {
	host, _, err := net.SplitHostPort(addr.String())
	if err != nil {
		return false
	}
	ip := net.ParseIP(host)
	if ip == nil {
		return false
	}
	for _, n := range l.trusted {
		if n.Contains(ip) {
			return true
		}
	}
	return false
}

// Accept returns the next connection, overriding RemoteAddr with the PROXY-declared
// source when the peer is a trusted balancer.
func (l *proxyListener) Accept() (net.Conn, error) {
	for {
		c, err := l.Listener.Accept()
		if err != nil {
			return nil, err
		}
		if !l.trusts(c.RemoteAddr()) {
			return c, nil // untrusted peer: never honor a PROXY header (anti-spoof)
		}
		real, perr := readProxyV2(c, l.timeout)
		if perr != nil {
			_ = c.Close() // a trusted proxy sent a malformed header: drop, keep serving
			continue
		}
		if real == nil {
			return c, nil // LOCAL command / UNSPEC: use the real peer
		}
		return &proxyConn{Conn: c, remote: real}, nil
	}
}

// proxyConn overrides RemoteAddr with the recovered client address; all other I/O
// (including the TLS handshake that follows the header) is the underlying conn.
type proxyConn struct {
	net.Conn
	remote net.Addr
}

func (c *proxyConn) RemoteAddr() net.Addr { return c.remote }

// readProxyV2 consumes exactly one PROXY v2 header from c and returns the declared
// source address (nil for a LOCAL command). It reads ONLY the header bytes, so the
// following TLS ClientHello is left intact on the connection.
func readProxyV2(c net.Conn, timeout time.Duration) (net.Addr, error) {
	_ = c.SetReadDeadline(time.Now().Add(timeout))
	defer func() { _ = c.SetReadDeadline(time.Time{}) }()

	hdr := make([]byte, 16)
	if _, err := io.ReadFull(c, hdr); err != nil {
		return nil, err
	}
	if !bytes.Equal(hdr[:12], proxyV2Sig) || hdr[12]&0xF0 != 0x20 {
		return nil, errNotProxyV2 // signature/version mismatch
	}
	alen := int(binary.BigEndian.Uint16(hdr[14:16]))
	block := make([]byte, alen)
	if _, err := io.ReadFull(c, block); err != nil {
		return nil, err
	}
	if hdr[12]&0x0F == 0x00 { // LOCAL (health check): no real client to recover
		return nil, nil
	}
	switch hdr[13] { // address family + transport
	case 0x11: // TCP over IPv4
		if alen < 12 {
			return nil, errNotProxyV2
		}
		return &net.TCPAddr{IP: append(net.IP(nil), block[0:4]...), Port: int(binary.BigEndian.Uint16(block[8:10]))}, nil
	case 0x21: // TCP over IPv6
		if alen < 36 {
			return nil, errNotProxyV2
		}
		return &net.TCPAddr{IP: append(net.IP(nil), block[0:16]...), Port: int(binary.BigEndian.Uint16(block[32:34]))}, nil
	default:
		return nil, nil // UNSPEC / UDP: fall back to the real peer
	}
}
