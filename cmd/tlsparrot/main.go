// Command tlsparrot is a Red-team TLS-parrot client (SoT-04): it connects with a
// real Chrome ClientHello via uTLS, so the server's JA4 resolves to "chrome"
// (the TLS fingerprint is faithfully spoofed), then POSTs /api/collect with a
// Chrome UA. It has no JS/WASM runtime, so it cannot produce a client report —
// the Blue engine must still block it (HR-10). It prints the verdict JSON.
//
// Local target only. Demonstrates the residual that even a perfect TLS parrot
// leaks: the absent JS/DOM layer (SoT-02 §8, SoT-06 threat model).
package main

import (
	"bufio"
	"flag"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"strings"

	utls "github.com/refraction-networking/utls"
)

func main() {
	target := flag.String("url", "https://127.0.0.1:8443/api/collect", "collect endpoint")
	flag.Parse()

	u, err := url.Parse(*target)
	if err != nil {
		fail(err)
	}
	host := u.Host
	if !strings.Contains(host, ":") {
		host += ":443"
	}

	raw, err := net.Dial("tcp", host)
	if err != nil {
		fail(err)
	}
	defer raw.Close()

	// Force http/1.1 ALPN so we can speak HTTP/1.1 over the parrot while keeping
	// Chrome's cipher/extension fingerprint (JA4 still resolves to chrome).
	spec, err := utls.UTLSIdToSpec(utls.HelloChrome_Auto)
	if err != nil {
		fail(err)
	}
	forceHTTP11(&spec)

	cfg := &utls.Config{ServerName: hostname(host), InsecureSkipVerify: true}
	uconn := utls.UClient(raw, cfg, utls.HelloCustom)
	if err := uconn.ApplyPreset(&spec); err != nil {
		fail(err)
	}
	if err := uconn.Handshake(); err != nil {
		fail(err)
	}

	// A minimal Chrome-looking request with no real client signals.
	body := `{"userAgent":"Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36","signals":[]}`
	req := strings.Join([]string{
		"POST " + u.RequestURI() + "?label=bot:tls-parrot HTTP/1.1",
		"Host: " + hostname(host),
		"User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
		"Content-Type: application/json",
		"Accept-Language: en-US,en;q=0.9",
		fmt.Sprintf("Content-Length: %d", len(body)),
		"Connection: close",
		"", body,
	}, "\r\n")
	if _, err := io.WriteString(uconn, req); err != nil {
		fail(err)
	}

	resp, err := http.ReadResponse(bufio.NewReader(uconn), nil)
	if err != nil {
		fail(err)
	}
	defer resp.Body.Close()
	out, _ := io.ReadAll(resp.Body)
	fmt.Println(string(out))
}

// forceHTTP11 rewrites the ALPN extension to offer only http/1.1.
func forceHTTP11(spec *utls.ClientHelloSpec) {
	for _, ext := range spec.Extensions {
		if alpn, ok := ext.(*utls.ALPNExtension); ok {
			alpn.AlpnProtocols = []string{"http/1.1"}
		}
	}
}

func hostname(hostport string) string {
	if h, _, err := net.SplitHostPort(hostport); err == nil {
		return h
	}
	return hostport
}

func fail(err error) {
	fmt.Fprintln(os.Stderr, "tlsparrot:", err)
	os.Exit(1)
}
