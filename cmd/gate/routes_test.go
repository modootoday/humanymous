package main

import (
	"os"
	"path/filepath"
	"testing"
)

func writeTemp(t *testing.T, body string) string {
	t.Helper()
	p := filepath.Join(t.TempDir(), "routes.conf")
	if err := os.WriteFile(p, []byte(body), 0o600); err != nil {
		t.Fatal(err)
	}
	return p
}

func TestLoadRoutesValid(t *testing.T) {
	p := writeTemp(t, "# my routes\n/login    strict\n/checkout strict\n\n/health   off\n/api      balanced\n")
	r, err := loadRoutes(p)
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]string{"/login": "strict", "/checkout": "strict", "/health": "off", "/api": "balanced"}
	if len(r) != len(want) {
		t.Fatalf("got %d routes, want %d: %v", len(r), len(want), r)
	}
	for k, v := range want {
		if r[k] != v {
			t.Errorf("route %q = %q, want %q", k, r[k], v)
		}
	}
}

func TestLoadRoutesUnknownPreset(t *testing.T) {
	p := writeTemp(t, "/login supersecure\n")
	if _, err := loadRoutes(p); err == nil {
		t.Fatal("expected error on unknown preset, got nil")
	}
}

// Ceiling-guard #1: the attestation floor is valid on a specific route but REFUSED on
// the catch-all — flooring an entire site's browse traffic to Pass is a misconfig.
func TestLoadRoutesAttestedValidAndCatchAllRefused(t *testing.T) {
	if r, err := loadRoutes(writeTemp(t, "/transfer attested\n/checkout attested\n")); err != nil || r["/transfer"] != "attested" {
		t.Fatalf("attested on a specific route must load: r=%v err=%v", r, err)
	}
	if _, err := loadRoutes(writeTemp(t, "/ attested\n")); err == nil {
		t.Fatal("expected error: attested on the catch-all prefix must be refused")
	}
}

func TestLoadRoutesMalformed(t *testing.T) {
	p := writeTemp(t, "/login strict extra\n")
	if _, err := loadRoutes(p); err == nil {
		t.Fatal("expected error on malformed line, got nil")
	}
}

func TestLoadRoutesMissingFile(t *testing.T) {
	if _, err := loadRoutes(filepath.Join(t.TempDir(), "nope.conf")); err == nil {
		t.Fatal("expected error on missing file, got nil")
	}
}
