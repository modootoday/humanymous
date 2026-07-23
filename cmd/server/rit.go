package main

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"net/http"
	"strconv"
	"time"

	"github.com/modootoday/humanymous/internal/integrity"
	"github.com/modootoday/humanymous/internal/signals"
)

// rit.go is the live server-side wiring of the Rotating Integrity Token
// (SoT-07). The live canonical signs over session + counter + time-bucket + body
// hash — the fetch/XHR-controllable surface — so body/replay tampering of an
// /api/* call is detected. Header tampering is caught separately by L5/L6.
//
//	canonical = sid "\n" n "\n" tb "\n" sha256hex(body)

const ritWindow = 10 // seconds per time bucket

// ritCanonical builds the live canonical string for a request.
func ritCanonical(sid string, n, tb uint64, body []byte) string {
	h := sha256.Sum256(body)
	return sid + "\n" + strconv.FormatUint(n, 10) + "\n" +
		strconv.FormatUint(tb, 10) + "\n" + hex.EncodeToString(h[:])
}

// issueSeed returns seed_n (base64) for the session's current counter, to be
// sent to the client for signing its next request.
func (a *app) issueSeed(sid string) (seedB64 string, n uint64, window int) {
	ks := integrity.SessionKey(a.masterKey, sid)
	n = a.store.RITCounter(sid)
	seed := integrity.Seed(ks, n)
	return base64.RawURLEncoding.EncodeToString(seed), n, ritWindow
}

// verifyRIT verifies the RIT token on an /api/* request and returns the signal
// it produces (SoT-07 §4). The expected token is HMAC(seed_lastN, canonical),
// matching the client (web/js/rit.js) exactly — the canonical already carries n
// and tb as text, so no extra binding is appended. On success it advances the
// session's seed counter.
func (a *app) verifyRIT(sid string, r *http.Request, body []byte) signals.Signal {
	src := signals.SourceServer
	lastN := a.store.RITCounter(sid)
	token := r.Header.Get("X-HM-Token")
	presentedN := parseUint(r.Header.Get("X-HM-N"))
	presentedTB := parseUint(r.Header.Get("X-HM-TB"))
	currentTB := integrity.TimeBucket(time.Now().Unix(), ritWindow)

	// First request in a session has no token yet — ONE-SHOT grace (SoT-07 §4.3). The
	// grace is consumed once per session so a bot cannot omit X-HM-Token on every call to
	// permanently re-enter grace and suppress l5.rit.absent (deep-review): the SECOND
	// tokenless request emits the absent-token BOT signal. Honest clients fetch /api/session
	// before their first collect, so they present a token and never rely on the grace. The
	// grace signal is DISTINCT from a verified l5.rit.ok so it is not mistaken for proof of
	// JS execution (see the HR-18 cross-check).
	if lastN == 0 && token == "" {
		if a.store.ClaimRITBootstrap(sid, time.Now()) {
			return signals.New("l5.rit.bootstrap", "bootstrap", signals.VerdictOK, 1.0, src, "first request, token grace")
		}
		return signals.New("l5.rit.absent", true, signals.VerdictBot, 1.0, src, "repeated tokenless API call (grace already consumed)")
	}
	if token == "" {
		return signals.New("l5.rit.absent", true, signals.VerdictBot, 1.0, src, "no RIT token on API call")
	}
	// Replay / stale checks.
	if presentedN != lastN+1 {
		return signals.New("l5.rit.stale_replay", presentedN, signals.VerdictSuspicious, 1.0, src,
			"RIT counter out of sequence (replay)")
	}
	if !withinTB(presentedTB, currentTB, 1) {
		return signals.New("l5.rit.stale_replay", presentedTB, signals.VerdictSuspicious, 1.0, src,
			"RIT time bucket outside window (replay)")
	}
	// HMAC over the OBSERVED canonical with seed_{lastN}.
	seed := integrity.Seed(integrity.SessionKey(a.masterKey, sid), lastN)
	canonical := ritCanonical(sid, presentedN, presentedTB, body)
	mac := hmac.New(sha256.New, seed)
	mac.Write([]byte(canonical))
	expect := base64.RawURLEncoding.EncodeToString(mac.Sum(nil))
	if !hmac.Equal([]byte(expect), []byte(token)) {
		// This one signal SUBSUMES both the "invalid HMAC" and "body mismatch" cases: the
		// canonical binds session + counter + time-bucket + body hash, so any of them
		// diverging fails the single equality check here. HR-5 and HR-16 also list the
		// finer-grained l5.rit.invalid_hmac / l5.rit.body_mismatch ids so those rules already
		// hold if a future verifier distinguishes the cases; the reference collapses them into
		// l5.rit.header_tampered (splitting them would change frozen scoring weights, so it is
		// intentionally not done here). No detection gap — every RIT failure still fires HR-16.
		return signals.New("l5.rit.header_tampered", true, signals.VerdictBot, 1.0, src,
			"signed request != observed (invalid HMAC / body or token tampered)")
	}
	a.store.AdvanceRIT(sid, time.Now())
	return signals.New("l5.rit.ok", true, signals.VerdictOK, 1.0, src, "RIT valid")
}

func withinTB(a, b, tol uint64) bool {
	if a > b {
		return a-b <= tol
	}
	return b-a <= tol
}

func parseUint(s string) uint64 {
	v, _ := strconv.ParseUint(s, 10, 64)
	return v
}
