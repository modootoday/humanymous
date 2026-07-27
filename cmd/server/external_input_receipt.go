package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/modootoday/humanymous/internal/signals"
)

const (
	externalInputCoreReceiptSchema = "humanymous.external-input-core-score/v1"
	externalInputScoreTraceSchema  = "humanymous.external-input-score-trace/v1"
	maximumDetectorWASMBytes       = 8 * 1024 * 1024
)

type externalInputReceiptWriter struct {
	directory          string
	detectorWASMHash   string
	detectorWASMPath   string
	maximumReceiptSize int
}

type externalInputCoreReceipt struct {
	SchemaVersion       string  `json:"schemaVersion"`
	RunLabel            string  `json:"runLabel"`
	RunID               string  `json:"runId"`
	ProfileID           string  `json:"profileId"`
	SessionRef          string  `json:"sessionRef"`
	RiskScore           float64 `json:"riskScore"`
	Verdict             string  `json:"verdict"`
	HardRuleFired       string  `json:"hardRuleFired"`
	PolicyVersion       string  `json:"policyVersion"`
	DetectorPath        string  `json:"detectorPath"`
	DetectorWASMHash    string  `json:"detectorWasmSha256"`
	EngineVersion       string  `json:"engineVersion"`
	WASMSignalCount     int     `json:"wasmSignalCount"`
	ReportSHA256        string  `json:"reportSha256"`
	ScoreTraceSHA256    string  `json:"scoreTraceSha256"`
	ScoreRecomputed     bool    `json:"scoreRecomputed"`
	ServerAuthoritative bool    `json:"serverAuthoritative"`
}

// configureExternalInputReceipts enables the lab-only evidence channel. It is
// intentionally separate from the SoT-40 operational logger and is disabled in
// normal deployments.
func (a *app) configureExternalInputReceipts(directory string) error {
	if directory == "" {
		a.externalInputReceipts = nil
		return nil
	}
	if !filepath.IsAbs(directory) {
		return errors.New("external-input receipt directory must be absolute")
	}
	info, err := os.Lstat(directory)
	if err != nil {
		return fmt.Errorf("external-input receipt directory: %w", err)
	}
	if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return errors.New("external-input receipt path must be a real directory")
	}
	detectorPath := filepath.Join(a.webDir, "detector.wasm")
	detector, err := os.ReadFile(detectorPath)
	if err != nil {
		return fmt.Errorf("read detector WASM: %w", err)
	}
	if len(detector) < 1 || len(detector) > maximumDetectorWASMBytes {
		return errors.New("detector WASM exceeds its byte policy")
	}
	digest := sha256.Sum256(detector)
	a.externalInputReceipts = &externalInputReceiptWriter{
		directory:          directory,
		detectorWASMHash:   hex.EncodeToString(digest[:]),
		detectorWASMPath:   "/static/detector.wasm",
		maximumReceiptSize: 16 * 1024,
	}
	return nil
}

func (a *app) writeExternalInputScoreReceipt(
	label, sid string,
	result signals.ScoreResult,
	report signals.SessionReport,
) error {
	if a.externalInputReceipts == nil {
		return nil
	}
	label, ok := canonicalExternalInputLabel(label)
	if !ok {
		return nil
	}
	parts := strings.Split(label, "/")
	if len(parts) != 3 || parts[0] != "external-input" ||
		parts[1] == "" || parts[2] == "" {
		return errors.New("external-input label is not run-bound")
	}
	reportJSON, err := json.Marshal(report)
	if err != nil {
		return errors.New("canonical report encoding failed")
	}
	reportDigest := sha256.Sum256(reportJSON)
	reportHash := hex.EncodeToString(reportDigest[:])
	wasmSignals := 0
	for _, signal := range report.AllSignals() {
		if signal.Collected == signals.SourceWASM {
			wasmSignals++
		}
	}
	if wasmSignals < 1 || report.Client.EngineVersion == "" {
		return errors.New("WASM collection evidence is missing")
	}
	sessionDigest := sha256.Sum256([]byte(sid))
	sessionRef := hex.EncodeToString(sessionDigest[:8])
	risk := strconv.FormatFloat(result.RiskScore, 'f', 1, 64)
	traceMaterial := strings.Join([]string{
		externalInputScoreTraceSchema,
		label,
		sessionRef,
		risk,
		string(result.Verdict),
		result.HardRuleFired,
		result.PolicyVersion,
		a.externalInputReceipts.detectorWASMHash,
		report.Client.EngineVersion,
		strconv.Itoa(wasmSignals),
		reportHash,
	}, "\x00")
	traceDigest := sha256.Sum256([]byte(traceMaterial))
	receipt := externalInputCoreReceipt{
		SchemaVersion:       externalInputCoreReceiptSchema,
		RunLabel:            label,
		RunID:               parts[1],
		ProfileID:           parts[2],
		SessionRef:          sessionRef,
		RiskScore:           result.RiskScore,
		Verdict:             string(result.Verdict),
		HardRuleFired:       result.HardRuleFired,
		PolicyVersion:       result.PolicyVersion,
		DetectorPath:        a.externalInputReceipts.detectorWASMPath,
		DetectorWASMHash:    a.externalInputReceipts.detectorWASMHash,
		EngineVersion:       report.Client.EngineVersion,
		WASMSignalCount:     wasmSignals,
		ReportSHA256:        reportHash,
		ScoreTraceSHA256:    hex.EncodeToString(traceDigest[:]),
		ScoreRecomputed:     false,
		ServerAuthoritative: true,
	}
	return writeExclusiveJSON(
		filepath.Join(a.externalInputReceipts.directory, "core.score.json"),
		receipt,
		a.externalInputReceipts.maximumReceiptSize,
	)
}

func writeExclusiveJSON(destination string, value any, maximumBytes int) error {
	raw, err := json.Marshal(value)
	if err != nil {
		return err
	}
	raw = append(raw, '\n')
	if len(raw) > maximumBytes {
		return errors.New("external-input receipt exceeds its byte policy")
	}
	directory := filepath.Dir(destination)
	temporary, err := os.CreateTemp(directory, ".core-score-*")
	if err != nil {
		return err
	}
	temporaryPath := temporary.Name()
	defer os.Remove(temporaryPath)
	closed := false
	defer func() {
		if !closed {
			_ = temporary.Close()
		}
	}()
	if err := temporary.Chmod(0o644); err != nil {
		return err
	}
	if _, err := temporary.Write(raw); err != nil {
		return err
	}
	if err := temporary.Sync(); err != nil {
		return err
	}
	if err := temporary.Close(); err != nil {
		return err
	}
	closed = true
	if err := os.Link(temporaryPath, destination); err != nil {
		return err
	}
	return nil
}
