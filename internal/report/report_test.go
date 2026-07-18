package report

import "testing"

func TestAggregate_TPRandFPR(t *testing.T) {
	recs := []Record{
		{Label: "human", Verdict: "ALLOW", Outcome: "TN"},
		{Label: "bot:selenium", Verdict: "DENY", HardRuleFired: "HR-1", Outcome: "TP"},
		{Label: "bot:puppeteer", Verdict: "DENY", HardRuleFired: "HR-7", Outcome: "TP"},
		{Label: "bot:nodriver", Verdict: "CHALLENGE", HardRuleFired: "HR-12", Outcome: "TP"},
		{Profile: "camoufox.mjs", Skipped: true, Reason: "not installed"},
	}
	s := Aggregate(recs)
	if s.BotTPR != 1.0 {
		t.Errorf("BotTPR want 1.0 got %v", s.BotTPR)
	}
	if s.HumanFPR != 0.0 {
		t.Errorf("HumanFPR want 0 got %v", s.HumanFPR)
	}
	if s.BotRuns != 3 || s.HumanRuns != 1 {
		t.Errorf("run counts wrong: bot=%d human=%d", s.BotRuns, s.HumanRuns)
	}
	if len(s.Skipped) != 1 {
		t.Errorf("skipped want 1 got %d", len(s.Skipped))
	}
}

func TestAggregate_MissedBotLowersTPR(t *testing.T) {
	recs := []Record{
		{Label: "bot:a", Verdict: "DENY", Outcome: "TP"},
		{Label: "bot:b", Verdict: "ALLOW", Outcome: "FN"},
	}
	s := Aggregate(recs)
	if s.BotTPR != 0.5 {
		t.Errorf("BotTPR want 0.5 got %v", s.BotTPR)
	}
}
