package logger

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
)

type sink struct {
	name   string
	format Format
	writer io.Writer
	owned  *os.File

	writeErrors atomic.Uint64
}

func openSinks(configs []SinkConfig) ([]*sink, error) {
	opened := make([]*sink, 0, len(configs))
	filePaths := make(map[string]string)
	closeOpened := func() {
		for _, s := range opened {
			if s.owned != nil {
				_ = s.owned.Close()
			}
		}
	}

	for i, cfg := range configs {
		if err := validateFormat(cfg.Format); err != nil {
			closeOpened()
			return nil, err
		}
		if (cfg.Path == "") == (cfg.Writer == nil) {
			closeOpened()
			return nil, fmt.Errorf("logger: sink %d must set exactly one of path or writer", i)
		}
		name := sanitizeString(cfg.Name, maxIdentityBytes)
		if name == "" {
			name = fmt.Sprintf("sink_%d", i+1)
		}
		s := &sink{name: name, format: cfg.Format}
		if cfg.Writer != nil {
			s.writer = cfg.Writer
			opened = append(opened, s)
			continue
		}

		canonical, err := canonicalFilePath(cfg.Path)
		if err != nil {
			closeOpened()
			return nil, err
		}
		key := canonical
		if runtime.GOOS == "windows" {
			key = strings.ToLower(key)
		}
		if previous, exists := filePaths[key]; exists {
			closeOpened()
			return nil, fmt.Errorf("logger: sinks %q and %q use the same file", previous, name)
		}
		filePaths[key] = name

		file, err := openAppendRegular(canonical)
		if err != nil {
			closeOpened()
			return nil, fmt.Errorf("logger: open sink %q: %w", name, err)
		}
		for _, prior := range opened {
			if prior.owned == nil {
				continue
			}
			left, leftErr := prior.owned.Stat()
			right, rightErr := file.Stat()
			if leftErr == nil && rightErr == nil && os.SameFile(left, right) {
				_ = file.Close()
				closeOpened()
				return nil, fmt.Errorf("logger: sinks %q and %q use the same file", prior.name, name)
			}
		}
		s.writer = file
		s.owned = file
		opened = append(opened, s)
	}
	return opened, nil
}

func canonicalFilePath(path string) (string, error) {
	if strings.TrimSpace(path) == "" {
		return "", errors.New("logger: empty sink path")
	}
	absolute, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("logger: resolve path: %w", err)
	}
	return filepath.Clean(absolute), nil
}

func openAppendRegular(path string) (*os.File, error) {
	info, err := os.Lstat(path)
	switch {
	case err == nil:
		if info.Mode()&os.ModeSymlink != 0 {
			return nil, errors.New("symlink sink is not allowed")
		}
		if !info.Mode().IsRegular() {
			return nil, errors.New("sink is not a regular file")
		}
	case !os.IsNotExist(err):
		return nil, err
	}

	file, err := os.OpenFile(path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o640)
	if err != nil {
		return nil, err
	}
	openedInfo, err := file.Stat()
	if err != nil {
		_ = file.Close()
		return nil, err
	}
	if !openedInfo.Mode().IsRegular() {
		_ = file.Close()
		return nil, errors.New("opened sink is not a regular file")
	}
	// Re-check the name after open and compare it with the already-open handle.
	// This closes the Lstat/OpenFile swap window: a symlink substituted between
	// the first check and open either remains a symlink here or resolves to a
	// different file identity.
	pathInfo, err := os.Lstat(path)
	if err != nil {
		_ = file.Close()
		return nil, err
	}
	if pathInfo.Mode()&os.ModeSymlink != 0 || !os.SameFile(openedInfo, pathInfo) {
		_ = file.Close()
		return nil, errors.New("sink path changed while opening")
	}
	return file, nil
}

func (s *sink) write(line []byte) error {
	n, err := s.writer.Write(line)
	if err == nil && n != len(line) {
		return io.ErrShortWrite
	}
	return err
}

func (s *sink) sync() error {
	if syncer, ok := s.writer.(interface{ Sync() error }); ok {
		return syncer.Sync()
	}
	return nil
}

func (s *sink) close() error {
	if s.owned == nil {
		return nil
	}
	return s.owned.Close()
}
