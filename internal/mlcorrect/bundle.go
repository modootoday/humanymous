package mlcorrect

import (
	"crypto/ed25519"
	"crypto/sha256"
	"crypto/x509"
	"encoding/hex"
	"encoding/pem"
	"errors"
	"fmt"
	"os"
	"sync"

	"github.com/modootoday/humanymous/internal/behavior"
	"github.com/modootoday/humanymous/internal/mlserve"
)

// bundle.go is the model-bundle promotion lifecycle for the self-correcting loop (SoT-42 Pillar A,
// steps ⑦–⑨ of SELF-CORRECTION.md). It is the ONLY thing that puts a new behavioral model in front
// of traffic, and it is built to make that transition safe and instantly reversible:
//
//   Stage  → load + VERIFY a candidate (schema-pin + digest + optional ed25519 signature); hold it
//            as a shadow, NOT serving.
//   Promote→ atomic swap into the mlserve seam; the prior model is retained for instant rollback.
//   Rollback→ one atomic swap back to the last-good model (or to the abstaining default).
//
// Freeze-safety is structural, not by discipline: (a) a promoted bundle only becomes reachable
// through mlserve, whose Score() already abstains on schema mismatch and fail-opens on absence; and
// (b) the l4.ml.behavioral signal it feeds is weight-0 / score-exempt in the registry, so even a
// live model shifts only the audit trace, never a verdict, until an operator explicitly wires
// enforcement. An unattended bad promotion is therefore bounded to "more audit noise", and the
// canary/auto-rollback in the update loop reverses it in a single swap.
//
// Signatures use stdlib ed25519 (no cgo, no new deps): the offline trainer signs the bundle file
// bytes; the operator installs the PKIX public key. Verification is refused-by-default only when a
// key is configured — an unconfigured manager still enforces the schema pin and (if provided) the
// digest, so integrity holds even before signing keys are provisioned.

// ErrNoStaged / ErrNoPrevious guard the lifecycle transitions.
var (
	ErrNoStaged    = errors.New("mlcorrect: no verified candidate staged")
	ErrNoPrevious  = errors.New("mlcorrect: no previous bundle to roll back to")
	ErrDigest      = errors.New("mlcorrect: bundle digest mismatch")
	ErrSignature   = errors.New("mlcorrect: bundle signature invalid")
	ErrStaleSchema = errors.New("mlcorrect: bundle schema does not match running engine")
)

// stagedBundle is a loaded, verified model plus the provenance used to admit it.
type stagedBundle struct {
	version string
	digest  string      // sha256 hex of the bundle file, for audit + rollback identity
	mlp     *mlserve.MLP
}

// BundleManager governs which model the mlserve seam serves. Safe for concurrent use.
type BundleManager struct {
	mu       sync.Mutex
	active   *stagedBundle     // currently serving; nil ⇒ abstaining default
	previous *stagedBundle     // last-good, retained for instant rollback
	staged   *stagedBundle     // verified candidate not yet serving (shadow)
	pubkey   ed25519.PublicKey // optional; when set, signatures are REQUIRED to stage
}

// NewBundleManager builds a manager. pubkeyPEM is an optional PKIX-encoded ed25519 public key; when
// nil/empty, signature verification is disabled (schema pin + optional digest still enforced).
func NewBundleManager(pubkeyPEM []byte) (*BundleManager, error) {
	m := &BundleManager{}
	if len(pubkeyPEM) == 0 {
		return m, nil
	}
	block, _ := pem.Decode(pubkeyPEM)
	if block == nil {
		return nil, errors.New("mlcorrect: public key is not valid PEM")
	}
	pub, err := x509.ParsePKIXPublicKey(block.Bytes)
	if err != nil {
		return nil, fmt.Errorf("mlcorrect: parse public key: %w", err)
	}
	ed, ok := pub.(ed25519.PublicKey)
	if !ok {
		return nil, errors.New("mlcorrect: public key is not ed25519")
	}
	m.pubkey = ed
	return m, nil
}

// Digest computes the canonical bundle digest (sha256 hex of the file bytes) — the identity the
// trainer signs and the operator pins.
func Digest(bundleBytes []byte) string {
	sum := sha256.Sum256(bundleBytes)
	return hex.EncodeToString(sum[:])
}

// Stage loads a candidate bundle and admits it as the shadow only if EVERY configured check passes:
//   - the on-disk shape validates (via mlserve.LoadMLP),
//   - its schema hash equals the running engine's behavior schema (reject stale by construction),
//   - if expectDigest != "", the file digest matches it,
//   - if a public key is configured, sig is a valid ed25519 signature over the file bytes.
// It never touches the serving path; call Promote to serve it.
func (m *BundleManager) Stage(path, expectDigest string, sig []byte) error {
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	digest := Digest(raw)
	if expectDigest != "" && digest != expectDigest {
		return fmt.Errorf("%w: got %s want %s", ErrDigest, digest, expectDigest)
	}
	if m.pubkey != nil {
		if !ed25519.Verify(m.pubkey, raw, sig) {
			return ErrSignature
		}
	}
	mlp, err := mlserve.LoadMLP(path)
	if err != nil {
		return err
	}
	if mlp.SchemaHash() != behavior.SchemaHash() {
		return fmt.Errorf("%w: bundle %s engine %s", ErrStaleSchema, mlp.SchemaHash(), behavior.SchemaHash())
	}
	m.mu.Lock()
	m.staged = &stagedBundle{version: mlp.BundleVersion(), digest: digest, mlp: mlp}
	m.mu.Unlock()
	return nil
}

// Staged reports the verified candidate awaiting promotion, if any (for shadow/observability).
func (m *BundleManager) Staged() (version, digest string, ok bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.staged == nil {
		return "", "", false
	}
	return m.staged.version, m.staged.digest, true
}

// Promote atomically installs the staged candidate as the serving model, retaining the prior model
// for rollback. Requires a prior successful Stage.
func (m *BundleManager) Promote() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.staged == nil {
		return ErrNoStaged
	}
	m.previous = m.active
	m.active = m.staged
	m.staged = nil
	mlserve.Set(m.active.mlp)
	return nil
}

// Rollback atomically reverts to the last-good model (or to the abstaining default when the prior
// state was "no model"). Used by the canary auto-rollback and by operators. It is always safe:
// worst case it returns the seam to Abstain, which is the freeze-safe baseline.
func (m *BundleManager) Rollback() error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.active == nil && m.previous == nil {
		return ErrNoPrevious
	}
	m.active = m.previous
	m.previous = nil
	if m.active == nil {
		mlserve.Set(nil) // back to AbstainPredictor
	} else {
		mlserve.Set(m.active.mlp)
	}
	return nil
}

// Active reports the serving bundle version and digest ("none" when abstaining).
func (m *BundleManager) Active() (version, digest string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	if m.active == nil {
		return "none", ""
	}
	return m.active.version, m.active.digest
}
