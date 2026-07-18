package main

import (
	"encoding/hex"
	"testing"

	"github.com/modootoday/humanymous/internal/network"
)

// randHex returns a well-formed hex id of the right length (SoT-31 R4). The
// error path panics rather than returning a predictable value; here we assert the
// happy path yields non-empty, decodable output.
func TestRandHex(t *testing.T) {
	s := randHex(16)
	if len(s) != 32 {
		t.Fatalf("randHex(16) length = %d, want 32", len(s))
	}
	if _, err := hex.DecodeString(s); err != nil {
		t.Fatalf("randHex not valid hex: %v", err)
	}
	if randHex(16) == s {
		t.Error("two randHex calls returned identical values")
	}
}

// connRegistry.remove clears every per-connection map (SoT-31 R5): previously
// h2/abuse leaked because only m was deleted.
func TestConnRegistryRemoveClearsAllMaps(t *testing.T) {
	r := newConnRegistry()
	const addr = "203.0.113.7:44321"
	r.SetH2(addr, &network.H2Fingerprint{})
	r.SetAbuse(addr, "l5.h2dos.rapid_reset")

	r.remove(addr)

	if r.H2(addr) != nil {
		t.Error("h2 fingerprint not cleared on remove")
	}
	if r.Abuse(addr) != "" {
		t.Error("abuse flag not cleared on remove")
	}
}
