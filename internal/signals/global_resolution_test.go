package signals

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// global_resolution_test.go is the PLAN-07 R10 producer-side anti-typo net, the
// complement to the consumer-side resolution test in internal/scoring. New()
// deliberately tolerates an UNKNOWN id (weight-0 fallback) so a bad emit degrades
// gracefully instead of panicking in production — but that same tolerance means a
// mistyped PRODUCER id (a signals.New / add() call site) silently emits a dead,
// weight-0 signal with a green build. This test scans every non-test source file in
// the module for signal-id string literals and asserts each one resolves in the
// registry, turning that silent degradation into a loud test failure.
//
// Scope note: test files are excluded on purpose — some tests intentionally use
// unregistered ids to exercise the UNKNOWN fallback itself.

// sigIDLiteral matches a quoted signal id of the shape lN.group.item (item may
// carry further dots), the canonical id format enforced by the registry.
var sigIDLiteral = regexp.MustCompile(`"(l[1-7]\.[a-z0-9_]+\.[a-z0-9_.]+)"`)

func TestAllSourceSignalIDsResolve(t *testing.T) {
	root := moduleRoot(t)
	refs := map[string][]string{} // id -> files referencing it

	err := filepath.WalkDir(root, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			// Skip vendored / VCS / build-output trees.
			base := d.Name()
			if base == ".git" || base == "vendor" || base == "node_modules" || base == ".cache" {
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(path, ".go") || strings.HasSuffix(path, "_test.go") {
			return nil
		}
		src, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(root, path)
		for _, m := range sigIDLiteral.FindAllStringSubmatch(string(src), -1) {
			refs[m[1]] = append(refs[m[1]], filepath.ToSlash(rel))
		}
		return nil
	})
	if err != nil {
		t.Fatalf("walk module: %v", err)
	}
	if len(refs) == 0 {
		t.Fatal("found no signal-id literals in the module — the scan regex is broken, " +
			"which would make this net pass vacuously")
	}

	unresolved := 0
	for id, files := range refs {
		if _, ok := Lookup(id); !ok {
			unresolved++
			t.Errorf("signal id %q is referenced/emitted but NOT registered (in %s) — "+
				"a typo here silently emits a dead weight-0 signal (PLAN-07 R10). "+
				"Register it in registry.go or fix the id.", id, strings.Join(dedup(files), ", "))
		}
	}
	t.Logf("scanned %d distinct signal-id literals across the module; %d unresolved", len(refs), unresolved)
}

// moduleRoot walks up from the test's working directory to the directory holding go.mod.
func moduleRoot(t *testing.T) string {
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

func dedup(xs []string) []string {
	seen := map[string]bool{}
	out := xs[:0]
	for _, x := range xs {
		if !seen[x] {
			seen[x] = true
			out = append(out, x)
		}
	}
	return out
}
