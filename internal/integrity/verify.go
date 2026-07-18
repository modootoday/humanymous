package integrity

import "github.com/modootoday/humanymous/internal/signals"

// verify.go turns a RIT verification into an L5 signal (SoT-07 §4). It is pure
// (no http, no clock): the caller supplies the expected session key, the last
// issued counter, the request's presented token/n/tb, the canonical input, and
// the current time bucket. This keeps it unit-testable.

// VerifyParams is everything needed to judge one request's RIT token.
type VerifyParams struct {
	SessionKey  []byte
	LastN       uint64 // last counter the server issued a seed for
	PresentedN  uint64 // n claimed by the client
	PresentedTB uint64 // time bucket claimed by the client
	CurrentTB   uint64 // server's current time bucket
	Token       string // presented token ("" if absent)
	Canonical   string // canonical string rebuilt from OBSERVED request
	FirstReq    bool   // bootstrap request (token not yet expected)
}

// Verify returns the RIT signal for a request. A valid token yields l5.rit.ok
// (OK, no risk); each failure mode yields its specific bot/suspicious signal.
func Verify(p VerifyParams) signals.Signal {
	src := signals.SourceServer
	// Bootstrap: first request has no token yet — not a violation.
	if p.FirstReq {
		return signals.New("l5.rit.ok", "bootstrap", signals.VerdictOK, 1.0, src, "first request, token grace")
	}
	if p.Token == "" {
		return signals.New("l5.rit.absent", true, signals.VerdictBot, 1.0, src, "no RIT token on API call")
	}
	// Replay / stale: counter must be the expected next one; time bucket within +-1.
	if p.PresentedN != p.LastN+1 {
		return signals.New("l5.rit.stale_replay", p.PresentedN, signals.VerdictSuspicious, 1.0, src,
			"RIT counter out of sequence (replay)")
	}
	if !withinBucket(p.PresentedTB, p.CurrentTB, 1) {
		return signals.New("l5.rit.stale_replay", p.PresentedTB, signals.VerdictSuspicious, 1.0, src,
			"RIT time bucket outside window (replay)")
	}
	// HMAC: recompute over the OBSERVED canonical with seed_{lastN}.
	seed := Seed(p.SessionKey, p.LastN)
	if !VerifyToken(seed, p.Canonical, p.PresentedN, p.PresentedTB, p.Token) {
		return signals.New("l5.rit.header_tampered", true, signals.VerdictBot, 1.0, src,
			"signed request != observed (header/body tampered)")
	}
	return signals.New("l5.rit.ok", true, signals.VerdictOK, 1.0, src, "RIT valid")
}

// withinBucket reports |a-b| <= tol using unsigned-safe arithmetic.
func withinBucket(a, b, tol uint64) bool {
	if a > b {
		return a-b <= tol
	}
	return b-a <= tol
}
