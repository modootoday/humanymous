// Package report aggregates Red vs Blue e2e results into metrics and renders a
// self-contained HTML report (plan/05). It reads the runner's results.json.
package report

import (
	"encoding/json"
	"os"
	"sort"
)

// Record mirrors one entry in test/e2e/results.json.
type Record struct {
	Profile       string   `json:"profile"`
	Label         string   `json:"label"`
	Run           int      `json:"run"`
	Verdict       string   `json:"verdict"`
	RiskScore     float64  `json:"riskScore"`
	HardRuleFired string   `json:"hardRuleFired"`
	Outcome       string   `json:"outcome"` // TP/FP/TN/FN
	Top           []string `json:"top"`
	Skipped       bool     `json:"skipped"`
	Reason        string   `json:"reason"`
	Error         string   `json:"error"`
}

// ProfileSummary aggregates one Red profile's runs.
type ProfileSummary struct {
	Label    string
	IsBot    bool
	Runs     int
	Verdicts map[string]int
	MeanRisk float64
	Rules    map[string]int
	TopHits  map[string]int
	Blocked  int // DENY+CHALLENGE for bots
}

// Summary is the whole-run aggregate.
type Summary struct {
	Profiles  []ProfileSummary
	BotTPR    float64 // blocked bots / total bot runs
	HumanFPR  float64 // denied humans / total human runs
	BotRuns   int
	HumanRuns int
	Skipped   []string
}

// Load reads results.json from path.
func Load(path string) ([]Record, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var recs []Record
	if err := json.Unmarshal(b, &recs); err != nil {
		return nil, err
	}
	return recs, nil
}

// Aggregate computes metrics from the records (SoT/plan-04 §1 definitions).
func Aggregate(recs []Record) Summary {
	byLabel := map[string]*ProfileSummary{}
	var skipped []string
	order := []string{}

	for _, r := range recs {
		if r.Skipped {
			skipped = append(skipped, r.Profile+" ("+r.Reason+")")
			continue
		}
		if r.Label == "" || r.Error != "" {
			continue
		}
		ps, ok := byLabel[r.Label]
		if !ok {
			ps = &ProfileSummary{
				Label:    r.Label,
				IsBot:    isBot(r.Label),
				Verdicts: map[string]int{},
				Rules:    map[string]int{},
				TopHits:  map[string]int{},
			}
			byLabel[r.Label] = ps
			order = append(order, r.Label)
		}
		ps.Runs++
		ps.Verdicts[r.Verdict]++
		ps.MeanRisk += r.RiskScore
		if r.HardRuleFired != "" {
			ps.Rules[r.HardRuleFired]++
		}
		for _, id := range r.Top {
			ps.TopHits[id]++
		}
		if ps.IsBot && (r.Verdict == "DENY" || r.Verdict == "CHALLENGE") {
			ps.Blocked++
		}
	}

	var sum Summary
	sum.Skipped = skipped
	var botBlocked, humanDenied int
	for _, label := range order {
		ps := byLabel[label]
		if ps.Runs > 0 {
			ps.MeanRisk /= float64(ps.Runs)
		}
		if ps.IsBot {
			sum.BotRuns += ps.Runs
			botBlocked += ps.Blocked
		} else {
			sum.HumanRuns += ps.Runs
			humanDenied += ps.Verdicts["DENY"]
		}
		sum.Profiles = append(sum.Profiles, *ps)
	}
	if sum.BotRuns > 0 {
		sum.BotTPR = float64(botBlocked) / float64(sum.BotRuns)
	}
	if sum.HumanRuns > 0 {
		sum.HumanFPR = float64(humanDenied) / float64(sum.HumanRuns)
	}
	return sum
}

func isBot(label string) bool {
	return len(label) >= 4 && label[:4] == "bot:"
}

// SortedTopHits returns a profile's top signal hits, most frequent first.
func (p ProfileSummary) SortedTopHits() []string {
	type kv struct {
		k string
		v int
	}
	var kvs []kv
	for k, v := range p.TopHits {
		kvs = append(kvs, kv{k, v})
	}
	sort.Slice(kvs, func(i, j int) bool {
		if kvs[i].v != kvs[j].v {
			return kvs[i].v > kvs[j].v
		}
		return kvs[i].k < kvs[j].k
	})
	out := make([]string, 0, len(kvs))
	for _, x := range kvs {
		out = append(out, x.k)
	}
	return out
}
