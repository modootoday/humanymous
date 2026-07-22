package redis

import (
	"bufio"
	"strings"
	"testing"
)

// client_test.go verifies the RESP2 wire codec: encoding commands and parsing every
// reply type the ledgers rely on (simple string, error, integer, bulk, null bulk,
// and nested arrays as returned by SCAN).

func TestRespCommandEncoding(t *testing.T) {
	got := string(respCommand([]string{"SET", "k", "v"}))
	want := "*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"
	if got != want {
		t.Fatalf("respCommand mismatch:\n got %q\nwant %q", got, want)
	}
}

func parse(t *testing.T, wire string) Reply {
	t.Helper()
	r, err := readReply(bufio.NewReader(strings.NewReader(wire)))
	if err != nil {
		t.Fatalf("readReply(%q): %v", wire, err)
	}
	return r
}

func TestReadReplyScalars(t *testing.T) {
	if r := parse(t, "+OK\r\n"); r.Str != "OK" {
		t.Errorf("simple string: got %q", r.Str)
	}
	if r := parse(t, ":42\r\n"); r.Int != 42 {
		t.Errorf("integer: got %d", r.Int)
	}
	if r := parse(t, "$5\r\nhello\r\n"); r.Str != "hello" {
		t.Errorf("bulk string: got %q", r.Str)
	}
	if r := parse(t, "$-1\r\n"); !r.Nil {
		t.Errorf("null bulk should set Nil")
	}
	// A bulk string that itself contains CRLF must be length-delimited, not line-split.
	if r := parse(t, "$7\r\na\r\nb\r\nc\r\n"); r.Str != "a\r\nb\r\nc" {
		t.Errorf("binary-safe bulk: got %q", r.Str)
	}
}

func TestReadReplyError(t *testing.T) {
	if _, err := readReply(bufio.NewReader(strings.NewReader("-WRONGTYPE nope\r\n"))); err == nil {
		t.Error("RESP error reply must surface as a Go error")
	}
}

func TestReadReplyScanArray(t *testing.T) {
	// SCAN reply: [cursor, [key1, key2]] — the exact nested shape scanBanKeys parses.
	wire := "*2\r\n$1\r\n0\r\n*2\r\n$9\r\nhmn:ban:a\r\n$9\r\nhmn:ban:b\r\n"
	r := parse(t, wire)
	if len(r.Array) != 2 {
		t.Fatalf("expected 2-element array, got %d", len(r.Array))
	}
	if r.Array[0].Str != "0" {
		t.Errorf("cursor: got %q", r.Array[0].Str)
	}
	keys := r.Array[1].Array
	if len(keys) != 2 || keys[0].Str != "hmn:ban:a" || keys[1].Str != "hmn:ban:b" {
		t.Errorf("scan keys: got %+v", keys)
	}
}

// PLAN-08 backlog: an oversized declared length must be rejected, not allocated — a
// compromised coordinator otherwise triggers an OOM/panic via a huge $ or * header.
func TestReadReplyRejectsOversizedLengths(t *testing.T) {
	if _, err := readReply(bufio.NewReader(strings.NewReader("$999999999\r\n"))); err == nil {
		t.Error("oversized bulk length must be rejected")
	}
	if _, err := readReply(bufio.NewReader(strings.NewReader("*999999999\r\n"))); err == nil {
		t.Error("oversized array length must be rejected")
	}
}
