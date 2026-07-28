package main

import (
	"net"
	"sync"

	"github.com/modootoday/humanymous/internal/network"
)

// tlsconn.go captures the raw TLS ClientHello off each connection before the
// TLS handshake consumes it, so we can compute JA3/JA4 including extension order
// (which crypto/tls does not expose — golang/go#32936). See plan/02 §3.1.

// captureConn wraps a net.Conn and snapshots bytes of the first TLS record
// (the ClientHello) as the handshake reads them.
type captureConn struct {
	net.Conn
	reg   *connRegistry
	mu    sync.Mutex
	buf   []byte
	done  bool
	hello *network.ClientHello
}

func (c *captureConn) Read(p []byte) (int, error) {
	n, err := c.Conn.Read(p)
	if n > 0 && !c.done {
		c.mu.Lock()
		c.buf = append(c.buf, p[:n]...)
		c.tryParse()
		c.mu.Unlock()
	}
	return n, err
}

// tryParse attempts to parse a complete ClientHello record from the buffer.
// Caller holds c.mu.
func (c *captureConn) tryParse() {
	if len(c.buf) < 5 {
		return
	}
	// TLS record: type(1)=0x16 handshake, version(2), length(2).
	if c.buf[0] != 0x16 {
		c.done = true // not a TLS handshake; stop capturing
		return
	}
	recLen := int(c.buf[3])<<8 | int(c.buf[4])
	if len(c.buf) < 5+recLen {
		return // wait for more bytes
	}
	hello, err := network.ParseRecord(c.buf[:5+recLen])
	if err == nil {
		c.hello = hello
	}
	c.done = true
	c.buf = nil // release
}

// Hello returns the captured ClientHello (nil if not captured yet).
func (c *captureConn) Hello() *network.ClientHello {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.hello
}

// Close removes the connection from the registry so lookups don't leak.
func (c *captureConn) Close() error {
	if c.reg != nil {
		c.reg.remove(c.RemoteAddr().String())
	}
	return c.Conn.Close()
}

// connRegistry maps RemoteAddr -> captureConn, so an HTTP handler can retrieve
// the ClientHello for the current request (tls.Conn hides the underlying conn,
// so we key by remote address which is unique per connection). It also holds the
// captured HTTP/2 fingerprint per address (SoT-02 §HTTP/2).
type connRegistry struct {
	mu       sync.Mutex
	m        map[string]*captureConn
	h2       map[string]*network.H2Fingerprint
	abuse    map[string]string   // remoteAddr -> HTTP/2 DoS signal id (SoT-17)
	hdrOrder map[string][]string // remoteAddr -> on-wire request header names (SoT-02 header order)
}

func newConnRegistry() *connRegistry {
	return &connRegistry{
		m:        map[string]*captureConn{},
		h2:       map[string]*network.H2Fingerprint{},
		abuse:    map[string]string{},
		hdrOrder: map[string][]string{},
	}
}

// SetHeaderOrder records the on-wire request header-name order for a remote address,
// captured before Go's net/http map destroys it (h1 raw peek / h2 HEADERS frame).
// First-write-wins: the first request on the connection pins the order.
func (r *connRegistry) SetHeaderOrder(addr string, order []string) {
	if len(order) == 0 {
		return
	}
	r.mu.Lock()
	if _, ok := r.hdrOrder[addr]; !ok {
		r.hdrOrder[addr] = order
	}
	r.mu.Unlock()
}

// HeaderOrder returns the captured on-wire header-name order for an address (nil if none).
func (r *connRegistry) HeaderOrder(addr string) []string {
	r.mu.Lock()
	o := r.hdrOrder[addr]
	r.mu.Unlock()
	return o
}

// SetAbuse flags a remote address as an HTTP/2 DoS source.
func (r *connRegistry) SetAbuse(addr, sigID string) {
	r.mu.Lock()
	r.abuse[addr] = sigID
	r.mu.Unlock()
}

// Abuse returns the HTTP/2 DoS signal id flagged for an address ("" if none).
func (r *connRegistry) Abuse(addr string) string {
	r.mu.Lock()
	s := r.abuse[addr]
	r.mu.Unlock()
	return s
}

// SetH2 stores the HTTP/2 fingerprint for a remote address.
func (r *connRegistry) SetH2(addr string, fp *network.H2Fingerprint) {
	r.mu.Lock()
	r.h2[addr] = fp
	r.mu.Unlock()
}

// H2 returns the HTTP/2 fingerprint for a remote address (nil if none).
func (r *connRegistry) H2(addr string) *network.H2Fingerprint {
	r.mu.Lock()
	fp := r.h2[addr]
	r.mu.Unlock()
	return fp
}

func (r *connRegistry) add(c *captureConn) {
	r.mu.Lock()
	r.m[c.RemoteAddr().String()] = c
	r.mu.Unlock()
}

func (r *connRegistry) remove(addr string) {
	r.mu.Lock()
	// Clear every per-connection map keyed by this remote address (SoT-31 R5).
	// Previously only r.m was deleted, so r.h2 and r.abuse grew unbounded on a
	// long-running engine.
	delete(r.m, addr)
	delete(r.h2, addr)
	delete(r.abuse, addr)
	delete(r.hdrOrder, addr)
	r.mu.Unlock()
}

// Hello returns the captured ClientHello for a remote address, if any.
func (r *connRegistry) Hello(addr string) *network.ClientHello {
	r.mu.Lock()
	c := r.m[addr]
	r.mu.Unlock()
	if c == nil {
		return nil
	}
	return c.Hello()
}

// captureListener wraps a net.Listener to return captureConns registered in reg.
type captureListener struct {
	net.Listener
	reg *connRegistry
}

func (l *captureListener) Accept() (net.Conn, error) {
	c, err := l.Listener.Accept()
	if err != nil {
		return nil, err
	}
	cc := &captureConn{Conn: c, reg: l.reg}
	l.reg.add(cc)
	return cc, nil
}
