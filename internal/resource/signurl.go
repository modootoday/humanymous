package resource

import (
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"strconv"
	"strings"
)

// signurl.go issues and verifies session-scoped, expiring signed URLs for heavy
// resources (SoT-10 §3.2). The signature binds sessionId + asset + expiry with
// the shared master key, so a leaked heavy-resource URL can't be replayed by
// another session or after expiry.

// Sign returns the signature token for a heavy-resource URL.
func Sign(masterKey []byte, sessionID, asset string, expiryUnix int64) string {
	mac := hmac.New(sha256.New, masterKey)
	mac.Write([]byte(sessionID))
	mac.Write([]byte{0})
	mac.Write([]byte(asset))
	mac.Write([]byte{0})
	mac.Write([]byte(strconv.FormatInt(expiryUnix, 10)))
	return base64.RawURLEncoding.EncodeToString(mac.Sum(nil)[:16])
}

// SignedURL builds "/res/<asset>?sid=<>&exp=<>&sig=<>".
func SignedURL(masterKey []byte, sessionID, asset string, expiryUnix int64) string {
	sig := Sign(masterKey, sessionID, asset, expiryUnix)
	var b strings.Builder
	b.WriteString("/res/")
	b.WriteString(strings.TrimPrefix(asset, "/"))
	b.WriteString("?sid=")
	b.WriteString(sessionID)
	b.WriteString("&exp=")
	b.WriteString(strconv.FormatInt(expiryUnix, 10))
	b.WriteString("&sig=")
	b.WriteString(sig)
	return b.String()
}

// Verify checks a signed-URL token in constant time and enforces expiry.
func Verify(masterKey []byte, sessionID, asset string, expiryUnix, nowUnix int64, sig string) bool {
	if nowUnix > expiryUnix {
		return false
	}
	want := Sign(masterKey, sessionID, asset, expiryUnix)
	return hmac.Equal([]byte(want), []byte(sig))
}
