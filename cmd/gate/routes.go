package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

// routes.go loads the enforcement route table from an external file (SoT-31 R2),
// so an adopter maps THEIR sensitive paths without recompiling the binary. The
// resolver (internal/gate/config.go) already accepts an arbitrary
// prefix -> preset map; this just fills it from disk.
//
// Format: one `<path-prefix> <preset>` per line; `#` comments and blank lines are
// ignored; preset is one of strict | balanced | attested | monitor | off.
//
// `attested` (ceiling-guard #1) is the attestation floor: reserve it for high-value
// MUTATING routes (POST /checkout, /transfer, /password, /api/keys). It is REFUSED on
// a catch-all prefix (`/` or empty) — flooring an entire site's browse traffic to Pass
// is a misconfiguration the ceiling-guard meeting explicitly rules out.

var validPresets = map[string]bool{"strict": true, "balanced": true, "attested": true, "monitor": true, "off": true}

// loadRoutes parses a routes file into a prefix -> preset map.
func loadRoutes(path string) (map[string]string, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	routes := map[string]string{}
	sc := bufio.NewScanner(f)
	ln := 0
	for sc.Scan() {
		ln++
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) != 2 {
			return nil, fmt.Errorf("routes %s:%d: expected `<prefix> <preset>`, got %q", path, ln, line)
		}
		prefix, preset := fields[0], fields[1]
		if !validPresets[preset] {
			return nil, fmt.Errorf("routes %s:%d: unknown preset %q (want strict|balanced|attested|monitor|off)", path, ln, preset)
		}
		// Ceiling-guard #1: the attestation floor must never blanket a whole site. Refuse
		// it on a catch-all prefix so an operator cannot accidentally Pass-wall all browse
		// traffic (the meeting's "refuse attestFloor on public/catch-all routes").
		if preset == "attested" && (prefix == "/" || prefix == "") {
			return nil, fmt.Errorf("routes %s:%d: preset \"attested\" is refused on the catch-all prefix %q — mark specific high-value routes (e.g. /checkout), not the whole site", path, ln, prefix)
		}
		routes[prefix] = preset
	}
	if err := sc.Err(); err != nil {
		return nil, err
	}
	return routes, nil
}
