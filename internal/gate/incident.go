package gate

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"strings"

	"github.com/modootoday/humanymous/internal/audit"
)

// incident.go implements opaque, non-enumerable incident handles (SoT-28 WS4).
// Exposing the monotonic record seq lets an authenticated insider trawl
// /admin/incidents/{1..N} to map the entire defense. The handle is an HMAC of
// the seq under the server key, computed at READ time only (never stored, not in
// the canonical body), so it cannot be guessed or walked; lookups accept only
// the handle, and are enumeration-capped per operator.

// incidentHandle is the opaque handle for a record seq.
func (s *Server) incidentHandle(seq uint64) string {
	m := hmac.New(sha256.New, s.tokenKey)
	var b [8]byte
	binary.BigEndian.PutUint64(b[:], seq)
	m.Write([]byte("incident|"))
	m.Write(b[:])
	return "INC-" + strings.ToUpper(hex.EncodeToString(m.Sum(nil)[:5]))
}

// resolveIncident returns the record behind an opaque handle ("" seq if none).
func (s *Server) resolveIncident(handle string) (audit.Record, bool) {
	for _, r := range s.sink.Log().Records() {
		if s.incidentHandle(r.Seq) == handle {
			r.Incident = handle
			return r, true
		}
	}
	return audit.Record{}, false
}
