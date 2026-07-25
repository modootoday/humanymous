package network

import (
	"testing"

	"github.com/modootoday/humanymous/internal/signals"
)

func TestScoreExemptResiduals(t *testing.T) {
	for _, id := range []string{
		"l5.header.proxy_hop", "l5.proxy.anon_chain", "l5.tcp.ttl_os",
		"l5.correlation.proxy_rotation", "l5.ip.tor_exit",
	} {
		if !IsScoreExempt(id) {
			t.Fatalf("%s should be score-exempt", id)
		}
	}
	if IsScoreExempt("l5.rit.absent") {
		t.Fatal("RIT integrity must still score")
	}
}

func TestAuditRecordsFromSignals(t *testing.T) {
	sigs := []signals.Signal{
		signals.New("l5.header.proxy_hop", "via-squid", signals.VerdictBot, 1, signals.SourceServer, "squid"),
		signals.New("l5.rit.absent", true, signals.VerdictBot, 1, signals.SourceServer, "rit"),
	}
	recs := AuditRecordsFromSignals(sigs)
	if len(recs) != 1 || recs[0].EventType != "net.proxy.hop" {
		t.Fatalf("want one net.proxy.hop audit record, got %+v", recs)
	}
}
