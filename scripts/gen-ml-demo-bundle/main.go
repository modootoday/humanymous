// Command gen-ml-demo-bundle SIGNS an already-trained behavioral-model bundle with a
// freshly-minted DEMO ed25519 key, so the ml.yaml overlay can exercise the SIGNED admission
// path (-ml-pubkey + -ml-bundle-sig) end-to-end in Docker. It is deliberately a pure signer:
// the bundle itself is produced by the project's own bootstrap trainer (cmd/ml-train -gen),
// so there is a single source of the grounded synthetic distributions and no training math is
// duplicated here.
//
//	go run ./cmd/ml-train -gen 12000 -out configs/ml/behavioral.json
//	go run ./scripts/gen-ml-demo-bundle -bundle configs/ml/behavioral.json -outdir configs/ml
//
// Writes, next to the bundle:
//   - pub.pem     PKIX ed25519 public key (world-readable; bind-mounted RO into the non-root Core)
//   - bundle.sig  raw ed25519 signature over the bundle file bytes (what BundleManager.Stage verifies)
//
// The private key is EPHEMERAL and never written: each build mints a fresh keypair and bakes the
// public half + signature together, so the pair is always self-consistent. These are DEMO artifacts
// for the lab — a real deployment signs bundles with an out-of-band operator key. Mirrors the spirit
// of scripts/gen-demo-keys (reproducible from a clean tree, no committed private material).
package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"crypto/x509"
	"encoding/hex"
	"encoding/pem"
	"flag"
	"fmt"
	"os"
	"path/filepath"
)

func main() {
	bundle := flag.String("bundle", "", "path to the trained (unsigned) bundle JSON to sign — required")
	outdir := flag.String("outdir", "", "directory to write pub.pem + bundle.sig (default: the bundle's directory)")
	flag.Parse()

	if *bundle == "" {
		must(fmt.Errorf("-bundle is required (train one first: go run ./cmd/ml-train -gen 12000 -out %s)", "configs/ml/behavioral.json"))
	}
	dir := *outdir
	if dir == "" {
		dir = filepath.Dir(*bundle)
	}

	raw, err := os.ReadFile(*bundle)
	must(err)

	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	must(err)
	der, err := x509.MarshalPKIXPublicKey(pub)
	must(err)

	// A PUBLIC key + a signature carry no secret — they are bind-mounted read-only into the
	// distroless, NON-ROOT Core container, so they must be world-readable or the container user
	// gets "permission denied" and the model fails to admit (same lesson as gen-demo-keys).
	writeFile(filepath.Join(dir, "pub.pem"),
		pem.EncodeToMemory(&pem.Block{Type: "PUBLIC KEY", Bytes: der}), 0o644)
	writeFile(filepath.Join(dir, "bundle.sig"), ed25519.Sign(priv, raw), 0o644)

	sum := sha256.Sum256(raw)
	fmt.Fprintf(os.Stderr, "gen-ml-demo-bundle: signed %s (%d bytes) → %s/{pub.pem,bundle.sig}\n", *bundle, len(raw), dir)
	// stdout carries the digest so a caller can pin it via -ml-bundle-digest if desired.
	fmt.Println(hex.EncodeToString(sum[:]))
}

func writeFile(path string, content []byte, mode os.FileMode) {
	must(os.MkdirAll(filepath.Dir(path), 0o755))
	must(os.WriteFile(path, content, mode))
}

func must(err error) {
	if err != nil {
		fmt.Fprintln(os.Stderr, "gen-ml-demo-bundle:", err)
		os.Exit(1)
	}
}
