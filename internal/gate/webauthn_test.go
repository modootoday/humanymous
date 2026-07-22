package gate

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"testing"
)

// webauthn_test.go verifies WebAuthn assertion checking: a valid assertion from a
// registered credential verifies, a replayed one (non-advancing counter) is rejected,
// and unknown/tampered/absent are handled without a false accept.

const testCredID = "cred-abc"

func makeAuthData(counter uint32) []byte {
	d := make([]byte, 37)
	// rpIdHash[0:32] zeros, flags[32]=0x05 (UP|UV), signCount[33:37].
	d[32] = 0x05
	binary.BigEndian.PutUint32(d[33:37], counter)
	return d
}

func signAssertion(t *testing.T, priv *ecdsa.PrivateKey, credID string, counter uint32) string {
	t.Helper()
	authData := makeAuthData(counter)
	clientData := []byte(`{"type":"webauthn.get","challenge":"Y2g","origin":"https://example.com"}`)
	cdHash := sha256.Sum256(clientData)
	signed := sha256.Sum256(append(append([]byte(nil), authData...), cdHash[:]...))
	sig, err := ecdsa.SignASN1(rand.Reader, priv, signed[:])
	if err != nil {
		t.Fatal(err)
	}
	a := webauthnAssertion{
		CredentialID:   credID,
		AuthData:       base64.StdEncoding.EncodeToString(authData),
		ClientDataJSON: base64.StdEncoding.EncodeToString(clientData),
		Signature:      base64.StdEncoding.EncodeToString(sig),
	}
	j, _ := json.Marshal(a)
	return base64.StdEncoding.EncodeToString(j)
}

func webauthnDir(t *testing.T, credID string, pub *ecdsa.PublicKey) *WebAuthnRegistry {
	t.Helper()
	der, _ := x509.MarshalPKIXPublicKey(pub)
	spec := credID + " " + base64.RawURLEncoding.EncodeToString(der)
	r, err := NewWebAuthnRegistry(spec)
	if err != nil {
		t.Fatal(err)
	}
	return r
}

func TestWebAuthnVerified(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 1)); v != webauthnVerified {
		t.Fatalf("valid assertion from a registered credential must verify, got %d", v)
	}
}

func TestWebAuthnReplayRejected(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	assertion := signAssertion(t, priv, testCredID, 5)
	if v, _ := r.verify(assertion); v != webauthnVerified {
		t.Fatal("first use must verify")
	}
	// Replaying the SAME assertion (counter 5, not advanced) must be rejected.
	if v, _ := r.verify(assertion); v != webauthnInvalid {
		t.Fatalf("a replayed assertion (non-advancing counter) must be rejected, got %d", v)
	}
}

func TestWebAuthnUnknownCredential(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	other, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &other.PublicKey) // registers a DIFFERENT key
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 1)); v != webauthnInvalid {
		t.Fatalf("assertion whose signature does not match the registered key must be invalid, got %d", v)
	}
}

func TestWebAuthnNone(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(""); v != webauthnNone {
		t.Fatalf("no assertion must be webauthnNone, got %d", v)
	}
}

// PLAN-08 backlog: origin binding rejects an assertion minted for another site.
func TestWebAuthnOriginBinding(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	r.SetBinding("https://example.com", "") // signAssertion uses origin https://example.com
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 1)); v != webauthnVerified {
		t.Fatal("matching origin must verify")
	}
	r2 := webauthnDir(t, testCredID, &priv.PublicKey)
	r2.SetBinding("https://victim.example", "")
	if v, _ := r2.verify(signAssertion(t, priv, testCredID, 2)); v != webauthnInvalid {
		t.Fatalf("assertion for a different origin must be rejected, got %d", v)
	}
}
