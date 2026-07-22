// Package httpx holds tiny HTTP helpers shared by the Core engine and the Gate so
// they cannot drift (PLAN-07 R7).
package httpx

import (
	"encoding/json"
	"log/slog"
	"net/http"
)

// WriteJSON writes v as a JSON response. A (rare) encode failure is logged rather than
// silently swallowed (PLAN-07 R7/R11): the header is already sent so the status cannot
// change, but the failure must be visible instead of vanishing.
func WriteJSON(w http.ResponseWriter, v any) {
	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(v); err != nil {
		slog.Warn("writeJSON encode failed", "err", err)
	}
}
