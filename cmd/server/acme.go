package main

import (
	"crypto/tls"
	"strings"

	"golang.org/x/crypto/acme"
	"golang.org/x/crypto/acme/autocert"
)

// acme.go builds the server's TLS config. Because the engine terminates TLS
// itself (to read the raw ClientHello for JA3/JA4 + the HTTP/2 fingerprint), it
// cannot sit behind a TLS-terminating load balancer. On a plain VPS the process
// binds :443 directly and obtains its own Let's Encrypt certificate via the
// TLS-ALPN-01 challenge — handled inline during the handshake, on the same
// socket, so no separate :80 responder or fronting proxy is involved.

// tlsSettings selects the certificate source for the listener.
type tlsSettings struct {
	acmeDomains []string // comma-separated -acme-domain; empty → self-signed
	acmeCache   string   // DirCache path for issued certs + account key
	acmeEmail   string   // optional ACME account contact
}

// buildTLSConfig returns the *tls.Config for the accept loop. With no ACME
// domain it uses a self-signed cert (local/dev, unchanged behavior). With one or
// more domains it drives autocert's TLS-ALPN-01 flow via GetCertificate.
//
// TLS-ALPN-01 requires the ClientHello to negotiate the "acme-tls/1" ALPN
// protocol; autocert.Manager.GetCertificate answers the validation handshake
// when it sees that proto, and serves the real cert otherwise. We therefore add
// acme.ALPNProto to NextProtos alongside h2/http-1.1.
func buildTLSConfig(s tlsSettings) (*tls.Config, error) {
	if len(s.acmeDomains) == 0 {
		cert, err := selfSignedCert()
		if err != nil {
			return nil, err
		}
		return &tls.Config{
			Certificates: []tls.Certificate{cert},
			MinVersion:   tls.VersionTLS12,
			NextProtos:   []string{"h2", "http/1.1"},
		}, nil
	}

	cache := s.acmeCache
	if cache == "" {
		cache = "acme-cache"
	}
	m := &autocert.Manager{
		Prompt:     autocert.AcceptTOS,
		HostPolicy: autocert.HostWhitelist(s.acmeDomains...),
		Cache:      autocert.DirCache(cache),
		Email:      s.acmeEmail,
	}
	return &tls.Config{
		GetCertificate: m.GetCertificate,
		MinVersion:     tls.VersionTLS12,
		// acme.ALPNProto ("acme-tls/1") lets the same socket answer the TLS-ALPN-01
		// challenge; h2/http-1.1 carry normal traffic.
		NextProtos: []string{"h2", "http/1.1", acme.ALPNProto},
	}, nil
}

// splitDomains parses a comma-separated -acme-domain flag, trimming blanks.
func splitDomains(csv string) []string {
	var out []string
	for _, d := range strings.Split(csv, ",") {
		if d = strings.TrimSpace(d); d != "" {
			out = append(out, d)
		}
	}
	return out
}
