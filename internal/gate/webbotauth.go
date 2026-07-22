package gate

import (
	"crypto/ed25519"
	"encoding/base64"
	"strconv"
	"strings"
	"time"
)

// webbotauth.go verifies Web Bot Auth request signatures (RFC 9421 HTTP Message
// Signatures, "web-bot-auth" profile) at the gate (PLAN-08 R3). A growing set of
// legitimate automated agents (crawlers, AI agents) sign their requests with an
// Ed25519 key published in a key directory; verifying that signature lets the gate
// (a) let a KNOWN, operator-allowlisted agent through without a behavioral fight, and
// (b) DENY a forgery that claims an allowlisted agent's key but cannot prove
// possession — shrinking the population that must be caught behaviorally.
//
// Scope (reference): Ed25519 over the covered set ("@authority"), tag "web-bot-auth",
// created/expires skew bounds, keyid resolved through a KeyDirectory seam. The live
// JWKS-directory fetch (with an SSRF egress allowlist + negative cache) is a
// documented future seam; the StaticKeyDirectory here is an operator-configured
// allowlist, which is the trust anchor either way. An unsupported covered-component
// set or an unknown key is treated as NEUTRAL (never a false deny).

// agentVerdict is the outcome of verifying a request's Web Bot Auth signature.
type agentVerdict int

const (
	agentNone            agentVerdict = iota // no signature present
	agentVerifiedTrusted                     // valid signature from an operator-allowlisted key
	agentVerifiedUnknown                     // signature present but key unknown/unsupported → neutral
	agentForged                              // claims an allowlisted key but fails verification → deny
)

// KeyDirectory resolves a Web Bot Auth keyid to its public key. trusted reports
// whether the key is on the operator allowlist (only allowlisted keys can earn a
// trust-upgrade; a signature claiming an allowlisted keyid that fails is a forgery).
type KeyDirectory interface {
	Lookup(keyid string) (pub ed25519.PublicKey, trusted bool)
}

// StaticKeyDirectory is an operator-configured allowlist of trusted agent keys.
type StaticKeyDirectory struct {
	keys map[string]ed25519.PublicKey
}

// NewStaticKeyDirectory parses "keyid base64url-ed25519-pubkey" lines (one per line;
// '#' comments and blanks ignored) into a directory. Every listed key is allowlisted.
func NewStaticKeyDirectory(spec string) (*StaticKeyDirectory, error) {
	d := &StaticKeyDirectory{keys: map[string]ed25519.PublicKey{}}
	for _, line := range strings.Split(spec, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		f := strings.Fields(line)
		if len(f) != 2 {
			continue
		}
		raw, err := base64.RawURLEncoding.DecodeString(f[1])
		if err != nil || len(raw) != ed25519.PublicKeySize {
			continue
		}
		d.keys[f[0]] = ed25519.PublicKey(raw)
	}
	return d, nil
}

func (d *StaticKeyDirectory) Lookup(keyid string) (ed25519.PublicKey, bool) {
	pub, ok := d.keys[keyid]
	return pub, ok
}

// sigParams are the parsed Signature-Input parameters for one label.
type sigParams struct {
	components []string // covered component identifiers, in order (e.g. "@authority")
	inner      string   // the raw inner-list+params value (the @signature-params value)
	created    int64
	expires    int64
	keyid      string
	alg        string
	tag        string
}

// verifyAgent verifies the request's Web Bot Auth signature against dir at time now.
// It returns the verdict and the claimed keyid (for audit).
func verifyAgent(host, sigInput, sig string, dir KeyDirectory, now time.Time) (agentVerdict, string) {
	if sigInput == "" || sig == "" || dir == nil {
		return agentNone, ""
	}
	label, p, ok := parseSignatureInput(sigInput)
	if !ok {
		return agentVerifiedUnknown, "" // malformed input we don't parse → neutral, not a deny
	}
	rawSig, ok := parseSignatureValue(sig, label)
	if !ok {
		return agentVerifiedUnknown, p.keyid
	}
	// Profile gate: only Ed25519 web-bot-auth signatures are handled here.
	if p.alg != "ed25519" || p.tag != "web-bot-auth" {
		return agentVerifiedUnknown, p.keyid
	}
	// Freshness: reject stale/not-yet-valid signatures, and BOUND every token's
	// lifetime (PLAN-08 backlog) so a long-lived signature cannot be replayed
	// indefinitely — by `expires` when present, else by `created` + a max age; a token
	// carrying neither is unbounded and rejected.
	const skew = 60
	const maxLifetime = int64(3600) // 1 hour
	nowU := now.Unix()
	if p.created != 0 && p.created > nowU+skew {
		return agentVerifiedUnknown, p.keyid // not yet valid
	}
	switch {
	case p.expires != 0:
		if p.expires < nowU-skew {
			return agentVerifiedUnknown, p.keyid // expired
		}
		if p.created != 0 && p.expires-p.created > maxLifetime {
			return agentVerifiedUnknown, p.keyid // over-long declared lifetime
		}
	case p.created != 0:
		if nowU-p.created > maxLifetime {
			return agentVerifiedUnknown, p.keyid // no expiry → bound by created + max age
		}
	default:
		return agentVerifiedUnknown, p.keyid // neither created nor expires → unbounded
	}
	// Only the "@authority" covered set is supported in this reference; anything else
	// is unverifiable here → neutral.
	if len(p.components) != 1 || p.components[0] != "@authority" {
		return agentVerifiedUnknown, p.keyid
	}
	pub, trusted := dir.Lookup(p.keyid)
	if !trusted {
		return agentVerifiedUnknown, p.keyid // unknown key: cannot verify, do not trust
	}
	base := buildSignatureBase(host, p.inner)
	if ed25519.Verify(pub, []byte(base), rawSig) {
		return agentVerifiedTrusted, p.keyid
	}
	return agentForged, p.keyid // claims an allowlisted key but the signature does not verify
}

// buildSignatureBase reconstructs the RFC 9421 signature base for the covered set
// {"@authority"} plus the @signature-params line (the inner Signature-Input value).
func buildSignatureBase(host, inner string) string {
	authority := strings.ToLower(host)
	return "\"@authority\": " + authority + "\n\"@signature-params\": " + inner
}

// parseSignatureInput parses `label=(...);params...` returning the label and params.
// It is a targeted parser for the web-bot-auth shape, not a full RFC 8941 parser.
func parseSignatureInput(v string) (string, sigParams, bool) {
	v = strings.TrimSpace(v)
	eq := strings.IndexByte(v, '=')
	if eq <= 0 {
		return "", sigParams{}, false
	}
	label := v[:eq]
	inner := strings.TrimSpace(v[eq+1:])
	open, close := strings.IndexByte(inner, '('), strings.IndexByte(inner, ')')
	if open != 0 || close < 0 {
		return "", sigParams{}, false
	}
	var p sigParams
	p.inner = inner
	for _, c := range strings.Fields(inner[1:close]) {
		p.components = append(p.components, strings.Trim(c, "\""))
	}
	for _, kv := range strings.Split(inner[close+1:], ";") {
		kv = strings.TrimSpace(kv)
		if kv == "" {
			continue
		}
		k, val, found := strings.Cut(kv, "=")
		if !found {
			continue
		}
		val = strings.Trim(val, "\"")
		switch k {
		case "created":
			p.created, _ = strconv.ParseInt(val, 10, 64)
		case "expires":
			p.expires, _ = strconv.ParseInt(val, 10, 64)
		case "keyid":
			p.keyid = val
		case "alg":
			p.alg = val
		case "tag":
			p.tag = val
		}
	}
	return label, p, true
}

// parseSignatureValue extracts the base64 signature bytes for label from a Signature
// header of the form `label=:base64:`.
func parseSignatureValue(v, label string) ([]byte, bool) {
	v = strings.TrimSpace(v)
	prefix := label + "="
	if !strings.HasPrefix(v, prefix) {
		return nil, false
	}
	body := strings.TrimPrefix(v, prefix)
	if len(body) < 2 || body[0] != ':' || body[len(body)-1] != ':' {
		return nil, false
	}
	raw, err := base64.StdEncoding.DecodeString(body[1 : len(body)-1])
	if err != nil {
		return nil, false
	}
	return raw, true
}
