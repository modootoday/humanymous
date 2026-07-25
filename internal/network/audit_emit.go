package network

import "github.com/modootoday/humanymous/internal/signals"

// AuditRecord is a minimal audit payload for network residuals. Callers map it
// onto audit.Record (EventType / FailReason / TopSignals) so this package does
// not import internal/audit (avoids import cycles with gate/server).
type AuditRecord struct {
	EventType string
	// SignalID is the residual id (for TopSignals / ban reason tags).
	SignalID string
	Verdict  string
	Notes    string
}

// AuditRecordsFromSignals builds one AuditRecord per score-exempt residual that
// has an audit mapping. BOT/SUSPICIOUS residuals are always included; UNKNOWN
// observational markers (e.g. tcp.not_observed) are included so operators can
// see when the L4 plane is dark.
func AuditRecordsFromSignals(sigs []signals.Signal) []AuditRecord {
	var out []AuditRecord
	for _, s := range sigs {
		evt := AuditEventFor(s.ID)
		if evt == "" {
			continue
		}
		notes := s.Notes
		if notes == "" {
			notes = s.ID
		}
		out = append(out, AuditRecord{
			EventType: evt,
			SignalID:  s.ID,
			Verdict:   string(s.Verdict),
			Notes:     notes,
		})
	}
	return out
}
