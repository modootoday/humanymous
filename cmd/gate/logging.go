package main

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"os"
	"strings"
	"time"

	"github.com/modootoday/humanymous/internal/logger"
)

var installedGateLogger *logger.Runtime

// gateLoggingConfig is the Gate adapter for the shared operational logging
// contract in SoT-40. Operational logging is diagnostics only and deliberately
// has no dependency on scoring, enforcement, settings, or the audit stream.
type gateLoggingConfig struct {
	level         string
	consoleFormat string
	consoleStream string
	plainFile     string
	jsonlFile     string
}

func envOrDefault(name, fallback string) string {
	if value, ok := os.LookupEnv(name); ok {
		return value
	}
	return fallback
}

func openGateLogger(cfg gateLoggingConfig, node string, stdout, stderr io.Writer) (*logger.Runtime, error) {
	consoleFormat := strings.ToLower(strings.TrimSpace(cfg.consoleFormat))
	consoleStream := strings.ToLower(strings.TrimSpace(cfg.consoleStream))
	level := strings.ToLower(strings.TrimSpace(cfg.level))

	var consoleWriter io.Writer
	switch consoleStream {
	case "stderr":
		consoleWriter = stderr
	case "stdout":
		consoleWriter = stdout
	default:
		return nil, fmt.Errorf("invalid console stream %q (want stderr or stdout)", consoleStream)
	}

	sinks := make([]logger.SinkConfig, 0, 3)
	switch consoleFormat {
	case "off":
	case "plain":
		sinks = append(sinks, logger.SinkConfig{
			Name:   "console",
			Format: logger.FormatPlain,
			Writer: consoleWriter,
		})
	case "jsonl":
		sinks = append(sinks, logger.SinkConfig{
			Name:   "console",
			Format: logger.FormatJSONL,
			Writer: consoleWriter,
		})
	default:
		return nil, fmt.Errorf("invalid console format %q (want off, plain, or jsonl)", consoleFormat)
	}
	if cfg.plainFile != "" {
		sinks = append(sinks, logger.SinkConfig{
			Name:   "plain_file",
			Format: logger.FormatPlain,
			Path:   cfg.plainFile,
		})
	}
	if cfg.jsonlFile != "" {
		sinks = append(sinks, logger.SinkConfig{
			Name:   "jsonl_file",
			Format: logger.FormatJSONL,
			Path:   cfg.jsonlFile,
		})
	}

	return logger.Open(logger.Config{
		Service: "gate",
		Version: version,
		Node:    node,
		Level:   level,
		Sinks:   sinks,
	})
}

func installGateLogger(runtime *logger.Runtime) {
	// SetDefault also bridges the package-level log.Logger into this handler.
	// Remove the legacy prefix so records have only the canonical timestamp.
	slog.SetDefault(runtime.Logger())
	slog.SetLogLoggerLevel(slog.LevelInfo)
	installedGateLogger = runtime
}

func closeGateLogger(runtime *logger.Runtime) {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	_ = runtime.Close(ctx)
}

// gateFatalf preserves log.Fatalf's process-exit contract while giving the
// bounded asynchronous logger a chance to write the final error record.
func gateFatalf(format string, args ...any) {
	// The original error can contain an operator path, credential-bearing URL,
	// or library-provided request text. Persist only a fixed classification.
	_ = args
	slog.Error("Gate stopped after a startup or listener failure.",
		"component", "gate.runtime",
		"event", "runtime.fatal",
		"error_class", gateFatalClass(format))
	if installedGateLogger != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		_ = installedGateLogger.Flush(ctx)
		cancel()
	}
	os.Exit(1)
}

func gateFatalClass(format string) string {
	switch {
	case strings.Contains(format, "keystore"):
		return "keystore"
	case strings.Contains(format, "audit-wal"):
		return "audit_wal"
	case strings.Contains(format, "audit-verify"):
		return "audit_verification"
	case strings.Contains(format, "REDIS_KEY"):
		return "shared_state_key"
	case strings.Contains(format, "TOKEN_KEY"):
		return "token_key"
	case strings.Contains(format, "settings"):
		return "settings"
	case strings.Contains(format, "agent-keys"):
		return "agent_keys"
	case strings.Contains(format, "pat-issuers"):
		return "token_issuers"
	case strings.Contains(format, "webauthn"):
		return "webauthn_credentials"
	case strings.Contains(format, "routes"):
		return "route_policy"
	case strings.Contains(format, "proxy:"):
		return "origin_proxy"
	case strings.Contains(format, "admin"):
		return "admin_listener"
	case strings.Contains(format, "tls"):
		return "tls"
	case strings.Contains(format, "trusted-proxies"):
		return "trusted_proxies"
	case strings.Contains(format, "edge listener"):
		return "edge_listener"
	case strings.Contains(format, "refusing to boot"):
		return "credential_policy"
	default:
		return "startup"
	}
}
