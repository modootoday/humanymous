package gate

import (
	"crypto"
	"crypto/ecdsa"
	"crypto/ed25519"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/rsa"
	"encoding/base64"
	"encoding/binary"
	"net"
	"strconv"
	"strings"
	"testing"
	"time"
)

// Axis B (r201+): credential / edge-trust planes — Privacy Pass (RFC 9578),
// Web Bot Auth (RFC 9421), WebAuthn origin+counter, PROXY v2 trust boundary.
//
// Web research grounding (series notes):
// - Privacy Pass: origins MUST implement double-spend prevention on token nonce
//   (IETF privacypass issuance / rate-limit drafts).
// - PROXY protocol: metadata transport not auth; accept only from trusted upstreams
//   (APNIC 2025 PROXY security study; maskproxy trust-boundary guidance).
// - WebAuthn: RP MUST validate origin; counter replay / phishing transcript risks
//   (W3C WebAuthn; origin-binding guidance).

// --- Privacy Pass / Private Access Tokens ---

func TestWargameR201_PATDoubleSpendNonce(t *testing.T) {
	// Web: double-spend prevention required for PAT redemption.
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	if got, _ := v.verifyPrivateToken(hdr); got != patVerified {
		t.Fatal("first redeem")
	}
	if got, _ := v.verifyPrivateToken(hdr); got != patInvalid {
		t.Fatalf("replay same nonce must patInvalid, got %d", got)
	}
}

func TestWargameR202_PATTamperedAuthenticator(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	raw, _ := base64.StdEncoding.DecodeString(hdr[len(`PrivateToken token="`) : len(hdr)-1])
	raw[len(raw)-1] ^= 0xaa
	tampered := `PrivateToken token="` + base64.StdEncoding.EncodeToString(raw) + `"`
	if got, _ := v.verifyPrivateToken(tampered); got != patInvalid {
		t.Fatalf("got %d", got)
	}
}

func TestWargameR203_PATUnknownIssuerNoTrust(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	other, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &other.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	if got, _ := v.verifyPrivateToken(hdr); got != patInvalid {
		t.Fatalf("unknown issuer must not upgrade, got %d", got)
	}
}

func TestWargameR204_PATAbsentIsNoneNotDeny(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	if got, _ := v.verifyPrivateToken(""); got != patNone {
		t.Fatalf("absent PAT is opt-in signal only, got %d", got)
	}
}

func TestWargameR205_PATWrongTokenType(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	raw := make([]byte, 0, 98+256)
	raw = binary.BigEndian.AppendUint16(raw, 0x0001) // not Blind RSA 0x0002
	raw = append(raw, make([]byte, 32+32)...)
	kid, _ := x509HexKeyID(&priv.PublicKey)
	raw = append(raw, kid...)
	tokenInput := append([]byte(nil), raw...)
	sig, _ := rsa.SignPSS(rand.Reader, priv, crypto.SHA384, sha384Sum(tokenInput),
		&rsa.PSSOptions{SaltLength: rsa.PSSSaltLengthEqualsHash, Hash: crypto.SHA384})
	raw = append(raw, sig...)
	hdr := `PrivateToken token="` + base64.StdEncoding.EncodeToString(raw) + `"`
	if got, _ := v.verifyPrivateToken(hdr); got != patInvalid {
		t.Fatalf("wrong token type must invalid, got %d", got)
	}
}

func TestWargameR206_PATMalformedPrivateTokenHeader(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	// Present scheme but no token= parameter
	if got, _ := v.verifyPrivateToken("PrivateToken challenge=xyz"); got != patNone && got != patInvalid {
		t.Fatalf("malformed PrivateToken must not verify, got %d", got)
	}
	if got, _ := v.verifyPrivateToken("PrivateToken challenge=xyz"); got == patVerified {
		t.Fatal("must not patVerified")
	}
}

func TestWargameR207_PATDistinctNoncesIndependent(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	h1 := mintPATWithNonce(t, &priv.PublicKey, priv, byte(1))
	h2 := mintPATWithNonce(t, &priv.PublicKey, priv, byte(2))
	if got, _ := v.verifyPrivateToken(h1); got != patVerified {
		t.Fatal("n1")
	}
	if got, _ := v.verifyPrivateToken(h2); got != patVerified {
		t.Fatal("n2 distinct nonce must still verify")
	}
}

func TestWargameR208_PATBareBase64Form(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	// strip PrivateToken wrapper → bare b64 path
	raw := hdr[len(`PrivateToken token="`) : len(hdr)-1]
	if got, _ := v.verifyPrivateToken(raw); got != patVerified {
		t.Fatalf("bare token form must verify, got %d", got)
	}
}

// --- PROXY protocol trust boundary (web: accept only trusted upstreams) ---

func TestWargameR209_ProxyParseRejectsWorldCIDR(t *testing.T) {
	if _, err := ParseCIDRs("0.0.0.0/0"); err == nil {
		t.Fatal("/0 must fail-closed (any client could spoof PROXY)")
	}
}

func TestWargameR210_ProxyParseRejectsV6World(t *testing.T) {
	if _, err := ParseCIDRs("::/0"); err == nil {
		t.Fatal("IPv6 /0 must fail-closed")
	}
}

func TestWargameR211_ProxyParseRejectsNearWorld(t *testing.T) {
	if _, err := ParseCIDRs("10.0.0.0/4"); err == nil {
		t.Fatal("/4 too broad")
	}
}

func TestWargameR212_ProxyParseAcceptsBalancerSlash24(t *testing.T) {
	nets, err := ParseCIDRs("172.40.0.0/24")
	if err != nil || len(nets) != 1 {
		t.Fatalf("%v %d", err, len(nets))
	}
}

func TestWargameR213_ProxyUntrustedPeerIgnoresHeader(t *testing.T) {
	// Covered structurally by TestProxyListenerTrustGate — pin ParseCIDRs single IP expand
	nets, err := ParseCIDRs("203.0.113.10")
	if err != nil || len(nets) != 1 {
		t.Fatal(err)
	}
	if ones, _ := nets[0].Mask.Size(); ones != 32 {
		t.Fatalf("bare IP should become /32, got /%d", ones)
	}
}

func TestWargameR214_ProxyV2GarbageNotHeader(t *testing.T) {
	// re-run contract under wargame id
	cli, srv := net.Pipe()
	defer cli.Close()
	defer srv.Close()
	go func() { _, _ = cli.Write([]byte("GET / HTTP/1.1\r\n\r\n")) }()
	if _, err := readProxyV2(srv, 2*time.Second); err == nil {
		t.Fatal("HTTP bytes must not parse as PROXY v2")
	}
}

func TestWargameR215_ProxyV2LocalNoSpoofAddr(t *testing.T) {
	cli, srv := net.Pipe()
	defer cli.Close()
	defer srv.Close()
	hdr := append(append([]byte{}, proxyV2Sig...), 0x20, 0x00, 0x00, 0x00)
	go func() { _, _ = cli.Write(hdr) }()
	addr, err := readProxyV2(srv, 2*time.Second)
	if err != nil || addr != nil {
		t.Fatalf("LOCAL => (nil,nil), got (%v,%v)", addr, err)
	}
}

// --- WebAuthn origin + counter (web: origin binding / replay) ---

func TestWargameR216_WebAuthnOriginMismatch(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	r.SetBinding("https://victim.example", "")
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 3)); v != webauthnInvalid {
		t.Fatalf("phishing-origin assertion must invalid, got %d", v)
	}
}

func TestWargameR217_WebAuthnZeroCounterNoUpgrade(t *testing.T) {
	// Web research: zero/non-incrementing counters cannot replay-protect without challenge.
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 0)); v != webauthnInvalid {
		t.Fatalf("counter=0 must not trust-upgrade, got %d", v)
	}
}

func TestWargameR218_WebAuthnCounterReplay(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	a := signAssertion(t, priv, testCredID, 9)
	if v, _ := r.verify(a); v != webauthnVerified {
		t.Fatal("first")
	}
	if v, _ := r.verify(a); v != webauthnInvalid {
		t.Fatalf("replay, got %d", v)
	}
}

func TestWargameR219_WebAuthnCounterMustStrictlyIncrease(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 10)); v != webauthnVerified {
		t.Fatal("c10")
	}
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 10)); v != webauthnInvalid {
		t.Fatal("same counter")
	}
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 9)); v != webauthnInvalid {
		t.Fatal("decreasing counter")
	}
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 11)); v != webauthnVerified {
		t.Fatal("advance ok")
	}
}

func TestWargameR220_WebAuthnRpIDMismatch(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	r.SetBinding("https://example.com", "example.com")
	// makeAuthData uses zero rpIdHash — will not match SHA256(example.com)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 4)); v != webauthnInvalid {
		t.Fatalf("rpIdHash mismatch must invalid, got %d", v)
	}
}

func TestWargameR221_WebAuthnNoneAbsent(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(""); v != webauthnNone {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR222_WebAuthnUnknownCred(t *testing.T) {
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	other, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &other.PublicKey)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 5)); v != webauthnInvalid {
		t.Fatalf("got %d", v)
	}
}

// --- Web Bot Auth / RFC 9421 ---

func TestWargameR223_WBAForgedAllowlistedKeyID(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)
	_, att, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, att, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR224_WBAAuthorityBinding(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, priv, now.Unix(), "n1")
	if v, _ := verifyAgent("evil.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR225_WBAPathBinding(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/public", testKID, priv, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "GET", "/admin", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR226_WBAMethodBinding(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/x", testKID, priv, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "POST", "/x", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR227_WBANonceSingleUse(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	nc := NewNonceCache(time.Hour)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, priv, now.Unix(), "nonce-once")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, nc, now); v != agentVerifiedTrusted {
		t.Fatal("first")
	}
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, nc, now); v != agentVerifiedUnknown {
		t.Fatalf("nonce replay must not re-upgrade, got %d", v)
	}
}

func TestWargameR228_WBAAuthorityOnlyNotTrusted(t *testing.T) {
	// Web: covering only @authority allows path/method replay — product requires full request line.
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// Craft signature covering only @authority
	inner := `("@authority");created=` + itoa64(now.Unix()) + `;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := "\"@authority\": example.com\n\"@signature-params\": " + inner
	sig := ed25519.Sign(priv, []byte(base))
	si := "sig1=" + inner
	sv := "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/admin", si, sv, dir, NewNonceCache(time.Hour), now); v == agentVerifiedTrusted {
		t.Fatal("@authority-only must not trust-upgrade")
	}
}

func TestWargameR229_WBANoCreatedExpiresNeutral(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	inner := `("@authority" "@method" "@path");keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", "GET", "/data", wbaComponents, inner)
	sig := ed25519.Sign(priv, []byte(base))
	si := "sig1=" + inner
	sv := "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sv, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("unbounded lifetime must be neutral, got %d", v)
	}
}

func TestWargameR230_WBAOverlongExpiresNeutral(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	// expires 2h in the future (> maxLifetime 1h remaining)
	exp := now.Unix() + 7200
	inner := `("@authority" "@method" "@path");created=` + itoa64(now.Unix()) +
		`;expires=` + itoa64(exp) + `;keyid="` + testKID + `";alg="ed25519";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", "GET", "/data", wbaComponents, inner)
	sig := ed25519.Sign(priv, []byte(base))
	si := "sig1=" + inner
	sv := "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sv, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown {
		t.Fatalf("over-long lifetime must be neutral, got %d", v)
	}
}

func TestWargameR231_WBAUnknownKeyNeutralNotDeny(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	// empty directory
	dir, _ := NewStaticKeyDirectory("")
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", "unknown-kid", priv, now.Unix(), "n1")
	_ = pub
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentVerifiedUnknown && v != agentNone {
		// unknown key → not trusted, not forged
		if v == agentVerifiedTrusted || v == agentForged {
			t.Fatalf("unknown key must be neutral, got %d", v)
		}
	}
}

func TestWargameR232_WBAAbsentSignatureNone(t *testing.T) {
	dir, _ := NewStaticKeyDirectory("")
	now := time.Unix(1_700_000_000, 0)
	if v, _ := verifyAgent("example.com", "GET", "/", "", "", dir, nil, now); v != agentNone {
		t.Fatalf("got %d", v)
	}
}

func TestWargameR233_WBAWrongAlgNeutral(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	inner := `("@authority" "@method" "@path");created=` + itoa64(now.Unix()) +
		`;keyid="` + testKID + `";alg="rsa-pss-sha512";tag="web-bot-auth"`
	base := buildSignatureBase("example.com", "GET", "/data", wbaComponents, inner)
	// still ed25519 sign but alg claims rsa → profile gate
	sig := ed25519.Sign(priv, []byte(base))
	si := "sig1=" + inner
	sv := "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sv, dir, NewNonceCache(time.Hour), now); v == agentVerifiedTrusted {
		t.Fatal("non-ed25519 alg must not trust")
	}
}

func TestWargameR234_WBAWrongTagNeutral(t *testing.T) {
	pub, priv, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	inner := `("@authority" "@method" "@path");created=` + itoa64(now.Unix()) +
		`;keyid="` + testKID + `";alg="ed25519";tag="other-profile"`
	base := buildSignatureBase("example.com", "GET", "/data", wbaComponents, inner)
	sig := ed25519.Sign(priv, []byte(base))
	si := "sig1=" + inner
	sv := "sig1=:" + base64.StdEncoding.EncodeToString(sig) + ":"
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sv, dir, NewNonceCache(time.Hour), now); v == agentVerifiedTrusted {
		t.Fatal("wrong tag must not trust")
	}
}

// --- compositions across axes ---

func TestWargameR235_PATDoesNotLaunderWBAForgery(t *testing.T) {
	// Orthogonal signals: PAT verified must not change agentForged classification.
	privRSA, _ := rsa.GenerateKey(rand.Reader, 2048)
	pv, _ := NewPATVerifier(pubPEM(t, &privRSA.PublicKey))
	hdr := mintPAT(t, &privRSA.PublicKey, privRSA)
	if got, _ := pv.verifyPrivateToken(hdr); got != patVerified {
		t.Fatal("pat")
	}
	pub, _, _ := ed25519.GenerateKey(nil)
	_, att, _ := ed25519.GenerateKey(nil)
	dir := testDir(t, testKID, pub)
	now := time.Unix(1_700_000_000, 0)
	si, sig := signWebBotAuth("example.com", "GET", "/data", testKID, att, now.Unix(), "n1")
	if v, _ := verifyAgent("example.com", "GET", "/data", si, sig, dir, NewNonceCache(time.Hour), now); v != agentForged {
		t.Fatal("WBA forgery independent of PAT")
	}
}

func TestWargameR236_WebAuthnDoesNotBypassProxyTrustConfig(t *testing.T) {
	// Composition honesty: credential planes do not weaken PROXY /0 reject.
	if _, err := ParseCIDRs("0.0.0.0/0"); err == nil {
		t.Fatal("still reject world trust")
	}
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &priv.PublicKey)
	if v, _ := r.verify(signAssertion(t, priv, testCredID, 7)); v != webauthnVerified {
		t.Fatal("webauthn still works independently")
	}
}

func TestWargameR237_CoversRequestLineHelper(t *testing.T) {
	if coversRequestLine([]string{"@authority"}) {
		t.Fatal("authority only")
	}
	if !coversRequestLine([]string{"@authority", "@method", "@path"}) {
		t.Fatal("full set")
	}
	if coversRequestLine([]string{"@authority", "@method", "@path", "content-digest"}) {
		t.Fatal("extra unverifiable component")
	}
}

func TestWargameR238_PATHeaderInjectionAfterToken(t *testing.T) {
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	// trailing junk after quoted token
	junked := strings.TrimSuffix(hdr, `"`) + `"; evil=1`
	// parsePrivateTokenHeader truncates at ; — may still decode
	got, _ := v.verifyPrivateToken(junked)
	if got != patVerified && got != patInvalid {
		t.Fatalf("must not panic; got %d", got)
	}
}

func TestWargameR239_StaticKeyDirectoryIgnoresBadLines(t *testing.T) {
	pub, _, _ := ed25519.GenerateKey(nil)
	spec := "# comment\n\nbadline\n" + testKID + " " + base64.RawURLEncoding.EncodeToString(pub) + "\n"
	d, err := NewStaticKeyDirectory(spec)
	if err != nil {
		t.Fatal(err)
	}
	if _, ok := d.Lookup(testKID); !ok {
		t.Fatal("good line must load")
	}
}

func TestWargameR240_AxisCloseRegression(t *testing.T) {
	// Lock: double-spend + PROXY /0 + zero counter + authority-only WBA
	priv, _ := rsa.GenerateKey(rand.Reader, 2048)
	v, _ := NewPATVerifier(pubPEM(t, &priv.PublicKey))
	hdr := mintPAT(t, &priv.PublicKey, priv)
	_, _ = v.verifyPrivateToken(hdr)
	if got, _ := v.verifyPrivateToken(hdr); got != patInvalid {
		t.Fatal("PAT double-spend lock")
	}
	if _, err := ParseCIDRs("0.0.0.0/0"); err == nil {
		t.Fatal("PROXY /0 lock")
	}
	ec, _ := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	r := webauthnDir(t, testCredID, &ec.PublicKey)
	if v, _ := r.verify(signAssertion(t, ec, testCredID, 0)); v != webauthnInvalid {
		t.Fatal("zero counter lock")
	}
	if coversRequestLine([]string{"@authority"}) {
		t.Fatal("WBA request-line lock")
	}
}

// --- helpers for this axis ---

func mintPATWithNonce(t *testing.T, pub *rsa.PublicKey, priv *rsa.PrivateKey, nonceByte byte) string {
	t.Helper()
	raw := make([]byte, 0, 98+256)
	raw = binary.BigEndian.AppendUint16(raw, patTokenType)
	nonce := make([]byte, 32)
	nonce[0] = nonceByte
	raw = append(raw, nonce...)
	raw = append(raw, make([]byte, 32)...) // challenge_digest
	kid, _ := x509HexKeyID(pub)
	raw = append(raw, kid...)
	tokenInput := append([]byte(nil), raw...)
	sig, err := rsa.SignPSS(rand.Reader, priv, crypto.SHA384, sha384Sum(tokenInput),
		&rsa.PSSOptions{SaltLength: rsa.PSSSaltLengthEqualsHash, Hash: crypto.SHA384})
	if err != nil {
		t.Fatal(err)
	}
	raw = append(raw, sig...)
	return `PrivateToken token="` + base64.StdEncoding.EncodeToString(raw) + `"`
}

func itoa64(n int64) string { return strconv.FormatInt(n, 10) }
