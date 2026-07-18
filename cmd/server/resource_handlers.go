package main

import (
	"io"
	"mime"
	"net/http"
	"path"
	"path/filepath"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/resource"
	"github.com/modootoday/humanymous/internal/signals"
	"github.com/modootoday/humanymous/internal/watermark"
)

// resource_handlers.go serves /res/* with adaptive gating (SoT-10) and per-
// session forensic watermarking (SoT-08), and exposes /api/trace and
// /api/traffic/{sid}.

// handleResource gates + watermarks a resource by session trust.
func (a *app) handleResource(w http.ResponseWriter, r *http.Request) {
	sid := a.ensureSession(w, r)
	asset := path.Clean("/" + strings.TrimPrefix(r.URL.Path, "/res/"))
	if strings.Contains(asset, "..") {
		http.Error(w, "bad path", http.StatusBadRequest)
		return
	}
	file := filepath.Join(a.webDir, filepath.FromSlash(asset))
	src, err := readFile(file)
	if err != nil {
		http.NotFound(w, r)
		return
	}
	mimeType := mime.TypeByExtension(filepath.Ext(file))
	if mimeType == "" {
		mimeType = "application/octet-stream"
	}

	// --- gating (SoT-10) ---
	tier := resource.Classify(asset, mimeType, int64(len(src)), r.Header.Get("Sec-Fetch-Dest"))
	verdict := a.sessionVerdict(sid)
	ritOK := !a.ritOn || r.Header.Get("X-HM-Token") != ""
	liveness := a.sessionLiveness(sid)
	decision := resource.Gate(tier, verdict, ritOK, liveness)
	w.Header().Set("X-HM-Gate", decision.String()+":"+tier.String())

	// --- media / embed inspection signals (SoT-10) ---
	a.recordResourceSignals(sid, r, tier)

	switch decision {
	case resource.DenyServe:
		http.Error(w, "resource blocked for this session", http.StatusForbidden)
		return
	case resource.Challenge:
		http.Error(w, "verification required for heavy resource", http.StatusUnavailableForLegalReasons)
		return
	case resource.Downgrade:
		// Serve a small preview placeholder instead of the full heavy resource.
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		w.Header().Set("X-HM-Downgrade", "preview")
		_, _ = io.WriteString(w, "[preview] full resource withheld pending trust (SoT-10)")
		return
	}

	// --- watermark (SoT-08) ---
	tagID := a.ledger.Allocate()
	p := watermark.Payload{TagID: tagID, SessionID: sid, AssetID: asset, Bucket: dayBucket()}
	out, channel, werr := watermark.Apply(a.masterKey, mimeType, src, p)
	if werr != nil {
		out = src
		channel = "none"
	}
	a.ledger.Put(watermark.Record{Payload: p, MIME: mimeType, Channel: channel, IssuedAt: time.Now(), IPHash: trafficIPHash(r)})

	w.Header().Set("Content-Type", mimeType)
	w.Header().Set("Cache-Control", "no-store") // per-session unique
	w.Header().Set("X-HM-Watermark", channel)
	_, _ = w.Write(out)
}

// handleTrace recovers the watermark from an uploaded (leaked) resource and
// identifies the session (SoT-08 §4). Admin endpoint.
func (a *app) handleTrace(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "POST only", http.StatusMethodNotAllowed)
		return
	}
	mimeType := r.Header.Get("Content-Type")
	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 32<<20))
	if err != nil {
		http.Error(w, "read error", http.StatusBadRequest)
		return
	}
	res, err := a.ledger.Trace(a.masterKey, mimeType, body)
	if err != nil {
		writeJSON(w, map[string]any{"traced": false, "reason": err.Error()})
		return
	}
	writeJSON(w, map[string]any{"traced": true, "result": res})
}

// handleTraffic returns a session's traffic log (SoT-12 debug).
func (a *app) handleTraffic(w http.ResponseWriter, r *http.Request) {
	sid := strings.TrimPrefix(r.URL.Path, "/api/traffic/")
	writeJSON(w, a.tlog.Session(sid))
}

// recordResourceSignals appends media/embed signals for a resource request.
func (a *app) recordResourceSignals(sid string, r *http.Request, tier resource.Tier) {
	now := time.Now()
	var sigs []signals.Signal

	ext := r.Header.Get("Sec-Fetch-Dest")
	if embedSigs := resource.Inspect(resource.EmbedContext{
		SecFetchDest: ext,
		Origin:       r.Header.Get("Origin"),
		AllowedHosts: nil, // demo: permit all origins
		RITPresent:   r.Header.Get("X-HM-Token") != "",
	}); len(embedSigs) > 0 {
		sigs = append(sigs, embedSigs...)
	}

	if tier == resource.TierHeavy || tier == resource.TierMedia {
		mediaSigs := a.media.Observe(resource.MediaEvent{
			SessionID:   sid,
			IsRange:     r.Header.Get("Range") != "",
			ExternalRef: externalReferer(r),
			HasRIT:      r.Header.Get("X-HM-Token") != "",
			At:          now,
		})
		sigs = append(sigs, mediaSigs...)
	}
	if len(sigs) > 0 && sid != "" {
		a.store.AppendNetworkSignals(sid, sigs, now)
	}
}

func (a *app) sessionVerdict(sid string) string {
	if rep, ok := a.store.Get(sid); ok && rep.Scoring.Verdict != "" {
		return rep.Scoring.Verdict
	}
	return resource.VerdictChallenge // unknown trust -> conservative on heavy tiers
}

func (a *app) sessionLiveness(sid string) bool {
	if rep, ok := a.store.Get(sid); ok {
		return rep.Client.Behavior.Mouse.Samples > 3 || rep.Client.Behavior.Key.Keystrokes > 0
	}
	return false
}

func externalReferer(r *http.Request) bool {
	ref := r.Header.Get("Referer")
	if ref == "" {
		return false
	}
	return !strings.Contains(ref, r.Host)
}
