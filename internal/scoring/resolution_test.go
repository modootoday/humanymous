package scoring

import (
	"os"
	"regexp"
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// resolution_test.go is the PLAN-07 R9 anti-typo net. The rule predicates in
// hardrules.go and the FP-mitigation checks in engine.go reference signals and
// cross-checks by STRING ID. A mistyped id is not a compile error — it silently
// makes a rule DEAD (it can never match, because no producer emits that exact id)
// with a green build and a green test suite. These tests turn that class of
// silent failure into a loud one: every id a rule reaches for must resolve in the
// registry / cross-check table, and the rule table itself must be well-formed.
//
// The tests scan the SOURCE (not the runtime), because the predicates are closures
// whose id references cannot be enumerated by reflection. Go runs package tests with
// the package directory as the working directory, so the file names are relative.

var (
	// signalIDLit matches a quoted signal id: lN.group.item (item may contain dots).
	signalIDLit = regexp.MustCompile(`"(l[1-7]\.[a-z0-9_]+\.[a-z0-9_.]+)"`)
	// crossIDLit matches a quoted cross-check id: x.name.
	crossIDLit = regexp.MustCompile(`"(x\.[a-z0-9_]+)"`)
)

// ruleSourceFiles are the files whose id references are load-bearing for a verdict.
var ruleSourceFiles = []string{"hardrules.go", "engine.go"}

// TestRuleSignalIDsResolve fails if any signal id referenced by a rule predicate or
// FP-mitigation check is not a registered signal — the exact typo-makes-a-dead-rule
// hazard R9 exists to catch.
func TestRuleSignalIDsResolve(t *testing.T) {
	seen := map[string]bool{}
	for _, f := range ruleSourceFiles {
		src, err := os.ReadFile(f)
		if err != nil {
			t.Fatalf("read %s: %v", f, err)
		}
		for _, m := range signalIDLit.FindAllStringSubmatch(string(src), -1) {
			id := m[1]
			if seen[id] {
				continue
			}
			seen[id] = true
			if _, ok := signals.Lookup(id); !ok {
				t.Errorf("%s references signal id %q which is NOT registered — "+
					"a typo here silently disables the rule (PLAN-07 R9). "+
					"Register it in internal/signals/registry.go or fix the id.", f, id)
			}
		}
	}
	if len(seen) == 0 {
		t.Fatalf("scanned %v but found no signal-id literals — the scan regex is broken, "+
			"which would make this net pass vacuously", ruleSourceFiles)
	}
	t.Logf("resolved %d distinct rule-referenced signal ids", len(seen))
}

// TestRuleCrossCheckIDsResolve fails if any cross-check id referenced by a rule
// predicate is not defined in the L6 cross-check weight table — same dead-rule
// hazard, different table.
func TestRuleCrossCheckIDsResolve(t *testing.T) {
	seen := map[string]bool{}
	for _, f := range ruleSourceFiles {
		src, err := os.ReadFile(f)
		if err != nil {
			t.Fatalf("read %s: %v", f, err)
		}
		for _, m := range crossIDLit.FindAllStringSubmatch(string(src), -1) {
			id := m[1]
			if seen[id] {
				continue
			}
			seen[id] = true
			if _, ok := crossWeights[id]; !ok {
				t.Errorf("%s references cross-check id %q which is NOT in crossWeights "+
					"(internal/scoring/crosscheck.go) — a typo here silently disables the rule "+
					"(PLAN-07 R9).", f, id)
			}
		}
	}
	if len(seen) == 0 {
		t.Fatalf("found no cross-check-id literals in %v — the scan regex is broken", ruleSourceFiles)
	}
	t.Logf("resolved %d distinct rule-referenced cross-check ids", len(seen))
}

// TestPromotionRulesWellFormed guards the rule table's structural invariants: every
// rule carries a non-empty id, a real verdict, a predicate, and a rationale, and no
// two rules share an id (a duplicate id makes trace attribution ambiguous and usually
// signals a copy-paste bug). It does NOT assert any ordering between verdict classes —
// first-match-wins ordering is verified by the behavioral hard-rule tests, and the
// table intentionally lets a CHALLENGE precede DENYs (SoT-05 §4.1).
func TestPromotionRulesWellFormed(t *testing.T) {
	ids := map[string]bool{}
	valid := map[string]bool{VerdictDeny: true, VerdictChallenge: true, VerdictAllow: true}
	for i, r := range promotionRules {
		if r.id == "" {
			t.Errorf("promotionRules[%d] has an empty id", i)
		}
		if ids[r.id] {
			t.Errorf("promotionRules[%d] duplicate rule id %q", i, r.id)
		}
		ids[r.id] = true
		if !valid[r.verdict] {
			t.Errorf("rule %q has non-verdict %q", r.id, r.verdict)
		}
		if r.pred == nil {
			t.Errorf("rule %q has a nil predicate (can never fire)", r.id)
		}
		if r.why == "" {
			t.Errorf("rule %q has no rationale (why) — the observatory teaching layer needs it", r.id)
		}
	}
	if len(ids) == 0 {
		t.Fatal("promotionRules is empty")
	}
	t.Logf("%d well-formed promotion rules", len(ids))
}
