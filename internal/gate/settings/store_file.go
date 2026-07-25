package settings

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// FileStore persists the active overlay + last-known-good (SoT-39 P1).
type FileStore struct {
	mu      sync.RWMutex
	dir     string
	active  *Overlay
	lkg     *Overlay // last successfully validated active
	loadErr error
}

// NewFileStore loads from dir (creates if needed). Corrupt active → try LKG → empty.
func NewFileStore(dir string) (*FileStore, error) {
	if dir == "" {
		return nil, errors.New("settings: empty store dir")
	}
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return nil, err
	}
	s := &FileStore{dir: dir}
	s.active, s.loadErr = s.readFile(s.activePath())
	if s.loadErr != nil || (s.active != nil && Validate(s.active) != nil) {
		lkg, lerr := s.readFile(s.lkgPath())
		if lerr == nil && lkg != nil && Validate(lkg) == nil {
			s.active = lkg
			s.lkg = lkg
			s.loadErr = fmt.Errorf("active corrupt; loaded LKG: %w", s.loadErr)
		} else {
			s.active = nil
			if s.loadErr == nil {
				s.loadErr = errors.New("active overlay invalid and no LKG")
			}
		}
	} else if s.active != nil {
		s.lkg = cloneOverlay(s.active)
	}
	return s, nil
}

func (s *FileStore) activePath() string { return filepath.Join(s.dir, "settings.overlay.v1.json") }
func (s *FileStore) lkgPath() string    { return filepath.Join(s.dir, "settings.overlay.lkg.json") }

// Active returns the in-memory active overlay (nil = empty).
func (s *FileStore) Active() *Overlay {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.active
}

// LoadError is non-nil if boot recovered from LKG or discarded corrupt state.
func (s *FileStore) LoadError() error {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.loadErr
}

// SetActive validates and persists overlay as active (+ LKG). nil clears.
func (s *FileStore) SetActive(o *Overlay) error {
	if o != nil {
		if err := Validate(o); err != nil {
			return err
		}
		o.Status = "active"
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if o == nil {
		_ = os.Remove(s.activePath())
		s.active = nil
		return nil
	}
	b, err := json.MarshalIndent(o, "", "  ")
	if err != nil {
		return err
	}
	if err := os.WriteFile(s.activePath(), b, 0o600); err != nil {
		return err
	}
	_ = os.WriteFile(s.lkgPath(), b, 0o600)
	s.active = cloneOverlay(o)
	s.lkg = cloneOverlay(o)
	s.loadErr = nil
	return nil
}

func (s *FileStore) readFile(path string) (*Overlay, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}
		return nil, err
	}
	var o Overlay
	if err := json.Unmarshal(b, &o); err != nil {
		return nil, err
	}
	return &o, nil
}

func cloneOverlay(o *Overlay) *Overlay {
	if o == nil {
		return nil
	}
	b, _ := json.Marshal(o)
	var c Overlay
	_ = json.Unmarshal(b, &c)
	return &c
}

// Schema is the static Settings schema for GET /settings/schema.
func Schema() map[string]any {
	ch, dn, lc := ScoringDefaults()
	critHR := make([]string, 0, len(IntegrityCriticalEngineHRs))
	for id := range IntegrityCriticalEngineHRs {
		critHR = append(critHR, id)
	}
	return map[string]any{
		"schemaVersion":              "1.0.0",
		"gates":                      KnownGates,
		"netPolicyClasses":           NetPolicyClasses,
		"integrityCriticalHardRules": critHR,
		"integrityCriticalGates": []string{
			"gate.smuggle", "gate.spoof_header", "gate.verdict_token",
		},
		"modes": map[string]any{
			"gates":     []string{"enforce", "monitor", "shadow", "off"},
			"hardRules": []string{"enforce", "monitor"},
			"netPolicy": []string{"enforce", "monitor", "shadow"},
		},
		"scoringBounds": map[string]any{
			"challengeAt": map[string]float64{"min": 10, "max": 60, "default": ch},
			"denyAt":      map[string]float64{"min": 40, "max": 95, "default": dn},
			"layerCap":    map[string]float64{"min": 30, "max": 80, "default": lc},
		},
		"weightMultiplier":  map[string]float64{"min": 0, "max": 2, "step": 0.05},
		"emptyOverlayMeans": "no active overlay; preserve built-in and startup behavior",
		"notes": []string{
			"Detection measurements remain observable; enforcement changes are governed",
			"Transport-pattern evidence never auto-enforces",
			"Protection-reducing changes require a distinct Approver",
			"Integrity-reducing changes also require an explicit confirmation phrase",
			"Preview uses aggregates only and makes no numeric claim below 50 samples",
			"No active overlay preserves built-in and startup behavior",
			"Settings do not overcome the coherent real-browser and human-paced automation boundary",
		},
	}
}
