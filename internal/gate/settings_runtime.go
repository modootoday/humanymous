package gate

import (
	"fmt"
	"strings"
	"sync/atomic"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/gate/settings"
)

// settingsRuntimeStats is a fixed-label Prometheus accumulator. Keeping the
// label set closed avoids an operator-controlled cardinality surface.
type settingsRuntimeStats struct {
	applied     atomic.Uint64
	rolledBack  atomic.Uint64
	rejected    atomic.Uint64
	errors      atomic.Uint64
	storeErrors atomic.Uint64
}

func (s *Server) validateSettingsRuntime(eff settings.Effective) error {
	if eff.RateWindowSec < 1 || eff.RateSoft < 1 || eff.RateHard < eff.RateSoft {
		return fmt.Errorf("settings: invalid effective rate limit %d/%d/%d",
			eff.RateWindowSec, eff.RateSoft, eff.RateHard)
	}
	if _, ok := s.bans.(RateConfigurableBanLedger); !ok {
		bootWindow := int(rlWindow(s.cfg).Seconds())
		if eff.RateWindowSec != bootWindow || eff.RateSoft != rlSoft(s.cfg) || eff.RateHard != rlHard(s.cfg) {
			return fmt.Errorf("settings: configured BanLedger does not support rate hot-apply")
		}
	}
	return nil
}

func (s *Server) syncSettingsRuntime(eff settings.Effective) {
	if rate, ok := s.bans.(RateConfigurableBanLedger); ok {
		rate.ConfigureRate(time.Duration(eff.RateWindowSec)*time.Second, eff.RateSoft, eff.RateHard)
	}
}

func (s *Server) recordSettingsStoreError(reason string) {
	s.settingsStats.storeErrors.Add(1)
	if s.sink != nil {
		s.sink.Emit(audit.Record{
			EventType: "settings.store.degraded", Actor: audit.Actor{Kind: "system"},
			TenantID: s.cfg.NodeID, Mode: "monitor", KeyID: "k1", FailReason: reason,
		})
	}
}

func (s *Server) settingsApplyError(err error) error {
	s.settingsStats.errors.Add(1)
	return err
}

func (s *Server) settingsApplyRejected(err error) error {
	s.settingsStats.rejected.Add(1)
	if s.sink != nil {
		s.sink.Emit(audit.Record{
			EventType: "settings.overlay.rejected", Actor: audit.Actor{Kind: "system"},
			TenantID: s.cfg.NodeID, Mode: "monitor", KeyID: "k1", FailReason: err.Error(),
		})
	}
	return err
}

func prometheusLabel(v string) string {
	r := strings.NewReplacer(`\`, `\\`, "\n", `\n`, `"`, `\"`)
	return r.Replace(v)
}

func (s *Server) settingsPendingAgeSeconds() float64 {
	oldest := 0.0
	now := s.nowFn()
	for _, p := range s.approvals.Pending() {
		if p.Kind != "settings.overlay" && p.Kind != "settings.overlay.rollback" {
			continue
		}
		if age := now.Sub(p.Created).Seconds(); age > oldest {
			oldest = age
		}
	}
	return oldest
}
