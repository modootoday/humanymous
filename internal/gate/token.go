package gate

import (
	"crypto/hmac"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"
)

// token.go implements the verdict trust token (SoT-21 §3, HR-28) and the
// single-use proof-nonce cache (SoT-21 §4, HR-29).
//
// The verdict token is a server-key-only HMAC over {sid, bind, exp, epoch}. It
// is bound to a client-fingerprint surrogate (`bind`) so an ALLOW token lifted
// from a scored human and replayed from a bot with a different fingerprint fails
// the binding check. It has a short TTL and a rotating epoch. There is no client
// algorithm field, so an alg:none downgrade is impossible.
//
// NOTE: at this proxy edge we do not (yet) capture TLS JA4, so `bind` is derived
// from the stable request-header fingerprint (UA + Accept-Language + sec-ch-ua).
// Production binds to ja4Stable + device-fp + subnet-class (SoT-21 §3); the
// verification flow is identical.

const verdictCookie = "hmn_vt"

// tokenReason enumerates why a token check failed (audit + HR-28).
type tokenReason string

const (
	tokenOK              tokenReason = ""
	tokenBadSig          tokenReason = "bad_sig"
	tokenExpired         tokenReason = "expired"
	tokenBindingMismatch tokenReason = "binding_mismatch"
	tokenMalformed       tokenReason = "malformed"
)

// bindKey derives the client-fingerprint surrogate the token is bound to. A
// fingerprint requires at least a User-Agent; without one there is nothing
// stable to key on, so we return "" and callers skip fp-based metering/binding
// (avoids collapsing all header-less clients into one shared bucket).
func bindKey(r *http.Request) string {
	if r.Header.Get("User-Agent") == "" {
		return ""
	}
	h := sha256.New()
	h.Write([]byte(r.Header.Get("User-Agent")))
	h.Write([]byte{0})
	h.Write([]byte(r.Header.Get("Accept-Language")))
	h.Write([]byte{0})
	h.Write([]byte(r.Header.Get("sec-ch-ua")))
	return base64.RawURLEncoding.EncodeToString(h.Sum(nil)[:12])
}

// tokenBind is the verdict-token binding: the header fingerprint PLUS the source
// subnet — a network anchor that is non-forgeable from a different machine, so a
// datacenter bot replaying a residential human's token lands on a different
// subnet and fails (SoT-28 WS6). JA4/device-fp are added once ClientHello capture
// is wired into the edge; subnet already carries the load-bearing weight.
func tokenBind(r *http.Request) string {
	h := sha256.New()
	h.Write([]byte(bindKey(r)))
	h.Write([]byte{0})
	h.Write([]byte(clientSubnet(r)))
	return base64.RawURLEncoding.EncodeToString(h.Sum(nil)[:16]) // opaque, no '|'
}

// issueVerdictToken mints a trust token: base64(payload) + "." + base64(mac).
func issueVerdictToken(key []byte, sid, bind, epoch string, exp time.Time) string {
	payload := sid + "|" + bind + "|" + strconv.FormatInt(exp.Unix(), 10) + "|" + epoch
	mac := tokenMAC(key, payload)
	return base64.RawURLEncoding.EncodeToString([]byte(payload)) + "." + base64.RawURLEncoding.EncodeToString(mac)
}

// verifyVerdictToken checks signature, expiry, fingerprint+subnet binding, and
// that the token is bound to THIS session (token.sid == expectSID, SoT-28 WS6).
// epochs is the accepted rotation window (current + previous, SoT-22 §4).
func verifyVerdictToken(key []byte, token, bind, expectSID string, now time.Time, epochs ...string) tokenReason {
	dot := strings.IndexByte(token, '.')
	if dot < 0 {
		return tokenMalformed
	}
	pb, err1 := base64.RawURLEncoding.DecodeString(token[:dot])
	mb, err2 := base64.RawURLEncoding.DecodeString(token[dot+1:])
	if err1 != nil || err2 != nil {
		return tokenMalformed
	}
	payload := string(pb)
	if subtle.ConstantTimeCompare(mb, tokenMAC(key, payload)) != 1 {
		return tokenBadSig
	}
	parts := strings.Split(payload, "|")
	if len(parts) != 4 {
		return tokenMalformed
	}
	exp, _ := strconv.ParseInt(parts[2], 10, 64)
	if now.Unix() > exp {
		return tokenExpired
	}
	// epoch must be in the accepted window.
	epochOK := false
	for _, e := range epochs {
		if parts[3] == e {
			epochOK = true
		}
	}
	if !epochOK {
		return tokenExpired
	}
	// Session binding: the token must be presented with the SAME session cookie
	// it was minted for, so a lifted hmn_vt without the matching hsid fails.
	if expectSID != "" && subtle.ConstantTimeCompare([]byte(parts[0]), []byte(expectSID)) != 1 {
		return tokenBindingMismatch
	}
	// Binding: the token was issued for a different fingerprint/subnet => lifted.
	if subtle.ConstantTimeCompare([]byte(parts[1]), []byte(bind)) != 1 {
		return tokenBindingMismatch
	}
	return tokenOK
}

func tokenMAC(key []byte, payload string) []byte {
	m := hmac.New(sha256.New, key)
	m.Write([]byte(payload))
	return m.Sum(nil)
}

// NonceCache is a single-use proof-nonce store (HR-29). A nonce is accepted once
// per (nonce, binding); a replay — same nonce, or the nonce from a different
// fingerprint — is rejected.
type NonceCache struct {
	mu   sync.Mutex
	seen map[string]time.Time
	ttl  time.Duration
}

// NewNonceCache builds a single-use nonce cache with a TTL sweep.
func NewNonceCache(ttl time.Duration) *NonceCache {
	return &NonceCache{seen: map[string]time.Time{}, ttl: ttl}
}

// Use records a nonce as consumed for a binding; returns false on replay.
func (c *NonceCache) Use(nonce, bind string, now time.Time) bool {
	if nonce == "" {
		return true // no nonce presented; not a replay (older client)
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	// GC opportunistically.
	for k, t := range c.seen {
		if now.Sub(t) > c.ttl {
			delete(c.seen, k)
		}
	}
	key := nonce + "|" + bind
	if _, ok := c.seen[key]; ok {
		return false // replay: this nonce+binding was already consumed
	}
	// A nonce reused across a DIFFERENT binding is also a replay (solve-once-
	// reuse-many): reject if the bare nonce was seen under any binding.
	for k := range c.seen {
		if strings.HasPrefix(k, nonce+"|") {
			return false
		}
	}
	c.seen[key] = now
	return true
}
