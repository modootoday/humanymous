package main

import (
	"testing"

	"golang.org/x/crypto/acme"
)

// No cert source -> self-signed, no ACME ALPN (unchanged dev behavior).
func TestBuildEdgeTLSSelfSigned(t *testing.T) {
	cfg, err := buildEdgeTLS(edgeTLS{})
	if err != nil {
		t.Fatal(err)
	}
	if len(cfg.Certificates) == 0 {
		t.Fatal("self-signed mode must set a static certificate")
	}
	if cfg.GetCertificate != nil {
		t.Error("self-signed mode must not set GetCertificate")
	}
	for _, p := range cfg.NextProtos {
		if p == acme.ALPNProto {
			t.Errorf("self-signed mode must not advertise %q", acme.ALPNProto)
		}
	}
}

// ACME domains -> autocert GetCertificate + acme-tls/1 ALPN so TLS-ALPN-01 works.
func TestBuildEdgeTLSACME(t *testing.T) {
	cfg, err := buildEdgeTLS(edgeTLS{acmeDomains: []string{"edge.example.com"}, acmeCache: t.TempDir()})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.GetCertificate == nil {
		t.Fatal("ACME mode must set GetCertificate")
	}
	if len(cfg.Certificates) != 0 {
		t.Error("ACME mode must not carry a static cert")
	}
	var hasALPN bool
	for _, p := range cfg.NextProtos {
		if p == acme.ALPNProto {
			hasALPN = true
		}
	}
	if !hasALPN {
		t.Errorf("ACME mode must advertise %q, got %v", acme.ALPNProto, cfg.NextProtos)
	}
}

// A custom ACME directory (e.g. Let's Encrypt staging) still yields a valid ACME
// TLS config — GetCertificate set, acme-tls/1 advertised — so staging dry-runs work.
func TestBuildEdgeTLSACMEStagingDirectory(t *testing.T) {
	cfg, err := buildEdgeTLS(edgeTLS{
		acmeDomains:   []string{"edge.example.com"},
		acmeCache:     t.TempDir(),
		acmeDirectory: "https://acme-staging-v02.api.letsencrypt.org/directory",
	})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.GetCertificate == nil {
		t.Fatal("ACME mode with a custom directory must still set GetCertificate")
	}
	var hasALPN bool
	for _, p := range cfg.NextProtos {
		if p == acme.ALPNProto {
			hasALPN = true
		}
	}
	if !hasALPN {
		t.Errorf("ACME mode must advertise %q, got %v", acme.ALPNProto, cfg.NextProtos)
	}
}

// A BYO cert path that does not exist is a hard error (not a silent self-sign).
func TestBuildEdgeTLSBadCert(t *testing.T) {
	_, err := buildEdgeTLS(edgeTLS{certFile: "/no/such/cert.pem", keyFile: "/no/such/key.pem"})
	if err == nil {
		t.Fatal("expected error loading a missing BYO cert, got nil")
	}
}

func TestSplitDomains(t *testing.T) {
	got := splitDomains(" a.com , , b.com ")
	if len(got) != 2 || got[0] != "a.com" || got[1] != "b.com" {
		t.Fatalf("splitDomains = %v, want [a.com b.com]", got)
	}
	if len(splitDomains("")) != 0 {
		t.Error("empty csv must yield no domains")
	}
}
