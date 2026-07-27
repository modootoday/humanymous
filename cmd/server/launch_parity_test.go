package main

import (
	"os"
	"path/filepath"
	"regexp"
	"testing"
)

// launch_parity_test.go is the PLAN-07 R5 parity net. The observatory launcher's
// allowlist (launchProfiles) and the redteam catalog it claims to mirror
// (test/e2e/runner.mjs PROFILES) are maintained in two languages, in two files, by
// hand. When they drift — a profile added to the catalog but not the allowlist, as
// happened with xff_spoof.mjs — the launcher silently cannot run it and there is no
// build or test failure to notice. This test reads the JS catalog and asserts the
// two sets are byte-for-byte identical, so any future drift fails loudly instead.

// catalogProfileLit matches a quoted '<name>.mjs' entry in runner.mjs PROFILES.
var catalogProfileLit = regexp.MustCompile(`'([a-z0-9_]+\.mjs)'`)
var observatoryProfileLit = regexp.MustCompile(`\["([a-z0-9_]+\.mjs)"`)
var observatoryDockerOnlyProfileLit = regexp.MustCompile(`\{id:"([a-z0-9_]+)",executionKind:"docker-external-input"`)

func TestLaunchProfilesMatchCatalog(t *testing.T) {
	// cwd is cmd/server during the test; the catalog lives at the module root.
	root := moduleRootFrom(t)
	runner := filepath.Join(root, "test", "e2e", "runner.mjs")
	src, err := os.ReadFile(runner)
	if err != nil {
		t.Fatalf("read catalog %s: %v", runner, err)
	}

	catalog := map[string]bool{}
	for _, m := range catalogProfileLit.FindAllStringSubmatch(string(src), -1) {
		catalog[m[1]] = true
	}
	if len(catalog) == 0 {
		t.Fatal("parsed zero profiles from runner.mjs — the scan regex is broken (vacuous pass)")
	}

	for name := range catalog {
		if !launchProfiles[name] {
			t.Errorf("catalog profile %q is NOT in launchProfiles (cmd/server/launch.go) — "+
				"the observatory launcher cannot run it (PLAN-07 R5 drift).", name)
		}
	}
	for name := range launchProfiles {
		if !catalog[name] {
			t.Errorf("launchProfiles allows %q which is NOT in the runner.mjs catalog — "+
				"it points at a profile the catalog no longer runs (PLAN-07 R5 drift).", name)
		}
	}
	if len(catalog) != len(launchProfiles) {
		t.Errorf("size mismatch: catalog has %d profiles, launchProfiles has %d", len(catalog), len(launchProfiles))
	}
}

func TestObservatoryCatalogMatchesLaunchProfiles(t *testing.T) {
	root := moduleRootFrom(t)
	page := filepath.Join(root, "web", "playground.html")
	src, err := os.ReadFile(page)
	if err != nil {
		t.Fatalf("read observatory %s: %v", page, err)
	}

	catalog := map[string]bool{}
	for _, m := range observatoryProfileLit.FindAllStringSubmatch(string(src), -1) {
		catalog[m[1]] = true
	}
	if len(catalog) == 0 {
		t.Fatal("parsed zero profiles from playground.html")
	}
	for name := range launchProfiles {
		if !catalog[name] {
			t.Errorf("launcher profile %q is missing from the Observatory catalog", name)
		}
	}
	for name := range catalog {
		if !launchProfiles[name] {
			t.Errorf("Observatory profile %q is not allowed by the launcher", name)
		}
	}
	if len(catalog) != len(launchProfiles) {
		t.Errorf("size mismatch: Observatory has %d profiles, launcher has %d", len(catalog), len(launchProfiles))
	}
}

func TestDockerOnlyProfilesMatchObservatoryAndNeverOverlapLocalLauncher(t *testing.T) {
	wantOrder := []string{
		"external_input_virtual",
		"external_input_dom_virtual",
		"external_input_usb",
		"external_input_dom_usb",
		"external_input_vusb",
		"external_input_dom_vusb",
	}
	want := make(map[string]bool, len(wantOrder))
	for _, name := range wantOrder {
		want[name] = true
	}
	if len(dockerOnlyLaunchProfiles) != len(want) {
		t.Errorf("Docker-only registry has %d profiles, want %d", len(dockerOnlyLaunchProfiles), len(want))
	}
	for name := range want {
		if !dockerOnlyLaunchProfiles[name] {
			t.Errorf("Docker-only registry is missing %q", name)
		}
	}
	for name := range dockerOnlyLaunchProfiles {
		if !want[name] {
			t.Errorf("unexpected Docker-only profile %q", name)
		}
		if launchProfiles[name] {
			t.Errorf("Docker-only profile %q must never enter the local launcher allowlist", name)
		}
	}

	root := moduleRootFrom(t)
	page := filepath.Join(root, "web", "playground.html")
	src, err := os.ReadFile(page)
	if err != nil {
		t.Fatalf("read observatory %s: %v", page, err)
	}
	observatory := map[string]bool{}
	matches := observatoryDockerOnlyProfileLit.FindAllStringSubmatch(string(src), -1)
	for i, m := range matches {
		observatory[m[1]] = true
		if i >= len(wantOrder) || m[1] != wantOrder[i] {
			t.Errorf("Docker-only Observatory order[%d] = %q, want canonical ladder %q", i, m[1], wantOrder)
		}
	}
	for name := range dockerOnlyLaunchProfiles {
		if !observatory[name] {
			t.Errorf("Docker-only profile %q is missing from the Observatory catalog", name)
		}
	}
	for name := range observatory {
		if !dockerOnlyLaunchProfiles[name] {
			t.Errorf("Observatory marks unknown profile %q as Docker-only", name)
		}
	}
	if len(observatory) != len(dockerOnlyLaunchProfiles) {
		t.Errorf("Docker-only size mismatch: Observatory has %d profiles, server has %d", len(observatory), len(dockerOnlyLaunchProfiles))
	}
	if !regexp.MustCompile(`querySelectorAll\("\.rcard\[data-p\]"\)`).Match(src) {
		t.Error("Observatory local-launch event binding is not restricted to cards with data-p")
	}
}

// moduleRootFrom walks up from the test's working directory to the go.mod directory.
func moduleRootFrom(t *testing.T) string {
	t.Helper()
	dir, err := os.Getwd()
	if err != nil {
		t.Fatalf("getwd: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(dir, "go.mod")); err == nil {
			return dir
		}
		parent := filepath.Dir(dir)
		if parent == dir {
			t.Fatal("could not locate module root (no go.mod found walking up)")
		}
		dir = parent
	}
}
