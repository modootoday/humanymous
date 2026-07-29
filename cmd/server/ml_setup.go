package main

import (
	"log"
	"os"

	"github.com/modootoday/humanymous/internal/mlcorrect"
	"github.com/modootoday/humanymous/internal/mlserve"
)

// ml_setup.go admits and installs the SoT-42 Pillar A behavioral model at startup.
//
// Promotion is a DEPLOY-TIME event by the SoT-42 expert-panel decision (red/blue/sre unanimous):
// there is deliberately NO runtime model-mutation endpoint. While l4.ml.behavioral is weight-0
// (audit-only), a "promote" is a redeploy with a new signed -ml-bundle; a future verdict-affecting
// weight (an explicit detection-freeze spend) would make it a GATE-ADMIN RBAC + dual-control action,
// never a Core bearer-token endpoint. Rationale: the Core ops plane is single-bearer and cannot
// express distinct approver → any "dual-control" there is theater; an immutable signed-bundle-in-
// deploy keeps "the running model == the deployed digest" for on-call; and rolling deploy already
// owns multi-replica convergence + rollback.
//
// The one real gap this closes: the served bundle is now admitted through BundleManager.Stage →
// Promote, so the schema-pin + sha256 digest + ed25519 signature are ENFORCED on the path that faces
// traffic — previously the startup path bypassed them via a raw mlserve.LoadMLP, leaving the "signed
// bundle" story unenforced on the served model. Any load/verify failure stays NON-FATAL: the seam
// abstains → heuristics baseline, so bad model plumbing never blocks a human (fail-open on the model).

// mlFlags is the behavioral-ML startup configuration.
type mlFlags struct {
	bundle          string
	pubkey          string // PKIX ed25519 public-key PEM; when set, the served bundle's signature is REQUIRED
	sig             string // detached ed25519 signature file over the bundle bytes
	digest          string // optional pinned sha256 hex of the bundle
	fpBudget        float64
	shadowBundle    string
	canary          bool
	canaryMaxFP     float64
	canaryProbation int
}

// mlSetup carries the constructed runtime handles (zero value = disabled).
type mlSetup struct {
	ctrl    *mlcorrect.Controller
	bundles *mlcorrect.BundleManager // owns the served-model identity + rollback (never a runtime mutation API)
}

// setupBehavioralML wires the model, self-calibration, canary auto-rollback, and shadow scoring.
func setupBehavioralML(f mlFlags) mlSetup {
	if f.bundle == "" {
		if f.shadowBundle != "" {
			log.Printf("ml-shadow-bundle: ignored — requires -ml-bundle")
		}
		return mlSetup{}
	}

	var pubPEM []byte
	if f.pubkey != "" {
		b, err := os.ReadFile(f.pubkey)
		if err != nil {
			log.Printf("ml-pubkey: read failed (%v) — behavioral model disabled (refusing to serve an unverifiable bundle)", err)
			return mlSetup{}
		}
		pubPEM = b
	}
	bm, err := mlcorrect.NewBundleManager(pubPEM)
	if err != nil {
		log.Printf("ml-pubkey: %v — behavioral model disabled", err)
		return mlSetup{}
	}

	var sig []byte
	if f.sig != "" {
		b, err := os.ReadFile(f.sig)
		if err != nil {
			log.Printf("ml-bundle-sig: read failed (%v) — behavioral model disabled", err)
			return mlSetup{}
		}
		sig = b
	}

	// Stage verifies (schema-pin always; digest if pinned; signature if a pubkey was configured); a
	// failure here means the served model is unverifiable → refuse it and fall back to heuristics.
	if err := bm.Stage(f.bundle, f.digest, sig); err != nil {
		log.Printf("ml-bundle: verify/stage failed (%v) — behavioral model disabled, engine runs on heuristics", err)
		return mlSetup{}
	}
	if err := bm.Promote(); err != nil {
		log.Printf("ml-bundle: promote failed (%v) — behavioral model disabled", err)
		return mlSetup{}
	}

	ver, dig := bm.Active()
	if pubPEM == nil && f.digest == "" {
		log.Printf("WARNING: ml-bundle admitted with NEITHER -ml-pubkey NOR -ml-bundle-digest — integrity rests on the feature-schema pin alone (any schema-matching artifact at this path is served). Set -ml-pubkey (signature) or at least -ml-bundle-digest (sha256 pin) for signed/pinned admission in production.")
	}
	ctrl := mlcorrect.NewController(float32(f.fpBudget), 0.5, 0.01)
	mlserve.SetThresholdProvider(ctrl)
	mlserve.SetFeatureObserver(ctrl) // covariate-drift tap (hot path, model-loaded only)
	log.Printf("ml-bundle: admitted %s (digest %s, signed=%t); self-calibration active (budget=%.4f θ0=%.2f)",
		ver, shortDigest(dig), pubPEM != nil, f.fpBudget, ctrl.FireThreshold())

	// ⑧ CANARY: put the served model on probation; the SAFE direction (revert to heuristics) is
	// autonomous. Rollback goes through the BundleManager so the reverted identity is recorded, not a
	// blind mlserve.Set(nil).
	if f.canary {
		ctrl.ArmCanary(mlcorrect.CanaryBudget{
			MaxHumanFP:            float32(f.canaryMaxFP),
			ProbationHumans:       uint64(f.canaryProbation),
			RollbackOnDrift:       true,
			RollbackOnPassAnomaly: true,
		}, func(reason string) {
			log.Printf("ml-canary: AUTO-ROLLBACK — %s; reverting the behavioral model to the heuristics baseline", reason)
			if err := bm.Rollback(); err != nil {
				log.Printf("ml-canary: BundleManager rollback failed (%v) — forcing abstain", err)
				mlserve.Set(nil)
			}
			mlserve.SetShadow(nil) // stop shadow-scoring too
		})
		log.Printf("ml-canary: probation armed (max human-FP=%.4f, graduate after %d humans)", f.canaryMaxFP, f.canaryProbation)
	}

	// ⑦ SHADOW: score a candidate in parallel (never served) to preview a promotion. The shadow is
	// schema-guarded in mlserve.Score and its output is structurally unreachable by the verdict path.
	if f.shadowBundle != "" {
		if sm, err := mlserve.LoadMLP(f.shadowBundle); err != nil {
			log.Printf("ml-shadow-bundle: load failed (%v) — shadow evaluation disabled", err)
		} else {
			mlserve.SetShadow(sm)
			mlserve.SetShadowObserver(ctrl)
			log.Printf("ml-shadow-bundle: shadow-scoring candidate %s (never served; divergence at /api/mlcorrect)", sm.BundleVersion())
		}
	}

	return mlSetup{ctrl: ctrl, bundles: bm}
}

// shortDigest trims a hex digest for logs.
func shortDigest(d string) string {
	if len(d) > 12 {
		return d[:12]
	}
	return d
}
