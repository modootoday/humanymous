package gate

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"time"
)

const (
	maxVirtualUSBRuns       = 32
	maxVirtualUSBCandidates = 128
	maxVirtualUSBResultSize = 64 << 10
)

type virtualUSBIMEView struct {
	SchemaVersion string   `json:"schemaVersion"`
	Order         []string `json:"order"`
	Expected      int      `json:"expected"`
	Measured      int      `json:"measured"`
	Pass          int      `json:"pass"`
	Failures      []string `json:"failures"`
}

type virtualUSBLadderView struct {
	SchemaVersion string            `json:"schemaVersion"`
	Canonical     bool              `json:"canonical"`
	Engines       []string          `json:"engines"`
	Order         []string          `json:"order"`
	Measured      int               `json:"measured"`
	Pass          int               `json:"pass"`
	Residual      int               `json:"residual"`
	Failures      []string          `json:"failures"`
	IME           virtualUSBIMEView `json:"ime"`
}

type virtualUSBRunView struct {
	RecordedAt time.Time            `json:"recordedAt"`
	Result     virtualUSBLadderView `json:"result"`
}

func (s *Server) adminVirtualUSB(w http.ResponseWriter) {
	root := s.cfg.VirtualUSBResultsDir
	if root == "" {
		writeJSON(w, map[string]any{
			"enabled": false,
			"runs":    []virtualUSBRunView{},
			"note":    "Start Gate with -virtual-usb-results-dir to display bounded lab results.",
		})
		return
	}
	runs, err := readVirtualUSBResults(root)
	if err != nil {
		writeJSON(w, map[string]any{
			"enabled": true,
			"healthy": false,
			"runs":    []virtualUSBRunView{},
			"note":    "The configured result directory is unavailable or contains an invalid terminal summary.",
		})
		return
	}
	writeJSON(w, map[string]any{
		"enabled":      true,
		"healthy":      true,
		"verification": "schema-only",
		"runs":         runs,
	})
}

func readVirtualUSBResults(root string) ([]virtualUSBRunView, error) {
	info, err := os.Lstat(root)
	if err != nil || !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return nil, errors.New("virtual USB result root must be a real directory")
	}
	entries, err := os.ReadDir(root)
	if err != nil {
		return nil, err
	}
	type candidate struct {
		path string
		mod  time.Time
		info os.FileInfo
	}
	var candidates []candidate
	for _, entry := range entries {
		if len(candidates) >= maxVirtualUSBCandidates {
			break
		}
		entryInfo, infoErr := entry.Info()
		if infoErr != nil || !entryInfo.IsDir() || entryInfo.Mode()&os.ModeSymlink != 0 {
			continue
		}
		path := filepath.Join(root, entry.Name(), "ladder-result.json")
		resultInfo, statErr := os.Lstat(path)
		if statErr != nil {
			continue
		}
		if !resultInfo.Mode().IsRegular() || resultInfo.Mode()&os.ModeSymlink != 0 ||
			resultInfo.Size() > maxVirtualUSBResultSize {
			return nil, errors.New("virtual USB result must be a bounded regular file")
		}
		candidates = append(candidates, candidate{
			path: path,
			mod:  resultInfo.ModTime().UTC(),
			info: resultInfo,
		})
	}
	sort.Slice(candidates, func(i, j int) bool { return candidates[i].mod.After(candidates[j].mod) })
	if len(candidates) > maxVirtualUSBRuns {
		candidates = candidates[:maxVirtualUSBRuns]
	}
	runs := make([]virtualUSBRunView, 0, len(candidates))
	for _, candidate := range candidates {
		f, openErr := os.Open(candidate.path)
		if openErr != nil {
			return nil, openErr
		}
		openedInfo, statErr := f.Stat()
		if statErr != nil || !os.SameFile(candidate.info, openedInfo) {
			_ = f.Close()
			return nil, errors.New("virtual USB result changed while it was opened")
		}
		decoder := json.NewDecoder(io.LimitReader(f, maxVirtualUSBResultSize+1))
		decoder.DisallowUnknownFields()
		var result virtualUSBLadderView
		decodeErr := decoder.Decode(&result)
		var trailing any
		if decodeErr == nil {
			if err := decoder.Decode(&trailing); err != io.EOF {
				decodeErr = errors.New("virtual USB result has trailing JSON")
			}
		}
		_ = f.Close()
		if decodeErr != nil || result.SchemaVersion != "humanymous.virtual-usb-ladder/v1" ||
			result.Measured != 8 || result.Pass < 0 ||
			result.Residual < 0 || result.Pass+result.Residual > result.Measured ||
			len(result.Failures) > 32 ||
			!equalStrings(result.Engines, []string{"chromium", "firefox"}) ||
			!equalStrings(result.Order, []string{
				"external_input_virtual",
				"external_input_dom_virtual",
				"external_input_vusb",
				"external_input_dom_vusb",
			}) ||
			result.IME.SchemaVersion != "humanymous.ime-composition-vusb/v1" ||
			result.IME.Expected != 6 || result.IME.Measured < 0 ||
			result.IME.Measured > result.IME.Expected || result.IME.Pass < 0 ||
			result.IME.Pass > result.IME.Measured || len(result.IME.Failures) > 32 ||
			!equalStrings(result.IME.Order, []string{"ko-KR", "zh-CN", "ja-JP"}) {
			return nil, errors.New("virtual USB result schema is invalid")
		}
		if result.Canonical &&
			(len(result.Failures) != 0 || result.Pass+result.Residual != 8 ||
				result.IME.Measured != 6 || result.IME.Pass != 6 ||
				len(result.IME.Failures) != 0) {
			return nil, errors.New("canonical virtual USB result is internally inconsistent")
		}
		runs = append(runs, virtualUSBRunView{RecordedAt: candidate.mod, Result: result})
	}
	return runs, nil
}

func equalStrings(actual, expected []string) bool {
	if len(actual) != len(expected) {
		return false
	}
	for i := range expected {
		if actual[i] != expected[i] {
			return false
		}
	}
	return true
}
