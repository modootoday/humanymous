package main

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"math"
)

// pass_proof.go is the client submission model plus the accessibility-aware
// real-event pre-filter and its timing statistics (PLAN-07 R12: split out of
// pass_handler.go by concern). It verifies that a submitted interaction looks
// like a genuine human event stream and fingerprints it for anti-replay; it makes
// no HTTP or scoring decisions. No behavior change.

// passProof is the client submission: the 3-row offsets + an interaction proof.
type passProof struct {
	Bucket         uint64 `json:"bucket"`
	ChallengeNonce string `json:"challengeNonce"` // binds the solve to the issued instance (axis ①)
	AttestToken    string `json:"attestToken"`    // rate-limited attestation token (axis ①, when required)
	Offsets        []int  `json:"offsets"`        // per-row shift; key lands at (keyIndex+offset) mod N
	Trusted        bool   `json:"trusted"`        // all events had isTrusted === true (pre-filter)
	// Pointer/touch channel (SoT-36 §5): mouse/touch users produce these.
	Moves       int       `json:"moves"`       // distinct pointermove events
	Coalesced   int       `json:"coalesced"`   // total getCoalescedEvents() sub-samples
	Durations   []float64 `json:"durations"`   // inter-event Δt (ms)
	PathLen     float64   `json:"pathLen"`     // total pointer path length (px)
	RawT        []float64 `json:"rawT"`        // raw coalesced sample timestamps (ms)
	Pressures   []float64 `json:"pressures"`   // touch/pen pressure samples (0..1)
	PointerType string    `json:"pointerType"` // "mouse" | "touch" | "pen"
	// Keyboard channel (accessible lane): keyboard users produce these instead.
	Keys    int       `json:"keys"`    // distinct arrow/Home keydowns
	KeyDurs []float64 `json:"keyDurs"` // inter-key Δt (ms)
	// Mobile sensor channel (SoT-36 §5): device-motion magnitude samples. A real phone
	// always carries hand-tremor micro-motion; a mobile-claiming emulator/bot is flat.
	Motion []float64 `json:"motion"`
}

// traceDigest fingerprints the raw motor evidence (NOT the offsets/nonce, which
// change per instance) so a replay of the same captured trace collides even when
// wrapped around a fresh challenge (SoT-36 §5).
func traceDigest(pr passProof) string {
	h := sha256.New()
	// Quantize before hashing (audit CWE-294): a replayed human trace must still collide
	// after sub-ms noise is added to defeat an exact-match digest. Timings snap to 1ms and
	// pressures to 0.05 — coarser than any perturbation an attacker can hide below, far
	// finer than real human variance (tens of ms), so genuine distinct traces stay distinct.
	q := func(xs []float64, quantum float64) {
		for _, x := range xs {
			fmt.Fprintf(h, "%d,", int64(math.Round(x/quantum)))
		}
		h.Write([]byte("|"))
	}
	q(pr.RawT, 1)
	q(pr.Durations, 1)
	q(pr.Pressures, 0.05)
	q(pr.KeyDurs, 1)
	return hex.EncodeToString(h.Sum(nil))
}

// realEventOK is the SoT-36 §5 pre-filter, accessibility-aware: it accepts EITHER a
// pointer/touch channel OR a keyboard channel, rejecting only the obviously synthetic
// (untrusted, no interaction, perfectly-uniform timing). Keyboard users are NEVER
// required to produce pointer microstructure (that would exclude blind/AT users). It
// is a soft pre-filter, not the whole gate — attestation + engine fusion carry the
// weak/keyboard case (SoT-36 §2), and the deeper motor model is the wargame's job.
func realEventOK(pr passProof) (bool, string) {
	if !pr.Trusted {
		return false, "untrusted events"
	}
	pointer := pr.Moves >= 5 && pr.PathLen >= 20
	keyboard := pr.Keys >= 3
	if !pointer && !keyboard {
		return false, "insufficient interaction"
	}
	// Keyboard path: irregular inter-key timing is human; uniform is a bot tell.
	if keyboard && len(pr.KeyDurs) >= 4 && stddev(pr.KeyDurs) < 0.4 {
		return false, "uniform key timing"
	}
	// Pointer path: uniform Δt + missing coalesced sub-samples + no raw stream are the
	// CDP/forged-aggregate tells. Only enforced when the user actually used a pointer.
	if pointer && !keyboard {
		if len(pr.Durations) >= 5 && stddev(pr.Durations) < 0.5 {
			return false, "uniform event timing"
		}
		if pr.Coalesced != 0 && pr.Coalesced <= pr.Moves {
			return false, "no coalesced sub-samples"
		}
		if len(pr.RawT) < 10 {
			return false, "missing raw input stream"
		}
		diffs := make([]float64, 0, len(pr.RawT)-1)
		for i := 1; i < len(pr.RawT); i++ {
			dd := pr.RawT[i] - pr.RawT[i-1]
			if dd < 0 {
				return false, "non-monotonic raw timestamps"
			}
			diffs = append(diffs, dd)
		}
		if stddev(diffs) < 0.15 {
			return false, "uniform raw sample spacing"
		}
	}
	return true, ""
}

func meanFloats(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	var s float64
	for _, x := range xs {
		s += x
	}
	return s / float64(len(xs))
}

// allIntegerMs reports whether every sample is (near) a whole millisecond — real
// performance.now() deltas carry sub-ms fractions, so all-integer timing is synthetic.
func allIntegerMs(xs []float64) bool {
	if len(xs) == 0 {
		return false
	}
	for _, x := range xs {
		if d := x - math.Floor(x); d > 1e-6 && d < 1-1e-6 {
			return false
		}
	}
	return true
}

func stddev(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	var sum float64
	for _, x := range xs {
		sum += x
	}
	mean := sum / float64(len(xs))
	var v float64
	for _, x := range xs {
		d := x - mean
		v += d * d
	}
	return math.Sqrt(v / float64(len(xs)))
}
