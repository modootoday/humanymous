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
