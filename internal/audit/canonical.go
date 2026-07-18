package audit

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"sort"
	"strconv"
	"strings"

	"golang.org/x/text/unicode/norm"
)

// canonical.go is the FROZEN canonical form for an audit Record (SoT-18 §3).
// It is deliberately NOT the 6-header request canonicalizer in internal/integrity
// (that one cannot serialize a nested typed record — a category error flagged in
// design review). This spec pins field order, the absent/empty/null distinction,
// integer formatting, Unicode NFC, and array ordering so the offline verifier
// reproduces record_hash byte-for-byte across Go versions and refactors.
//
// Rules:
//   - Fields are emitted in a FIXED order (below), never alphabetized.
//   - Absent optional scalars are OMITTED entirely (no key); empty arrays emit
//     "[]"; null is never produced.
//   - Integers are minimal decimal (no leading zero, no sign for non-negative).
//   - All strings are NFC-normalized.
//   - triggered_rules is sorted ascending; top_signals is sorted by id.
//   - prev_hash/record_hash/hmac are EXCLUDED from the canonical body.
//
// Encoding is a length-prefixed key/value stream (unambiguous framing), so no
// value can be confused with a field boundary.

// canonicalBytes serializes the record's semantic fields (excluding hashes) into
// the frozen byte form. prevHash is appended by the caller for chaining.
func canonicalBytes(r *Record) []byte {
	var b strings.Builder
	put := func(key, val string) {
		if val == "" {
			return // absent scalar -> omit
		}
		writeField(&b, key, norm.NFC.String(val))
	}
	putInt := func(key string, v int64) {
		writeField(&b, key, strconv.FormatInt(v, 10))
	}

	// FIXED field order (SoT-18 §2 schema order). event_id/seq/node_id/ts first.
	put("event_id", r.EventID)
	putInt("seq", int64(r.Seq))
	put("node_id", r.NodeID)
	put("ts", r.TS)
	put("event_type", r.EventType)
	put("event_version", r.EventVer)
	put("actor.kind", r.Actor.Kind)
	put("actor.id", r.Actor.IDPsn)
	put("tenant_id", r.TenantID)
	put("session_pseudonym", r.SessionPsn)
	put("request_id", r.RequestID)
	put("correlation_id", r.Correlation)
	put("host", r.Host)
	put("route_class", r.RouteClass)
	put("verdict", r.Verdict)
	putInt("risk_score", int64(r.RiskScore))

	// triggered_rules: sorted ascending, always present (empty -> "[]").
	rules := append([]string(nil), r.Rules...)
	sort.Strings(rules)
	writeField(&b, "triggered_rules", "["+strings.Join(rules, ",")+"]")

	// top_signals: sorted by id; each element id|verdict|conf (conf fixed 3dp).
	sigs := append([]Signal(nil), r.TopSignals...)
	sort.Slice(sigs, func(i, j int) bool { return sigs[i].ID < sigs[j].ID })
	parts := make([]string, len(sigs))
	for i, s := range sigs {
		parts[i] = norm.NFC.String(s.ID) + "|" + s.Verdict + "|" + strconv.FormatFloat(s.Conf, 'f', 3, 64)
	}
	writeField(&b, "top_signals", "["+strings.Join(parts, ",")+"]")

	put("enforcement_action", r.Action)
	put("enforcement_mode", r.Mode)
	put("fail_mode", r.FailMode)
	put("fail_reason", r.FailReason)
	putInt("latency_us", int64(r.LatencyUS))
	if r.Upstream != nil {
		putInt("upstream.status", int64(r.Upstream.Status))
		put("upstream.error_class", r.Upstream.ErrorClass)
	}
	if r.TLS != nil {
		put("tls.ja4", r.TLS.JA4Psn)
		put("tls.h2fp", r.TLS.H2FPPsn)
		put("tls.sni", r.TLS.SNIPsn)
	}
	put("config_version", r.ConfigVer)
	put("key_id", r.KeyID)
	put("data_class", r.DataClass)

	return []byte(b.String())
}

// writeField appends a length-prefixed key/value pair: <klen><key><vlen><val>.
// Length prefixes make the stream unambiguous (no delimiter can appear in data).
func writeField(b *strings.Builder, key, val string) {
	var lp [4]byte
	binary.BigEndian.PutUint32(lp[:], uint32(len(key)))
	b.Write(lp[:])
	b.WriteString(key)
	binary.BigEndian.PutUint32(lp[:], uint32(len(val)))
	b.Write(lp[:])
	b.WriteString(val)
}

// computeRecordHash returns hex SHA256(canonical(record) ‖ prevHash) (SoT-18 §4).
func computeRecordHash(r *Record, prevHash string) string {
	h := sha256.New()
	h.Write(canonicalBytes(r))
	h.Write([]byte(prevHash))
	return hex.EncodeToString(h.Sum(nil))
}
