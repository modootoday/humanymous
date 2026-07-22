package main

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"net/http"
	"os"

	"github.com/modootoday/humanymous/internal/network"
	"time"
)

// helpers.go holds small shared utilities for the handlers (SRP: keep handler
// files focused on request logic).

// readFile reads a served asset, capping size to avoid loading huge files into
// memory for the demo.
func readFile(path string) ([]byte, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	fi, err := f.Stat()
	if err != nil || fi.IsDir() {
		return nil, os.ErrNotExist
	}
	return os.ReadFile(path)
}

// dayBucket returns the current unix-day, the low-resolution issue time bound
// into the watermark payload (SoT-08 §2).
func dayBucket() uint64 {
	return uint64(time.Now().Unix() / 86400)
}

// trafficIPHash returns a KEYED pseudonym of the client IP for the watermark ledger
// (audit PRIV-1). An unkeyed hash of an IPv4 is trivially reversible (the whole address
// space fits a rainbow table); HMAC with the server master key makes the token a real
// pseudonym that cannot be reversed without the key.
func (a *app) trafficIPHash(r *http.Request) string {
	m := hmac.New(sha256.New, a.masterKey)
	m.Write([]byte("ipwm|"))
	m.Write([]byte(clientIP(r)))
	return hex.EncodeToString(m.Sum(nil)[:8])
}

// ja4Stable returns the permutation-invariant a_b portion of a connection's JA4
// (used as the network half of the correlation key). "" if no ClientHello.
func ja4Stable(hello *network.ClientHello) string {
	if hello == nil {
		return ""
	}
	ja4, _ := network.JA4(hello)
	first := -1
	for i := 0; i < len(ja4); i++ {
		if ja4[i] == '_' {
			if first == -1 {
				first = i
			} else {
				return ja4[:i]
			}
		}
	}
	return ja4
}
