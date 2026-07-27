package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/modootoday/humanymous/internal/signals"
)

func externalInputReceiptFixture(t *testing.T) (*app, string, []byte) {
	t.Helper()
	webDirectory := t.TempDir()
	detector := []byte("\x00asm-test-detector")
	if err := os.WriteFile(
		filepath.Join(webDirectory, "detector.wasm"),
		detector,
		0o600,
	); err != nil {
		t.Fatal(err)
	}
	receiptDirectory := t.TempDir()
	a := newApp(webDirectory, make([]byte, 32), false)
	if err := a.configureExternalInputReceipts(receiptDirectory); err != nil {
		t.Fatal(err)
	}
	return a, receiptDirectory, detector
}

func externalInputReport() signals.SessionReport {
	return signals.SessionReport{
		SessionID: "sensitive-session-id",
		Timestamp: time.Unix(1_700_000_000, 0).UTC(),
		Client: signals.ClientReport{
			SessionID:     "sensitive-session-id",
			EngineVersion: "detector-test-v1",
			Signals: []signals.Signal{
				signals.New(
					"l1.navigator.webdriver",
					false,
					signals.VerdictOK,
					1,
					signals.SourceWASM,
					"fixture",
				),
			},
		},
	}
}

func externalInputScoreResult() signals.ScoreResult {
	return signals.ScoreResult{
		RiskScore:     74.3,
		Verdict:       "CHALLENGE",
		HardRuleFired: "HR-12",
		PolicyVersion: "1.0.0",
	}
}

func TestExternalInputReceiptBindsCanonicalCoreEvidence(t *testing.T) {
	a, directory, detector := externalInputReceiptFixture(t)
	report := externalInputReport()
	result := externalInputScoreResult()
	label := "external-input/run-20260727/external_input_virtual"

	if err := a.writeExternalInputScoreReceipt(
		label,
		"sensitive-session-id",
		result,
		report,
	); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(filepath.Join(directory, "core.score.json"))
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(raw), "sensitive-session-id") {
		t.Fatalf("raw session identifier reached the receipt: %s", raw)
	}
	var receipt externalInputCoreReceipt
	if err := json.Unmarshal(raw, &receipt); err != nil {
		t.Fatal(err)
	}
	detectorDigest := sha256.Sum256(detector)
	reportJSON, err := json.Marshal(report)
	if err != nil {
		t.Fatal(err)
	}
	reportDigest := sha256.Sum256(reportJSON)
	sessionDigest := sha256.Sum256([]byte("sensitive-session-id"))
	if receipt.SchemaVersion != externalInputCoreReceiptSchema ||
		receipt.RunLabel != label ||
		receipt.RunID != "run-20260727" ||
		receipt.ProfileID != "external_input_virtual" ||
		receipt.SessionRef != hex.EncodeToString(sessionDigest[:8]) ||
		receipt.DetectorWASMHash != hex.EncodeToString(detectorDigest[:]) ||
		receipt.DetectorPath != "/static/detector.wasm" ||
		receipt.ReportSHA256 != hex.EncodeToString(reportDigest[:]) ||
		receipt.WASMSignalCount != 1 ||
		receipt.EngineVersion != "detector-test-v1" ||
		receipt.ScoreRecomputed ||
		!receipt.ServerAuthoritative {
		t.Fatalf("receipt binding is incomplete: %#v", receipt)
	}
	traceMaterial := strings.Join([]string{
		externalInputScoreTraceSchema,
		receipt.RunLabel,
		receipt.SessionRef,
		strconv.FormatFloat(receipt.RiskScore, 'f', 1, 64),
		receipt.Verdict,
		receipt.HardRuleFired,
		receipt.PolicyVersion,
		receipt.DetectorWASMHash,
		receipt.EngineVersion,
		strconv.Itoa(receipt.WASMSignalCount),
		receipt.ReportSHA256,
	}, "\x00")
	traceDigest := sha256.Sum256([]byte(traceMaterial))
	if receipt.ScoreTraceSHA256 != hex.EncodeToString(traceDigest[:]) {
		t.Fatalf("trace digest mismatch: %s", receipt.ScoreTraceSHA256)
	}
	if err := a.writeExternalInputScoreReceipt(
		label,
		"another-session",
		result,
		report,
	); err == nil {
		t.Fatal("exclusive receipt publication permitted overwrite")
	}
}

func TestExternalInputReceiptFailsClosedWithoutWASMEvidence(t *testing.T) {
	a, _, _ := externalInputReceiptFixture(t)
	report := externalInputReport()
	report.Client.Signals[0].Collected = signals.SourceJS
	if err := a.writeExternalInputScoreReceipt(
		"external-input/run-1/external_input_virtual",
		"session",
		externalInputScoreResult(),
		report,
	); err == nil {
		t.Fatal("receipt accepted a report without a WASM-collected signal")
	}
}

func TestExternalInputReceiptConfigurationIsExplicit(t *testing.T) {
	a := newApp(t.TempDir(), make([]byte, 32), false)
	if err := a.configureExternalInputReceipts(""); err != nil {
		t.Fatal(err)
	}
	if err := a.writeExternalInputScoreReceipt(
		"external-input/run-1/external_input_virtual",
		"session",
		externalInputScoreResult(),
		externalInputReport(),
	); err != nil {
		t.Fatalf("disabled receipt channel must be a no-op: %v", err)
	}
	if err := a.configureExternalInputReceipts("relative"); err == nil {
		t.Fatal("relative receipt directory was accepted")
	}
	missingDetectorDirectory := t.TempDir()
	a.webDir = missingDetectorDirectory
	if err := a.configureExternalInputReceipts(t.TempDir()); err == nil {
		t.Fatal("receipt channel started without detector.wasm")
	}
}

func TestExternalInputReceiptNeverUsesOperationalLog(t *testing.T) {
	a, _, _ := externalInputReceiptFixture(t)
	preserveGlobalLogging(t)
	jsonlPath := filepath.Join(t.TempDir(), "core.jsonl")
	runtime, err := a.configureLogging(coreLoggingConfig{
		Level:         "info",
		ConsoleFormat: "off",
		ConsoleStream: "stderr",
		JSONLFile:     jsonlPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if err := a.writeExternalInputScoreReceipt(
		"external-input/run-secret/external_input_virtual",
		"sensitive-session-id",
		externalInputScoreResult(),
		externalInputReport(),
	); err != nil {
		t.Fatal(err)
	}
	a.log.Info("fixed event", "component", "test", "event", "test.fixed")
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	if err := runtime.Flush(ctx); err != nil {
		t.Fatal(err)
	}
	closeOperationalLogger(runtime)
	content, err := os.ReadFile(jsonlPath)
	if err != nil {
		t.Fatal(err)
	}
	for _, forbidden := range []string{
		"run-secret",
		"sensitive-session-id",
		"74.3",
		"CHALLENGE",
		"HR-12",
		"external_input.detector_scored",
	} {
		if strings.Contains(string(content), forbidden) {
			t.Fatalf("operational log contains score evidence %q: %s", forbidden, content)
		}
	}
}
