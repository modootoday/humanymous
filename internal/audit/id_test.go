package audit

import (
	"regexp"
	"testing"
	"time"
)

// id_test.go locks the PLAN-07 R15 event-id + timestamp stamping.

var uuidV7Re = regexp.MustCompile(`^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`)

func TestNewEventIDIsUUIDv7(t *testing.T) {
	id := newEventID(time.Unix(1_700_000_000, 0))
	if !uuidV7Re.MatchString(id) {
		t.Fatalf("event id %q is not a well-formed UUIDv7 (version 7 + variant 10)", id)
	}
}

func TestNewEventIDUnique(t *testing.T) {
	seen := map[string]bool{}
	at := time.Unix(1_700_000_000, 0) // same instant every call → collisions must come from randomness only
	for i := 0; i < 10000; i++ {
		id := newEventID(at)
		if seen[id] {
			t.Fatalf("duplicate event id at i=%d: %s", i, id)
		}
		seen[id] = true
	}
}

func TestAppendStampsIDAndTS(t *testing.T) {
	l := NewLog(Config{NodeID: "n1", HMACKey: []byte("k")})
	r := l.Append(Record{EventType: EventEnfDeny})
	if !uuidV7Re.MatchString(r.EventID) {
		t.Errorf("Append did not stamp a UUIDv7 event id, got %q", r.EventID)
	}
	if r.TS == "" {
		t.Error("Append did not stamp a timestamp")
	}
	if _, err := time.Parse(time.RFC3339Nano, r.TS); err != nil {
		t.Errorf("stamped TS %q is not RFC3339Nano: %v", r.TS, err)
	}
	// A caller-supplied id must be preserved (WAL replay path relies on this).
	pre := l.Append(Record{EventType: EventEnfAllow, EventID: "preset-id", TS: "preset-ts"})
	if pre.EventID != "preset-id" || pre.TS != "preset-ts" {
		t.Errorf("Append overwrote caller-set id/ts: got %q/%q", pre.EventID, pre.TS)
	}
}
