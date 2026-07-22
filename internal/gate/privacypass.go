package gate

import (
	"crypto"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/binary"
	"encoding/hex"
	"encoding/pem"
	"strings"

	_ "crypto/sha512" // register SHA-384 for rsa.VerifyPSS
)

// privacypass.go verifies Privacy Pass Private Access Tokens (RFC 9578 token type
// 0x0002, Blind RSA) at the gate (PLAN-08 R2). A PAT is a privacy-preserving
// proof-of-humanity: a client redeems, at an issuer, a token whose authenticator is
// an RSABSSA blind signature over the token structure. The blinding is the ISSUER's
// privacy property (it never sees the token it signs); to the RELYING PARTY the
// unblinded authenticator is simply a valid RSA-PSS signature, so verification here
// is stdlib crypto/rsa — no blind-signature code, no third-party client.
//
// A valid token from a trusted issuer is a trust signal (the redeemer solved the
// issuer's human/attestation challenge); an absent or invalid token is simply not a
// signal (never a deny — PAT is opt-in proof, not an accusation). The operator
// configures the trusted issuer public keys; utility depends on having a real issuer
// relationship, which is why the blueprint deferred this — the verifier itself is
// complete and correct.

// patTokenType is the RFC 9578 Blind-RSA token type.
const patTokenType = 0x0002

// PATVerifier holds the trusted issuer keys, indexed by token_key_id (the SHA-256 of
// the issuer's SubjectPublicKeyInfo DER, as carried in the token).
type PATVerifier struct {
	issuers map[string]*rsa.PublicKey // hex(token_key_id) -> issuer public key
}

// NewPATVerifier parses PEM-encoded RSA public keys (one or more "PUBLIC KEY" blocks)
// into a verifier, computing each key's RFC 9578 token_key_id.
func NewPATVerifier(pemBytes []byte) (*PATVerifier, error) {
	v := &PATVerifier{issuers: map[string]*rsa.PublicKey{}}
	rest := pemBytes
	for {
		var block *pem.Block
		block, rest = pem.Decode(rest)
		if block == nil {
			break
		}
		pub, err := x509.ParsePKIXPublicKey(block.Bytes)
		if err != nil {
			continue
		}
		rp, ok := pub.(*rsa.PublicKey)
		if !ok {
			continue
		}
		v.issuers[patKeyID(rp)] = rp
	}
	return v, nil
}

// patKeyID computes token_key_id = SHA-256(SubjectPublicKeyInfo DER), hex-encoded.
func patKeyID(pub *rsa.PublicKey) string {
	spki, err := x509.MarshalPKIXPublicKey(pub)
	if err != nil {
		return ""
	}
	sum := sha256.Sum256(spki)
	return hex.EncodeToString(sum[:])
}

// patVerdict is the outcome of verifying a Private Access Token.
type patVerdict int

const (
	patNone     patVerdict = iota // no token present
	patVerified                   // valid token from a trusted issuer
	patInvalid                    // token present but not verifiable / unknown issuer
)

// verifyPrivateToken parses and verifies a base64 PAT against the trusted issuers.
// Returns the verdict and the issuer key id (for audit).
func (v *PATVerifier) verifyPrivateToken(header string) (patVerdict, string) {
	tok := parsePrivateTokenHeader(header)
	if tok == "" {
		return patNone, ""
	}
	raw, err := base64.StdEncoding.DecodeString(tok)
	if err != nil {
		raw, err = base64.RawStdEncoding.DecodeString(tok)
		if err != nil {
			return patInvalid, ""
		}
	}
	// struct { uint16 type; nonce[32]; challenge_digest[32]; token_key_id[32]; authenticator[Nk] }
	const prefix = 2 + 32 + 32 + 32
	if len(raw) <= prefix {
		return patInvalid, ""
	}
	if binary.BigEndian.Uint16(raw[0:2]) != patTokenType {
		return patInvalid, ""
	}
	keyID := hex.EncodeToString(raw[66:98])
	pub, ok := v.issuers[keyID]
	if !ok {
		return patInvalid, keyID // unknown issuer: cannot verify, no trust
	}
	tokenInput := raw[:prefix] // everything the authenticator signs
	authenticator := raw[prefix:]
	sum := sha384Sum(tokenInput)
	// SaltLengthAuto accepts both the randomized (salt=hashLen) and deterministic
	// (salt=0) RSABSSA variants, so the verifier is agnostic to the issuer's choice.
	if rsa.VerifyPSS(pub, crypto.SHA384, sum, authenticator,
		&rsa.PSSOptions{SaltLength: rsa.PSSSaltLengthAuto, Hash: crypto.SHA384}) == nil {
		return patVerified, keyID
	}
	return patInvalid, keyID
}

// parsePrivateTokenHeader extracts the token from `PrivateToken token="<b64>"`
// (RFC 9577 Authorization header), tolerating the bare-base64 form too.
func parsePrivateTokenHeader(h string) string {
	h = strings.TrimSpace(h)
	if h == "" {
		return ""
	}
	if i := strings.Index(h, "token="); i >= 0 {
		val := strings.TrimSpace(h[i+len("token="):])
		val = strings.Trim(val, `"`)
		if c := strings.IndexAny(val, `,; `); c >= 0 {
			val = val[:c]
		}
		return val
	}
	if strings.HasPrefix(h, "PrivateToken") {
		return "" // header present but malformed
	}
	return h // bare token
}

// sha384Sum returns the SHA-384 digest of b.
func sha384Sum(b []byte) []byte {
	h := crypto.SHA384.New()
	h.Write(b)
	return h.Sum(nil)
}
