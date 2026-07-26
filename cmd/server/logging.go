package main

import (
	"context"
	"fmt"
	"log"
	"log/slog"
	"net/http"
	"os"
	"time"

	"github.com/modootoday/humanymous/internal/logger"
)

const loggingShutdownTimeout = 2 * time.Second

type coreLoggingConfig struct {
	Level         string
	ConsoleFormat string
	ConsoleStream string
	PlainFile     string
	JSONLFile     string
}

// configureLogging opens the SoT-40 operational logger and installs its handler
// as the slog and standard-log compatibility bridge. Core remains off by
// default, while explicit invalid configuration fails startup.
func (a *app) configureLogging(cfg coreLoggingConfig) (*logger.Runtime, error) {
	sinks, err := coreLogSinks(cfg)
	if err != nil {
		return nil, err
	}
	runtime, err := logger.Open(logger.Config{
		Service: "core",
		Version: version,
		Node:    coreLogNode(),
		Level:   cfg.Level,
		Sinks:   sinks,
	})
	if err != nil {
		return nil, err
	}

	a.log = runtime.Logger()
	a.logEnabled = cfg.Level != "off"

	// Shared packages still using slog or the standard logger receive the same
	// bounded handler and physical format. Core call sites continue to use a.log
	// so they do not depend on mutable process-global state.
	slog.SetDefault(a.log)
	compat := slog.NewLogLogger(runtime.Handler(), slog.LevelInfo)
	log.SetOutput(compat.Writer())
	log.SetFlags(0)
	log.SetPrefix("")
	return runtime, nil
}

func coreLogSinks(cfg coreLoggingConfig) ([]logger.SinkConfig, error) {
	switch cfg.Level {
	case "off", "debug", "info", "warn", "error":
	default:
		return nil, fmt.Errorf("invalid log level %q", cfg.Level)
	}
	var consoleWriter *os.File
	switch cfg.ConsoleStream {
	case "stderr":
		consoleWriter = os.Stderr
	case "stdout":
		consoleWriter = os.Stdout
	default:
		return nil, fmt.Errorf("invalid log console stream %q", cfg.ConsoleStream)
	}

	sinks := make([]logger.SinkConfig, 0, 3)
	switch cfg.ConsoleFormat {
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
		return nil, fmt.Errorf("invalid log console format %q", cfg.ConsoleFormat)
	}
	if cfg.PlainFile != "" {
		sinks = append(sinks, logger.SinkConfig{
			Name:   "plain_file",
			Format: logger.FormatPlain,
			Path:   cfg.PlainFile,
		})
	}
	if cfg.JSONLFile != "" {
		sinks = append(sinks, logger.SinkConfig{
			Name:   "jsonl_file",
			Format: logger.FormatJSONL,
			Path:   cfg.JSONLFile,
		})
	}
	return sinks, nil
}

func coreLogNode() string {
	node, err := os.Hostname()
	if err != nil || node == "" {
		return "unknown"
	}
	return node
}

func resolveLogSetting(flagValue string, flagSet bool, envName, fallback string) string {
	if flagSet {
		return flagValue
	}
	if value, ok := os.LookupEnv(envName); ok {
		return value
	}
	return fallback
}

func closeOperationalLogger(runtime *logger.Runtime) {
	if runtime == nil {
		return
	}
	ctx, cancel := context.WithTimeout(context.Background(), loggingShutdownTimeout)
	defer cancel()
	_ = runtime.Close(ctx)
}

// withObservability is strictly opt-in. It emits only fixed event identifiers
// and allowlisted protocol fields; request paths and panic material are excluded.
func (a *app) withObservability(next http.Handler) http.Handler {
	if !a.logEnabled {
		return next
	}
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		defer func() {
			if recover() != nil {
				a.log.Error("HTTP handler panic recovered.",
					"component", "core.http",
					"event", "http.panic_recovered")
				w.WriteHeader(http.StatusInternalServerError)
			}
		}()
		next.ServeHTTP(w, r)
		a.log.Info("HTTP request completed.",
			"component", "core.http",
			"event", "http.request_completed",
			"method", r.Method,
			"protocol", protoVer(r))
	})
}
