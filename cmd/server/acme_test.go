package main

import (
	"crypto/ecdsa"
	"crypto/x509"
	"encoding/pem"
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"golang.org/x/crypto/acme"
)

func TestSplitDomains(t *testing.T) {
	cases := map[string][]string{
		"":                    nil,
		"example.com":         {"example.com"},
		"a.com, b.com":        {"a.com", "b.com"},
		"  a.com ,, b.com , ": {"a.com", "b.com"},
		"demo.humanymous.dev": {"demo.humanymous.dev"},
	}
	for in, want := range cases {
		if got := splitDomains(in); !reflect.DeepEqual(got, want) {
			t.Errorf("splitDomains(%q) = %v, want %v", in, got, want)
		}
	}
}

// With no ACME domain the listener uses a self-signed cert and advertises only
// the normal protocols — unchanged local/dev behavior.
func TestBuildTLSConfigSelfSigned(t *testing.T) {
	cfg, err := buildTLSConfig(tlsSettings{})
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

// With a domain, autocert drives certs via GetCertificate and the socket must
// advertise the acme-tls/1 ALPN so TLS-ALPN-01 challenges can be answered inline.
func TestBuildTLSConfigACME(t *testing.T) {
	cfg, err := buildTLSConfig(tlsSettings{acmeDomains: []string{"demo.example.com"}, acmeCache: t.TempDir()})
	if err != nil {
		t.Fatal(err)
	}
	if cfg.GetCertificate == nil {
		t.Fatal("ACME mode must set GetCertificate")
	}
	if len(cfg.Certificates) != 0 {
		t.Error("ACME mode must not carry a static self-signed cert")
	}
	var hasALPN bool
	for _, p := range cfg.NextProtos {
		if p == acme.ALPNProto {
			hasALPN = true
		}
	}
	if !hasALPN {
		t.Errorf("ACME mode must advertise %q in NextProtos, got %v", acme.ALPNProto, cfg.NextProtos)
	}
}

func TestBuildTLSConfigProvidedKeyPair(t *testing.T) {
	generated, err := selfSignedCert()
	if err != nil {
		t.Fatal(err)
	}
	key, ok := generated.PrivateKey.(*ecdsa.PrivateKey)
	if !ok {
		t.Fatalf("private key type = %T, want ECDSA", generated.PrivateKey)
	}
	keyDER, err := x509.MarshalECPrivateKey(key)
	if err != nil {
		t.Fatal(err)
	}
	dir := t.TempDir()
	certFile := filepath.Join(dir, "server.pem")
	keyFile := filepath.Join(dir, "server-key.pem")
	if err := os.WriteFile(certFile, pem.EncodeToMemory(&pem.Block{
		Type:  "CERTIFICATE",
		Bytes: generated.Certificate[0],
	}), 0o600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(keyFile, pem.EncodeToMemory(&pem.Block{
		Type:  "EC PRIVATE KEY",
		Bytes: keyDER,
	}), 0o600); err != nil {
		t.Fatal(err)
	}

	cfg, err := buildTLSConfig(tlsSettings{certFile: certFile, keyFile: keyFile})
	if err != nil {
		t.Fatal(err)
	}
	if len(cfg.Certificates) != 1 || cfg.GetCertificate != nil {
		t.Fatal("operator-provided mode must configure exactly one static certificate")
	}
}

func TestBuildTLSConfigRejectsAmbiguousSources(t *testing.T) {
	for name, settings := range map[string]tlsSettings{
		"certificate without key": {certFile: "server.pem"},
		"key without certificate": {keyFile: "server-key.pem"},
		"certificate with ACME": {
			certFile:    "server.pem",
			keyFile:     "server-key.pem",
			acmeDomains: []string{"example.test"},
		},
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := buildTLSConfig(settings); err == nil {
				t.Fatal("expected configuration error")
			}
		})
	}
}
