package scoring

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

// freezeGolden is the on-disk freeze corpus entry (SoT-38 §9.1 / v0.2.2).
// Captures today's Score() outputs for fixed SessionReports so a silent score
// move fails CI unless the fixture is updated deliberately (with a ! commit).
type freezeGolden struct {
	ID     string                `json:"id"`
	Report signals.SessionReport `json:"report"`
	Want   freezeWant            `json:"want"`
}

type freezeWant struct {
	RiskScore     float64 `json:"riskScore"`
	Verdict       string  `json:"verdict"`
	HardRuleFired string  `json:"hardRuleFired,omitempty"`
	PolicyVersion string  `json:"policyVersion"`
}

func freezeFixtureDir(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("runtime.Caller failed")
	}
	// internal/scoring -> repo root -> test/fixtures/scoring
	root := filepath.Clean(filepath.Join(filepath.Dir(file), "..", ".."))
	return filepath.Join(root, "test", "fixtures", "scoring")
}

// freezeReports is the authoritative input set for the freeze corpus.
// Add cases here, then: UPDATE_SCORING_GOLDEN=1 go test ./internal/scoring -run TestFreezeGoldenCorpus
func freezeReports() map[string]*signals.SessionReport {
	return map[string]*signals.SessionReport{
		"human-chrome-allow": {
			Client: signals.ClientReport{
				UserAgent:     "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
				UAClientHints: signals.UAClientHints{Platform: "Windows", Brands: []signals.UABrand{{Brand: "Chromium", Version: "126"}}},
				Signals: []signals.Signal{
					sig("l1.navigator.webdriver", signals.VerdictOK, 1),
				},
				Environment: signals.Environment{Probed: true, AdBlock: true},
			},
			Network: signals.NetworkReport{
				JA4Engine: "chrome", H2Engine: "chrome",
				SecFetchPresent: true, SecCHUAPresent: true,
			},
		},
		"selenium-hr1": {
			Client: signals.ClientReport{
				UserAgent: "Mozilla/5.0 Chrome/126.0",
				Signals: []signals.Signal{
					sig("l1.artifact.selenium", signals.VerdictBot, 1),
					sig("l1.navigator.webdriver", signals.VerdictBot, 1),
				},
			},
			Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome", SecFetchPresent: true, SecCHUAPresent: true},
		},
		"ua-tls-h2-hr2": {
			Client: signals.ClientReport{
				UserAgent: "Mozilla/5.0 (Windows NT 10.0) Chrome/126.0 Safari/537.36",
				Signals:   []signals.Signal{},
			},
			Network: signals.NetworkReport{JA4Engine: "go", H2Engine: "go", SecFetchPresent: false, SecCHUAPresent: false},
		},
		"no-client-hr10": {
			Client:  signals.ClientReport{},
			Network: signals.NetworkReport{JA4Engine: "chrome", H2Engine: "chrome"},
		},
		"bot-webdriver-cdp-hr9": {
			Client: signals.ClientReport{
				UserAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/126.0.0.0 Safari/537.36",
				Signals: []signals.Signal{
					{ID: "l1.navigator.webdriver", Layer: "L1", Verdict: signals.VerdictBot, Weight: 40, Score: 40, Confidence: 0.95, Collected: "js"},
					{ID: "l1.cdp.proxy_leak", Layer: "L1", Verdict: signals.VerdictBot, Weight: 40, Score: 40, Confidence: 0.95, Collected: "wasm"},
				},
			},
		},
		"go-http-client": {
			Client:  signals.ClientReport{UserAgent: "Go-http-client/2.0"},
			Network: signals.NetworkReport{JA4Engine: "go", H2Engine: "go"},
		},
	}
}

func TestFreezeGoldenCorpus(t *testing.T) {
	dir := freezeFixtureDir(t)
	update := os.Getenv("UPDATE_SCORING_GOLDEN") == "1"
	if update {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatalf("mkdir fixtures: %v", err)
		}
	}

	e := NewEngine()
	for id, report := range freezeReports() {
		got := e.Score(report)
		wantSnap := freezeWant{
			RiskScore:     got.RiskScore,
			Verdict:       got.Verdict,
			HardRuleFired: got.HardRuleFired,
			PolicyVersion: got.PolicyVersion,
		}
		path := filepath.Join(dir, id+".json")
		entry := freezeGolden{ID: id, Report: *report, Want: wantSnap}

		if update {
			raw, err := json.MarshalIndent(entry, "", "  ")
			if err != nil {
				t.Fatalf("%s: marshal: %v", id, err)
			}
			raw = append(raw, '\n')
			if err := os.WriteFile(path, raw, 0o644); err != nil {
				t.Fatalf("%s: write: %v", id, err)
			}
			t.Logf("updated %s", path)
			continue
		}

		raw, err := os.ReadFile(path)
		if err != nil {
			t.Fatalf("%s: missing fixture %s — run UPDATE_SCORING_GOLDEN=1 go test ./internal/scoring -run TestFreezeGoldenCorpus", id, path)
		}
		var disk freezeGolden
		if err := json.Unmarshal(raw, &disk); err != nil {
			t.Fatalf("%s: parse fixture: %v", id, err)
		}
		// Re-score the *disk* report so the fixture is the freeze contract.
		scored := e.Score(&disk.Report)
		if scored.Verdict != disk.Want.Verdict ||
			scored.HardRuleFired != disk.Want.HardRuleFired ||
			scored.PolicyVersion != disk.Want.PolicyVersion ||
			scored.RiskScore != disk.Want.RiskScore {
			t.Errorf("%s: score moved without golden update\n got verdict=%s rule=%q risk=%v policy=%s\nwant verdict=%s rule=%q risk=%v policy=%s\n(re-run with UPDATE_SCORING_GOLDEN=1 only with a deliberate !/BREAKING change)",
				id,
				scored.Verdict, scored.HardRuleFired, scored.RiskScore, scored.PolicyVersion,
				disk.Want.Verdict, disk.Want.HardRuleFired, disk.Want.RiskScore, disk.Want.PolicyVersion,
			)
		}
	}

	// Every on-disk fixture must still be in freezeReports (no orphan goldens).
	if !update {
		ents, err := os.ReadDir(dir)
		if err != nil {
			t.Fatalf("read fixtures dir: %v", err)
		}
		known := freezeReports()
		for _, ent := range ents {
			if ent.IsDir() || filepath.Ext(ent.Name()) != ".json" {
				continue
			}
			id := ent.Name()[:len(ent.Name())-len(".json")]
			if _, ok := known[id]; !ok {
				t.Errorf("orphan fixture %s — add to freezeReports() or delete", ent.Name())
			}
		}
	}
}
