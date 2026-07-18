package gate

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"sort"
)

// configsign.go binds a record's config_version to a signed hash of the running
// policy (SoT-28 WS8). A free-form label lets an operator silently run an
// unapproved policy while records claim an approved version; hashing the actual
// effective config under the server key makes every past verdict explainable
// against the exact policy in force, and makes an unapproved change detectable.

// configVersion is a deterministic keyed hash of the effective policy.
func (s *Server) configVersion() string {
	h := hmac.New(sha256.New, s.tokenKey)
	fmt.Fprintf(h, "monitor=%v|kill=%v", s.cfg.GlobalMonitor, s.killSwitch.Load())
	prefixes := make([]string, 0, len(s.cfg.Routes))
	for p := range s.cfg.Routes {
		prefixes = append(prefixes, p)
	}
	sort.Strings(prefixes)
	for _, p := range prefixes {
		fmt.Fprintf(h, "|%s=%s", p, s.cfg.Routes[p])
	}
	fmt.Fprintf(h, "|rl=%d/%d/%d", int(rlWindow(s.cfg).Seconds()), rlSoft(s.cfg), rlHard(s.cfg))
	return "cfg-" + hex.EncodeToString(h.Sum(nil)[:6])
}
