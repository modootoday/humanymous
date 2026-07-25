package gate

import (
	"crypto/ed25519"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/audit"
	"github.com/modootoday/humanymous/internal/collector"
	"github.com/modootoday/humanymous/internal/scoring"
)

// SoT-38 §12 / WS2: an auditor fetches published keys over the admin API and
// verifies exported chain material with the public key alone (HMAC nil).
func TestAdminKeysEnableOfflineVerify(t *testing.T) {
	up := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		w.Write([]byte("<html><head></head><body>ok</body></html>"))
	}))
	defer up.Close()

	wit := audit.NewWitness()
	alog := audit.NewLog(audit.Config{
		NodeID: "audit-node", HMACKey: []byte("offline-hmac-key"),
		CheckpointEvery: 4, Witness: wit,
	})
	sink := audit.NewSink(alog)
	vault := audit.NewVault()
	store := collector.NewStore(time.Minute)
	engine := scoring.NewEngine()
	verdicts := NewVerdictStore(time.Minute)
	control := NewControlPlane(store, engine, verdicts, sink, vault)
	srv, err := NewServer(Config{
		Upstream: up.URL, NodeID: "audit-node", ControlPath: "/__hmn/",
		RateWindow: time.Minute, RateSoft: 100, RateHard: 1000,
	}, sink, vault, verdicts, control.Handler())
	if err != nil {
		t.Fatal(err)
	}
	toks := seedAdmins(t, srv)

	// Seal enough decisions to force at least one STH.
	for i := 0; i < 8; i++ {
		r := httptest.NewRequest("GET", "http://p/", nil)
		r.RemoteAddr = "198.51.100.10:1"
		r.Header.Set("User-Agent", "Chrome/126")
		srv.ServeHTTP(httptest.NewRecorder(), r)
	}
	alog.Checkpoint()

	// 1) Publish surfaces: keys + checkpoints (no secrets).
	kw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/keys", "")
	if kw.Code != http.StatusOK {
		t.Fatalf("keys: %d %s", kw.Code, kw.Body.String())
	}
	var keys struct {
		STHPublicKey     string `json:"sth_public_key"`
		WitnessPublicKey string `json:"witness_public_key"`
	}
	if err := json.Unmarshal(kw.Body.Bytes(), &keys); err != nil {
		t.Fatalf("keys json: %v", err)
	}
	pubRaw, err := hex.DecodeString(keys.STHPublicKey)
	if err != nil || len(pubRaw) != ed25519.PublicKeySize {
		t.Fatalf("sth_public_key not a valid ed25519 hex: %q err=%v", keys.STHPublicKey, err)
	}
	if keys.WitnessPublicKey == "" {
		t.Fatal("witness_public_key should be published when a co-signer is configured")
	}

	cpw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/checkpoints", "")
	if cpw.Code != http.StatusOK {
		t.Fatalf("checkpoints: %d %s", cpw.Code, cpw.Body.String())
	}
	var cpBody struct {
		Checkpoints []audit.Checkpoint `json:"checkpoints"`
		Count       int                `json:"count"`
	}
	if err := json.Unmarshal(cpw.Body.Bytes(), &cpBody); err != nil {
		t.Fatalf("checkpoints json: %v", err)
	}
	if cpBody.Count == 0 || len(cpBody.Checkpoints) == 0 {
		t.Fatal("expected exported checkpoints for offline verify")
	}

	// 2) Offline Verify with HMAC nil + public key from the admin keys response
	//    (not the in-process alog.PublicKey() helper — the auditor only has the export).
	recs := alog.Records()
	if len(recs) == 0 {
		t.Fatal("expected sealed records")
	}
	res := audit.Verify(recs, cpBody.Checkpoints, nil, ed25519.PublicKey(pubRaw))
	if !res.OK || res.Class != audit.ClassHMACUnchecked {
		t.Fatalf("public-key-only offline verify failed: %+v", res)
	}

	// 3) Inclusion proof for seq=1 is offline-verifiable against the STH merkle root.
	pw := adminDo(srv, toks[RoleAuditor], "GET", "/__hmn/admin/proof?seq=1", "")
	if pw.Code != http.StatusOK {
		t.Fatalf("proof: %d %s", pw.Code, pw.Body.String())
	}
	var proofBody struct {
		LeafData  string   `json:"leafData"`
		LeafIndex int      `json:"leafIndex"`
		TreeSize  int      `json:"treeSize"`
		Proof     []string `json:"proof"`
		STH       struct {
			MerkleRoot string `json:"merkleRoot"`
			Sig        string `json:"sig"`
		} `json:"sth"`
	}
	if err := json.Unmarshal(pw.Body.Bytes(), &proofBody); err != nil {
		t.Fatalf("proof json: %v", err)
	}
	leaf, err := hex.DecodeString(proofBody.LeafData)
	if err != nil {
		t.Fatalf("leafData: %v", err)
	}
	root, err := hex.DecodeString(proofBody.STH.MerkleRoot)
	if err != nil {
		t.Fatalf("merkleRoot: %v", err)
	}
	sibs := make([][]byte, len(proofBody.Proof))
	for i, h := range proofBody.Proof {
		b, err := hex.DecodeString(h)
		if err != nil {
			t.Fatalf("proof[%d]: %v", i, err)
		}
		sibs[i] = b
	}
	if !audit.VerifyInclusion(leaf, proofBody.LeafIndex, proofBody.TreeSize, sibs, root) {
		t.Fatal("inclusion proof did not verify against exported STH merkle root")
	}
	// STH Ed25519 signature was already validated by audit.Verify under the
	// published public key (proofBody.STH.Sig is the same writer material).
	if proofBody.STH.Sig == "" {
		t.Fatal("proof must export sth.sig for independent auditors")
	}
}
