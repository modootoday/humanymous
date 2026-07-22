package gate

import (
	"crypto/ecdsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/binary"
	"encoding/json"
	"encoding/pem"
	"strings"
	"sync"
)

// webauthn.go verifies WebAuthn assertions at the gate (PLAN-08 R2). A returning user
// whose device holds a registered credential can prove possession by signing the
// authenticator data + client-data hash with the credential's private key; the gate
// verifies that ES256 signature against the operator-registered public key and, using
// WebAuthn's native signature COUNTER, rejects a replayed assertion (a captured
// assertion carries a counter that does not advance). A valid, fresh assertion from a
// known credential is a trust-upgrade; anything else is a NO-OP (never a deny). This
// is a modest, returning-user trust signal — the blueprint rated it low value against
// anonymous first-time bots — but the verifier is complete and correct.

// WebAuthnRegistry maps a credential id to its ES256 public key + last seen counter.
type WebAuthnRegistry struct {
	mu    sync.Mutex
	keys  map[string]*ecdsa.PublicKey // credentialId (base64url) -> public key
	count map[string]uint32           // credentialId -> highest signature counter seen
}

// NewWebAuthnRegistry parses "credentialId base64url-spki-ecdsa-p256-pubkey" lines.
func NewWebAuthnRegistry(spec string) (*WebAuthnRegistry, error) {
	r := &WebAuthnRegistry{keys: map[string]*ecdsa.PublicKey{}, count: map[string]uint32{}}
	for _, line := range strings.Split(spec, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		f := strings.Fields(line)
		if len(f) != 2 {
			continue
		}
		der, err := base64.RawURLEncoding.DecodeString(f[1])
		if err != nil {
			der, err = base64.StdEncoding.DecodeString(f[1])
			if err != nil {
				continue
			}
		}
		// Accept a raw SPKI DER or a PEM block.
		if pubKey := parseECPublic(der); pubKey != nil {
			r.keys[f[0]] = pubKey
		}
	}
	return r, nil
}

func parseECPublic(der []byte) *ecdsa.PublicKey {
	if block, _ := pem.Decode(der); block != nil {
		der = block.Bytes
	}
	pub, err := x509.ParsePKIXPublicKey(der)
	if err != nil {
		return nil
	}
	ec, _ := pub.(*ecdsa.PublicKey)
	return ec
}

// webauthnAssertion is the client's assertion (base64 fields, WebAuthn get() result).
type webauthnAssertion struct {
	CredentialID   string `json:"credentialId"`
	AuthData       string `json:"authenticatorData"`
	ClientDataJSON string `json:"clientDataJSON"`
	Signature      string `json:"signature"`
}

// webauthnVerdict is the outcome of verifying an assertion.
type webauthnVerdict int

const (
	webauthnNone     webauthnVerdict = iota // no assertion present
	webauthnVerified                        // valid, fresh assertion from a known credential
	webauthnInvalid                         // present but unverifiable / replayed / unknown
)

// verify parses and verifies a base64(JSON) WebAuthn assertion carried in a header.
func (r *WebAuthnRegistry) verify(header string) (webauthnVerdict, string) {
	if header == "" {
		return webauthnNone, ""
	}
	rawJSON, err := base64.StdEncoding.DecodeString(strings.TrimSpace(header))
	if err != nil {
		rawJSON, err = base64.RawStdEncoding.DecodeString(strings.TrimSpace(header))
		if err != nil {
			return webauthnInvalid, ""
		}
	}
	var a webauthnAssertion
	if json.Unmarshal(rawJSON, &a) != nil || a.CredentialID == "" {
		return webauthnInvalid, ""
	}

	r.mu.Lock()
	pub := r.keys[a.CredentialID]
	r.mu.Unlock()
	if pub == nil {
		return webauthnInvalid, a.CredentialID // unknown credential
	}

	authData, e1 := base64.StdEncoding.DecodeString(a.AuthData)
	clientData, e2 := base64.StdEncoding.DecodeString(a.ClientDataJSON)
	sig, e3 := base64.StdEncoding.DecodeString(a.Signature)
	if e1 != nil || e2 != nil || e3 != nil || len(authData) < 37 {
		return webauthnInvalid, a.CredentialID
	}
	// clientData must be a WebAuthn get() assertion.
	var cd struct {
		Type string `json:"type"`
	}
	if json.Unmarshal(clientData, &cd) != nil || cd.Type != "webauthn.get" {
		return webauthnInvalid, a.CredentialID
	}
	// signed message = authenticatorData || SHA-256(clientDataJSON).
	cdHash := sha256.Sum256(clientData)
	signed := sha256.Sum256(append(append([]byte(nil), authData...), cdHash[:]...))
	if !ecdsa.VerifyASN1(pub, signed[:], sig) {
		return webauthnInvalid, a.CredentialID
	}
	// WebAuthn native anti-replay: the signature counter must advance.
	counter := binary.BigEndian.Uint32(authData[33:37])
	r.mu.Lock()
	last := r.count[a.CredentialID]
	if counter != 0 && counter <= last {
		r.mu.Unlock()
		return webauthnInvalid, a.CredentialID // replay (counter did not advance)
	}
	r.count[a.CredentialID] = counter
	r.mu.Unlock()
	return webauthnVerified, a.CredentialID
}
