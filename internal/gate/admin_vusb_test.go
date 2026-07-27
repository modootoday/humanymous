package gate

import (
	"encoding/json"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func writeVirtualUSBResult(t *testing.T, root, run string, mutate func(map[string]any)) {
	t.Helper()
	dir := filepath.Join(root, run)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		t.Fatal(err)
	}
	body := map[string]any{
		"schemaVersion": "humanymous.virtual-usb-ladder/v1",
		"canonical":     true,
		"engines":       []string{"chromium", "firefox"},
		"order": []string{
			"external_input_virtual",
			"external_input_dom_virtual",
			"external_input_vusb",
			"external_input_dom_vusb",
		},
		"measured": 8,
		"pass":     7,
		"residual": 1,
		"failures": []string{},
		"ime": map[string]any{
			"schemaVersion": "humanymous.ime-composition-vusb/v1",
			"order":         []string{"ko-KR", "zh-CN", "ja-JP"},
			"expected":      6,
			"measured":      6,
			"pass":          6,
			"failures":      []string{},
		},
	}
	if mutate != nil {
		mutate(body)
	}
	raw, err := json.Marshal(body)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "ladder-result.json"), raw, 0o600); err != nil {
		t.Fatal(err)
	}
}

func TestAdminVirtualUSBDisabledByDefault(t *testing.T) {
	s := &Server{}
	rr := httptest.NewRecorder()
	s.adminVirtualUSB(rr)
	if rr.Code != 200 {
		t.Fatalf("status = %d", rr.Code)
	}
	var body struct {
		Enabled bool `json:"enabled"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	if body.Enabled {
		t.Fatal("virtual USB result view must be disabled without an explicit directory")
	}
}

func TestAdminVirtualUSBReadsOnlyBoundedTerminalSummary(t *testing.T) {
	root := t.TempDir()
	writeVirtualUSBResult(t, root, "run-1", nil)
	s := &Server{cfg: Config{VirtualUSBResultsDir: root}}
	rr := httptest.NewRecorder()
	s.adminVirtualUSB(rr)
	if rr.Code != 200 {
		t.Fatalf("status = %d", rr.Code)
	}
	var body struct {
		Enabled bool                `json:"enabled"`
		Healthy bool                `json:"healthy"`
		Runs    []virtualUSBRunView `json:"runs"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	if !body.Enabled || !body.Healthy || len(body.Runs) != 1 {
		t.Fatalf("unexpected body: %+v", body)
	}
	if !body.Runs[0].Result.Canonical || body.Runs[0].Result.Measured != 8 {
		t.Fatalf("unexpected result: %+v", body.Runs[0].Result)
	}
}

func TestAdminVirtualUSBRejectsUnboundedOrUnknownResultSchema(t *testing.T) {
	root := t.TempDir()
	writeVirtualUSBResult(t, root, "run-1", func(body map[string]any) {
		body["rawDevicePaths"] = []string{"/dev/input/event0"}
	})
	s := &Server{cfg: Config{VirtualUSBResultsDir: root}}
	rr := httptest.NewRecorder()
	s.adminVirtualUSB(rr)
	var body struct {
		Healthy bool `json:"healthy"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	if body.Healthy {
		t.Fatal("unknown/raw result fields must fail the read-only view closed")
	}
}
