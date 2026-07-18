package network

import (
	"fmt"
	"sort"
	"strings"
)

// http2.go builds the Akamai HTTP/2 fingerprint and resolves an engine family
// from it (SoT-02 §HTTP/2). Format:
//
//	SETTINGS | WINDOW_UPDATE | PRIORITY | pseudo-header-order
//
// e.g. Chrome: 1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p

// H2Setting is one SETTINGS id:value pair in sent order.
type H2Setting struct {
	ID    uint16
	Value uint32
}

// H2Fingerprint is the structured HTTP/2 client fingerprint.
type H2Fingerprint struct {
	Settings     []H2Setting // in sent order
	WindowUpdate uint32      // first connection-level WINDOW_UPDATE increment
	Priorities   []string    // "streamID:excl:dep:weight" tuples
	PseudoOrder  []string    // subset/order of m,a,s,p
}

// Akamai renders the canonical pipe-separated Akamai HTTP/2 fingerprint string.
func (f H2Fingerprint) Akamai() string {
	var settings []string
	for _, s := range f.Settings {
		settings = append(settings, fmt.Sprintf("%d:%d", s.ID, s.Value))
	}
	prio := "0"
	if len(f.Priorities) > 0 {
		prio = strings.Join(f.Priorities, ",")
	}
	return strings.Join([]string{
		strings.Join(settings, ";"),
		fmt.Sprintf("%d", f.WindowUpdate),
		prio,
		strings.Join(f.PseudoOrder, ","),
	}, "|")
}

// PseudoString returns the pseudo-header order joined (e.g. "m,a,s,p").
func (f H2Fingerprint) PseudoString() string { return strings.Join(f.PseudoOrder, ",") }

// settingsMap returns the SETTINGS as an id->value map (order lost).
func (f H2Fingerprint) settingsMap() map[uint16]uint32 {
	m := map[uint16]uint32{}
	for _, s := range f.Settings {
		m[s.ID] = s.Value
	}
	return m
}

// EngineFromH2 resolves an engine family from the HTTP/2 fingerprint.
// Pseudo-header order alone separates the three browser engines; library
// defaults (Go/python) are caught via their SETTINGS profile (SoT-02 §2.2/2.3).
func EngineFromH2(f H2Fingerprint) string {
	switch f.PseudoString() {
	case "m,a,s,p":
		return EngineChrome
	case "m,p,a,s":
		return EngineFirefox
	case "m,s,p,a":
		return EngineSafari
	}
	// No/other pseudo order: inspect SETTINGS. Go's x/net/http2 defaults are a
	// small set with MAX_CONCURRENT_STREAMS present, unlike Chrome.
	m := f.settingsMap()
	_, hasMaxConc := m[3] // MAX_CONCURRENT_STREAMS
	if hasMaxConc && m[4] == 65535 {
		return EngineGo
	}
	if len(f.Settings) == 0 {
		return EngineUnknown
	}
	return EngineUnknown
}

// pseudoOrderCode maps a pseudo-header name to its Akamai abbreviation.
func pseudoOrderCode(name string) string {
	switch name {
	case ":method":
		return "m"
	case ":authority":
		return "a"
	case ":scheme":
		return "s"
	case ":path":
		return "p"
	}
	return ""
}

// SortedSettingIDs returns setting ids sorted (helper for stable comparison).
func (f H2Fingerprint) SortedSettingIDs() []int {
	ids := make([]int, 0, len(f.Settings))
	for _, s := range f.Settings {
		ids = append(ids, int(s.ID))
	}
	sort.Ints(ids)
	return ids
}
