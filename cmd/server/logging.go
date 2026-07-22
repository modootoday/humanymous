package main

import (
	"log/slog"
	"net/http"
	"os"
	"runtime/debug"
	"strings"
)

// logging.go is the PLAN-07 R11 opt-in observability seam. Structured logging is
// OFF by default: the engine emits nothing unless an operator sets -log-level (or
// HMN_LOG_LEVEL). When disabled, a.log is a DiscardHandler and the withObservability
// middleware returns the handler unwrapped — so there is zero hot-path cost AND zero
// behavior change (net/http's own panic handling is left intact). Nothing here logs
// a raw IP or User-Agent in cleartext; the session id is truncated before it is logged.

// configureLogging installs the structured logger from the flag value, falling back
// to the HMN_LOG_LEVEL env var, then to "off". Levels: off|error|warn|info|debug.
// When enabled it also becomes the slog default, so the shared httpx.WriteJSON
// encode-error warnings route through the same handler/level.
func (a *app) configureLogging(flagVal string) {
	val := strings.ToLower(strings.TrimSpace(flagVal))
	if val == "" {
		val = strings.ToLower(strings.TrimSpace(os.Getenv("HMN_LOG_LEVEL")))
	}
	lvl, ok := parseLogLevel(val)
	if !ok {
		a.log = slog.New(slog.DiscardHandler)
		a.logEnabled = false
		return
	}
	a.log = slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: lvl}))
	a.logEnabled = true
	slog.SetDefault(a.log)
}

// parseLogLevel maps a level name to an slog.Level. "off"/""/unknown → disabled.
func parseLogLevel(v string) (slog.Level, bool) {
	switch v {
	case "debug":
		return slog.LevelDebug, true
	case "info":
		return slog.LevelInfo, true
	case "warn", "warning":
		return slog.LevelWarn, true
	case "error":
		return slog.LevelError, true
	default: // "off", "none", "", or unrecognized → logging disabled
		return 0, false
	}
}

// withObservability logs one structured line per request and recovers panics into a
// clean 500 (instead of a dropped connection). It is strictly opt-in: when logging is
// disabled it returns next unchanged, so there is no added cost and no change to the
// default panic behavior.
func (a *app) withObservability(next http.Handler) http.Handler {
	if !a.logEnabled {
		return next
	}
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		defer func() {
			if rec := recover(); rec != nil {
				a.log.Error("panic recovered",
					"method", r.Method, "path", r.URL.Path, "err", rec, "stack", string(debug.Stack()))
				w.WriteHeader(http.StatusInternalServerError)
			}
		}()
		next.ServeHTTP(w, r)
		a.log.Info("request", "method", r.Method, "path", r.URL.Path, "proto", protoVer(r))
	})
}

// shortSID truncates a session id for logs: enough to correlate lines within a run,
// not the full cookie value (which is a bearer-style secret and must not be logged whole).
func shortSID(sid string) string {
	if len(sid) > 8 {
		return sid[:8]
	}
	return sid
}
