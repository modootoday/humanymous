package sentinel

import (
	"net/http"
	"strings"

	"github.com/modootoday/humanymous/internal/audit"
)

// smuggle.go normalizes request framing to defeat HTTP request smuggling on the
// proxy↔upstream hop (SoT-23 §3, HR-23). Go's net/http already rejects several
// desync primitives at parse time; this guard makes the policy EXPLICIT and
// auditable, and covers the ambiguous combinations the RFC leaves to the proxy:
// Content-Length together with Transfer-Encoding, duplicate/conflicting
// Content-Length, and a Transfer-Encoding that is not exactly "chunked".

// smuggleReason names the detected framing ambiguity.
type smuggleReason string

const (
	smuggleNone      smuggleReason = ""
	smuggleTECL      smuggleReason = "te_cl_conflict"     // CL and TE both present
	smuggleDupCL     smuggleReason = "ambiguous_len"      // multiple/conflicting CL
	smuggleBadTE     smuggleReason = "te_not_chunked"     // TE present but not exactly chunked
	smuggleObsFold   smuggleReason = "obs_fold"           // header value with embedded CR/LF
)

// smuggleScan inspects the request headers for framing ambiguity (HR-23).
func smuggleScan(r *http.Request) smuggleReason {
	te := r.Header.Values("Transfer-Encoding")
	cl := r.Header.Values("Content-Length")

	if len(te) > 0 && len(cl) > 0 {
		return smuggleTECL // CL.TE / TE.CL desync primitive
	}
	// Duplicate or conflicting Content-Length values.
	if len(cl) > 1 {
		first := strings.TrimSpace(cl[0])
		for _, v := range cl[1:] {
			if strings.TrimSpace(v) != first {
				return smuggleDupCL
			}
		}
		return smuggleDupCL // even identical duplicates are rejected (RFC 9110)
	}
	// Transfer-Encoding must be exactly "chunked" (case-insensitive), never a
	// list like "chunked, chunked" / "identity" / "gzip, chunked" tricks.
	for _, v := range te {
		if !strings.EqualFold(strings.TrimSpace(v), "chunked") {
			return smuggleBadTE
		}
	}
	// obs-fold / embedded CR-LF in any header value (bare-LF smuggling).
	for _, vals := range r.Header {
		for _, v := range vals {
			if strings.ContainsAny(v, "\r\n") {
				return smuggleObsFold
			}
		}
	}
	return smuggleNone
}

// smuggleRecord builds the audit event for a detected smuggling attempt (HR-23).
func smuggleRecord(nodeID, sidPsn string, reason smuggleReason) audit.Record {
	return audit.Record{
		EventType:  audit.EventSmuggle,
		Actor:      audit.Actor{Kind: "subject", IDPsn: sidPsn},
		TenantID:   nodeID,
		Verdict:    string(VerdictDeny),
		Rules:      []string{"HR-23"},
		Action:     "block",
		Mode:       "enforce",
		FailReason: "request framing ambiguity: " + string(reason),
		KeyID:      "k1",
	}
}
