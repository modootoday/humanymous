package mltrain

import (
	"bufio"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

func TestTraceSink_AppendRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "sub", "oracle.jsonl") // sub dir must be auto-created
	s, err := NewTraceSink(path)
	if err != nil {
		t.Fatal(err)
	}
	recs := []Record{
		{Label: 0, TS: 100, Source: "pass", Behavior: signals.BehaviorSummary{DurationS: 8}},
		{Label: 0, TS: 101, Source: "pass", Cohort: "typical"},
	}
	for _, r := range recs {
		if err := s.Append(r); err != nil {
			t.Fatalf("append: %v", err)
		}
	}
	if s.Count() != 2 {
		t.Fatalf("count = %d, want 2", s.Count())
	}
	if err := s.Close(); err != nil {
		t.Fatal(err)
	}

	// the file must parse back as exactly the two records we wrote.
	f, err := os.Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer f.Close()
	var got []Record
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		var r Record
		if err := json.Unmarshal(sc.Bytes(), &r); err != nil {
			t.Fatalf("reparse: %v", err)
		}
		got = append(got, r)
	}
	if len(got) != 2 || got[0].TS != 100 || got[1].Cohort != "typical" {
		t.Fatalf("round-trip mismatch: %+v", got)
	}
	// the round-tripped record must yield a usable training sample.
	if s := got[0].Sample(); s.Human != true || len(s.X) != len(got[0].Sample().X) {
		t.Fatalf("sample conversion off: %+v", s)
	}
}

func TestTraceSink_NilIsNoop(t *testing.T) {
	var s *TraceSink
	if err := s.Append(Record{Label: 0}); err != nil {
		t.Fatalf("nil sink Append must be a no-op, got %v", err)
	}
	if s.Count() != 0 {
		t.Fatal("nil sink Count must be 0")
	}
	if err := s.Close(); err != nil {
		t.Fatalf("nil sink Close must be a no-op, got %v", err)
	}
}

func TestTraceSink_AppendsAcrossReopen(t *testing.T) {
	path := filepath.Join(t.TempDir(), "oracle.jsonl")
	s1, _ := NewTraceSink(path)
	_ = s1.Append(Record{Label: 0, TS: 1})
	s1.Close()
	// reopening must APPEND, not truncate — labels already collected are never lost.
	s2, _ := NewTraceSink(path)
	_ = s2.Append(Record{Label: 0, TS: 2})
	s2.Close()

	data, _ := os.ReadFile(path)
	lines := 0
	for _, b := range data {
		if b == '\n' {
			lines++
		}
	}
	if lines != 2 {
		t.Fatalf("reopen must append: want 2 lines, got %d", lines)
	}
}
