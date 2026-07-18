package network

// enginemap.go resolves a browser-engine family from TLS/HTTP2 observations.
// This is the single source the L6 cross-check diffs against the claimed UA
// (SoT-02 §L6). A production system would use a curated JA4->engine database;
// for this educational sample we use documented structural heuristics and keep
// them isolated here so they can be swapped for a database later.

// Engine families.
const (
	EngineChrome  = "chrome"  // BoringSSL
	EngineFirefox = "firefox" // NSS
	EngineSafari  = "safari"  // WebKit/SecureTransport
	EngineGo      = "go"      // crypto/tls
	EngineOpenSSL = "openssl" // curl/python default
	EngineUnknown = "unknown"
)

// TLS extension type numbers referenced by the heuristics.
const (
	extRecordSizeLimit = 0x001c // Firefox-specific (Chrome lacks it)
	extALPS            = 0x4469 // application_settings (Chrome)
	extALPSOld         = 0x0a0a // (also a GREASE value; handled separately)
	extCompressCert    = 0x001b // compress_certificate (Chrome)
	extECH             = 0xfe0d // encrypted_client_hello (Chrome)
)

// hasGREASE reports whether any cipher or extension is a GREASE code point,
// which real Chromium/Firefox always inject and libraries never do.
func hasGREASE(ch *ClientHello) bool {
	for _, v := range ch.CipherSuites {
		if isGREASE(v) {
			return true
		}
	}
	for _, v := range ch.Extensions {
		if isGREASE(v) {
			return true
		}
	}
	return false
}

func hasExt(ch *ClientHello, want uint16) bool {
	for _, e := range ch.Extensions {
		if e == want {
			return true
		}
	}
	return false
}

// EngineFromClientHello resolves the engine family from TLS characteristics.
func EngineFromClientHello(ch *ClientHello) string {
	if ch == nil {
		return EngineUnknown
	}
	greased := hasGREASE(ch)

	// No GREASE => not a modern browser. Distinguish Go (very small cipher set,
	// no browser extensions) from generic OpenSSL libraries.
	if !greased {
		if len(stripGREASE(ch.CipherSuites)) <= 16 && !hasExt(ch, extCompressCert) {
			return EngineGo
		}
		return EngineOpenSSL
	}

	// GREASE present => browser family. Firefox uniquely offers record_size_limit.
	if hasExt(ch, extRecordSizeLimit) {
		return EngineFirefox
	}
	// Chrome offers compress_certificate / ECH / ALPS and no record_size_limit.
	if hasExt(ch, extCompressCert) || hasExt(ch, extECH) || hasExt(ch, extALPS) {
		return EngineChrome
	}
	// Safari: GREASE but none of the Chrome/Firefox markers (approximate).
	return EngineSafari
}
