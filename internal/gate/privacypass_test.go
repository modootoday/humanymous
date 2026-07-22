package gate

import (
	"crypto"
	"crypto/rand"
	"crypto/rsa"
	"crypto/x509"
	"encoding/base64"
	"encoding/binary"
	"encoding/pem"
	"testing"
)

// privacypass_test.go verifies the RFC 9578 Private Access Token path: a valid token
// from a trusted issuer verifies, a tampered authenticator and an unknown issuer are
// rejected, and an absent token is a no-op. The mock issuer PSS-signs the token input
// directly — the unblinded RSABSSA authenticator IS a valid RSA-PSS signature.

func mintPAT(t *testing.T, pub *rsa.PublicKey, priv *rsa.PrivateKey) string {
	t.Helper()
	raw := make([]byte, 0, 98+256)
	raw = binary.BigEndian.AppendUint16(raw, patTokenType)
	raw = append(raw, make([]byte, 32)...) // nonce (zeros for the test)
	raw = append(raw, make([]byte, 32)...) // challenge_digest
	kid, _ := x509HexKeyID(pub)
	raw = append(raw, kid...) // token_key_id (32 bytes)
	tokenInput := append([]byte(nil), raw...)
	sig, err := rsa.SignPSS(rand.Reader, priv, crypto.SHA384, sha384Sum(tokenInput),
		&rsa.PSSOptions{SaltLength: rsa.PSSSaltLengthEqualsHash, Hash: crypto.SHA384})
	if err != nil {
		t.Fatalf("sign: %v", err)
	}
	raw = append(raw, sig...)
	return `PrivateToken token="` + base64.StdEncoding.EncodeToString(raw) + `"`
}

// x509HexKeyID returns the 32-byte token_key_id (SHA-256 of the SPKI DER).
func x509HexKeyID(pub *rsa.PublicKey) ([]byte, error) {
	spki, err := x509.MarshalPKIXPublicKey(pub)
	if err != nil {
		return nil, err
	}
	sum := sha256Sum(spki)
	return sum, nil
}

func pubPEM(t *testing.T, pub *rsa.PublicKey) []byte {
	t.Helper()
	der, _ := x509.MarshalPKIXPublicKey(pub)
	return pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der})
}

func TestPATVerified(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	if got, _ := v.verifyPrivateToken(hdr); got != patVerified {
		t.Fatalf("a valid PAT from the trusted issuer must verify, got %d", got)
	}
}

func TestPATTampered(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	// Flip a byte inside the base64 token → the RSA-PSS signature must fail.
	raw, _ := base64.StdEncoding.DecodeString(hdr[len(`PrivateToken token="`) : len(hdr)-1])
	raw[len(raw)-1] ^= 0xff
	tampered := `PrivateToken token="` + base64.StdEncoding.EncodeToString(raw) + `"`
	if got, _ := v.verifyPrivateToken(tampered); got != patInvalid {
		t.Fatalf("a tampered PAT must be patInvalid, got %d", got)
	}
}

func TestPATUnknownIssuer(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	other, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &other.PublicKey)) // trusts a DIFFERENT issuer
	hdr := mintPAT(t, &priv.PublicKey, priv)
	if got, _ := v.verifyPrivateToken(hdr); got != patInvalid {
		t.Fatalf("a token from an untrusted issuer must be patInvalid, got %d", got)
	}
}

func TestPATNone(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	if got, _ := v.verifyPrivateToken(""); got != patNone {
		t.Fatalf("no token must be patNone, got %d", got)
	}
}

// sha256Sum is a small helper mirroring the verifier's key-id hashing.
func sha256Sum(b []byte) []byte {
	h := crypto.SHA256.New()
	h.Write(b)
	return h.Sum(nil)
}
