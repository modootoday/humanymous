package gate

import (
	"io"
	"net"
	"testing"
	"time"
)

// proxyproto_test.go verifies PROXY v2 parsing (correct IP recovery, payload left
// intact) and the anti-spoof trust gate (only allowlisted peers may declare a real IP).

func mkV2IPv4(src net.IP, sport int, dst net.IP, dport int) []byte {
	b := append([]byte{}, proxyV2Sig...)
	b = append(b, 0x21, 0x11, 0x00, 0x0C) // PROXY | TCPv4, addr block length 12
	b = append(b, src.To4()...)
	b = append(b, dst.To4()...)
	b = append(b, byte(sport>>8), byte(sport), byte(dport>>8), byte(dport))
	return b
}

func TestReadProxyV2RecoversIPAndKeepsPayload(t *testing.T) {
	cli, srv := net.Pipe()
	defer cli.Close()
	defer srv.Close()
	hdr := mkV2IPv4(net.IPv4(203, 0, 113, 9), 44321, net.IPv4(10, 0, 0, 1), 443)
	go func() {
		cli.Write(hdr)
		cli.Write([]byte("TLS-CLIENTHELLO")) // the real payload that must survive
	}()
	addr, err := readProxyV2(srv, 2*time.Second)
	if err != nil {
		t.Fatalf("readProxyV2: %v", err)
	}
	if addr == nil || addr.String() != "203.0.113.9:44321" {
		t.Fatalf("recovered addr = %v, want 203.0.113.9:44321", addr)
	}
	buf := make([]byte, len("TLS-CLIENTHELLO"))
	_ = srv.SetReadDeadline(time.Now().Add(2 * time.Second))
	if _, err := io.ReadFull(srv, buf); err != nil {
		t.Fatalf("payload read: %v", err)
	}
	if string(buf) != "TLS-CLIENTHELLO" {
		t.Errorf("payload corrupted: got %q — the header consumed too many/few bytes", buf)
	}
}

func TestReadProxyV2LocalCommand(t *testing.T) {
	cli, srv := net.Pipe()
	defer cli.Close()
	defer srv.Close()
	// LOCAL command (health check): signature, ver/cmd 0x20, family 0x00, len 0.
	hdr := append(append([]byte{}, proxyV2Sig...), 0x20, 0x00, 0x00, 0x00)
	go func() { cli.Write(hdr) }()
	addr, err := readProxyV2(srv, 2*time.Second)
	if err != nil || addr != nil {
		t.Fatalf("LOCAL command should yield (nil,nil), got (%v,%v)", addr, err)
	}
}

func TestReadProxyV2RejectsGarbage(t *testing.T) {
	cli, srv := net.Pipe()
	defer cli.Close()
	defer srv.Close()
	go func() { cli.Write([]byte("GET / HTTP/1.1\r\nHost: x\r\n\r\n")) }()
	if _, err := readProxyV2(srv, 2*time.Second); err == nil {
		t.Error("non-PROXY bytes must be rejected, not parsed as a header")
	}
}

func TestProxyListenerTrustGate(t *testing.T) {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer ln.Close()

	// Case A: peer IS trusted (127.0.0.0/8) → the declared IP is recovered.
	_, trusted, _ := net.ParseCIDR("127.0.0.0/8")
	pl := WrapProxyProto(ln, []*net.IPNet{trusted})
	hdr := mkV2IPv4(net.IPv4(198, 51, 100, 23), 5555, net.IPv4(10, 0, 0, 1), 443)
	go func() {
		c, _ := net.Dial("tcp", ln.Addr().String())
		c.Write(hdr)
		c.Write([]byte("payload"))
	}()
	conn, err := pl.Accept()
	if err != nil {
		t.Fatal(err)
	}
	if conn.RemoteAddr().String() != "198.51.100.23:5555" {
		t.Errorf("trusted proxy: RemoteAddr = %s, want 198.51.100.23:5555", conn.RemoteAddr())
	}
	conn.Close()

	// Case B: peer is NOT in the allowlist → the header is NOT honored; the real peer
	// IP stands and the "header" bytes are delivered as ordinary payload (anti-spoof).
	_, other, _ := net.ParseCIDR("10.0.0.0/8")
	pl2 := WrapProxyProto(ln, []*net.IPNet{other})
	go func() {
		c, _ := net.Dial("tcp", ln.Addr().String())
		c.Write(hdr)
	}()
	conn2, err := pl2.Accept()
	if err != nil {
		t.Fatal(err)
	}
	defer conn2.Close()
	host, _, _ := net.SplitHostPort(conn2.RemoteAddr().String())
	if host != "127.0.0.1" {
		t.Errorf("untrusted peer: RemoteAddr host = %s, want the real peer 127.0.0.1 (spoof rejected)", host)
	}
}

// PLAN-08 backlog: a dangerously broad trusted-proxy CIDR must be rejected (fail-closed
// on misconfig) so an operator cannot let any client spoof any source IP.
func TestParseCIDRsRejectsBroad(t *testing.T) {
	for _, broad := range []string{"0.0.0.0/0", "::/0", "10.0.0.0/4"} {
		if _, err := ParseCIDRs(broad); err == nil {
			t.Errorf("broad CIDR %q must be rejected", broad)
		}
	}
	// A normal balancer CIDR is accepted.
	if nets, err := ParseCIDRs("172.40.0.0/24, 10.1.2.3"); err != nil || len(nets) != 2 {
		t.Errorf("valid CIDRs must parse: %v, %d nets", err, len(nets))
	}
}
