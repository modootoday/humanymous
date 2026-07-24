package main

import (
	"crypto/tls"
	"strings"

	"golang.org/x/crypto/acme"
	"golang.org/x/crypto/acme/autocert"
)

// tls.go selects the public edge listener's certificate source (SoT-31 R1). The
// engine (cmd/server/acme.go) established this pattern; the reverse proxy needs
// the same so an adopter can present a browser-trusted certificate instead of the
// dev self-signed one. The admin listener stays self-signed (management plane —
// real control is a management network + mTLS, per the operability audit).

type edgeTLS struct {
	acmeDomains   []string // -acme-domain (csv); autocert TLS-ALPN-01 on the edge
	acmeCache     string   // -acme-cache DirCache path
	acmeEmail     string   // -acme-email account contact
	acmeDirectory string   // -acme-directory ACME server URL; empty = production Let's Encrypt
	certFile      string   // -tls-cert (bring-your-own)
	keyFile       string   // -tls-key
}

// buildEdgeTLS returns the *tls.Config for the public edge:
//   - ACME domains  -> autocert TLS-ALPN-01 (adds acme-tls/1 to NextProtos)
//   - cert + key    -> bring-your-own certificate
//   - neither       -> self-signed (dev, unchanged)
func buildEdgeTLS(e edgeTLS) (*tls.Config, error) {
	switch {
	case len(e.acmeDomains) > 0:
		cache := e.acmeCache
		if cache == "" {
			cache = "acme-cache"
		}
		m := &autocert.Manager{
			Prompt:     autocert.AcceptTOS,
			HostPolicy: autocert.HostWhitelist(e.acmeDomains...),
			Cache:      autocert.DirCache(cache),
			Email:      e.acmeEmail,
		}
		// A custom ACME directory (empty = production Let's Encrypt). Point this at the
		// Let's Encrypt STAGING directory to dry-run issuance without burning the strict
		// production rate limits, or at ZeroSSL / an internal step-ca. Staging certs are
		// signed by an untrusted root (browsers warn) — for validation only.
		if e.acmeDirectory != "" {
			m.Client = &acme.Client{DirectoryURL: e.acmeDirectory}
		}
		return &tls.Config{
			GetCertificate: m.GetCertificate,
			MinVersion:     tls.VersionTLS12,
			NextProtos:     []string{"h2", "http/1.1", acme.ALPNProto},
		}, nil
	case e.certFile != "" && e.keyFile != "":
		cert, err := tls.LoadX509KeyPair(e.certFile, e.keyFile)
		if err != nil {
			return nil, err
		}
		return &tls.Config{Certificates: []tls.Certificate{cert}, MinVersion: tls.VersionTLS12}, nil
	default:
		cert, err := selfSignedCert()
		if err != nil {
			return nil, err
		}
		return &tls.Config{Certificates: []tls.Certificate{cert}, MinVersion: tls.VersionTLS12}, nil
	}
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
