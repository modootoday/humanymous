package gate

import (
	"net/http"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/pkg/origincloak"
)

// guard.go implements the proxy↔upstream trust-boundary guards (SoT-23): origin
// cloaking (HR-24) and inbound trust-header hygiene/flagging (HR-27b).

// internalHeaders are headers a client must never send inbound: our own trust
// headers plus the standard forwarding headers we re-derive from the socket
// (SoT-23 §4). Their presence from an external client is itself an attack signal.
var internalHeaders = []string{
	"X-Hmny-Origin-Auth", // our origin-cloaking secret (HR-24)
	"X-Hmn-Verdict",      // our internal verdict header
	"X-Forwarded-For",    // re-derived from the socket
	"X-Real-Ip",
	"Forwarded",
	"Cf-Connecting-Ip",
}

// spoofScan reports which internal/trust headers an inbound request carried. A
// non-empty result means a client tried to forge source IP / impersonate the
// proxy's trusted channel (HR-27b). The caller strips them and audits.
func spoofScan(r *http.Request) []string {
	var found []string
	for _, h := range internalHeaders {
		if r.Header.Get(h) != "" {
			found = append(found, h)
		}
	}
	return found
}

// stripInbound removes every internal/trust header so nothing forged survives to
// the upstream or into scoring.
func stripInbound(r *http.Request) {
	for _, h := range internalHeaders {
		r.Header.Del(h)
	}
}

// The origin-cloaking construction (rotating X-Hmny-Origin-Auth token + epoch grace)
// lives in the PUBLIC, importable pkg/origincloak so an origin app can verify + strip it
// with the same code Gate uses. These thin wrappers keep the internal call sites unchanged
// and guarantee a single source of truth (the enforcement path and the public package are
// the same code, so they cannot drift).

// originAuth computes the rotating origin-cloaking token the proxy attaches to upstream
// requests (SoT-23 §1, HR-24). The origin validates + strips it; a request arriving without
// it is a direct-hit bypass. `epoch` rotates the value so a leaked token expires.
func originAuth(key []byte, epoch string) string { return origincloak.Token(key, epoch) }

// originEpoch returns the wall-clock-derived origin epoch for now.
func originEpoch(now time.Time) string { return origincloak.Epoch(now) }

// originEpochGrace returns the next/current/previous epoch set an origin should accept to
// tolerate a bucket boundary crossed in either direction (a past-only grace blackholes
// traffic when the gate clock is ahead).
func originEpochGrace(now time.Time) []string { return origincloak.EpochGrace(now) }

// ValidateOriginAuth is the ORIGIN-side check. It delegates to pkg/origincloak (the public
// copy an origin app imports directly); returns true only if the header matches one of the
// grace epochs (constant-time).
func ValidateOriginAuth(key []byte, header string, epochs ...string) bool {
	return origincloak.ValidAt(key, header, epochs...)
}

// spoofRecord builds the audit event for a detected trust-header spoof.
func spoofRecord(nodeID, sidPsn string, found []string) audit.Record {
	return audit.Record{
		EventType:  audit.EventHdrInternalIngress,
		Actor:      audit.Actor{Kind: "subject", IDPsn: sidPsn},
		TenantID:   nodeID,
		RouteClass: "html",
		Verdict:    string(VerdictDeny),
		Rules:      []string{"HR-27b"},
		Action:     "block",
		Mode:       "enforce",
		FailReason: "inbound internal headers: " + strings.Join(found, ","),
		KeyID:      "k1",
	}
}
