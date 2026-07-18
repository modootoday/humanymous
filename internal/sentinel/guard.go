package sentinel

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"net/http"
	"strings"

	"github.com/modootoday/humanymous/internal/audit"
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

// originAuth computes the rotating origin-cloaking token the proxy attaches to
// upstream requests (SoT-23 §1). The origin validates + strips it; a request
// arriving at the origin without it is a direct-hit bypass (HR-24). `epoch`
// rotates the value so a leaked token expires.
func originAuth(key []byte, epoch string) string {
	m := hmac.New(sha256.New, key)
	m.Write([]byte("hmny-origin|" + epoch))
	return hex.EncodeToString(m.Sum(nil))
}

// ValidateOriginAuth is the ORIGIN-side check (used by a compliant upstream or
// the demo origin guard): returns true only if the header matches the current or
// previous epoch (rotation grace, SoT-22 §4).
func ValidateOriginAuth(key []byte, header string, epochs ...string) bool {
	for _, e := range epochs {
		if hmac.Equal([]byte(header), []byte(originAuth(key, e))) {
			return true
		}
	}
	return false
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
