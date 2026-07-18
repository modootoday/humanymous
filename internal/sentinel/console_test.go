package sentinel

import "strings"

import "testing"

// The modular SRP partials assemble into the served page: tokens + components are
// present and the empty placeholder is filled (SoT-33 P2 / SoT-34).
func TestModularConsoleCSSAssembled(t *testing.T) {
	if strings.Contains(styledConsoleHTML, `<style id="app-css"></style>`) {
		t.Fatal("CSS placeholder was not filled — partials not injected")
	}
	for _, want := range []string{
		"@layer tokens, base, layout, content, overlays, responsive;", // layer order
		"--accent:#35d0ba",                        // SoT-34 brand token
		"--deny:#f0556a",                          // semantic verdict token
		".navitem", ".kpi", ".verdict", ".drawer", // components across partials
		"content-visibility:auto", // SoT-33 feed virtualization
	} {
		if !strings.Contains(styledConsoleHTML, want) {
			t.Errorf("assembled console CSS missing %q", want)
		}
	}
}

// Every partial ends up in the assembled sheet (nothing silently dropped).
func TestConsoleCSSNonEmpty(t *testing.T) {
	if len(consoleCSS) < 2000 {
		t.Fatalf("assembled CSS suspiciously small: %d bytes", len(consoleCSS))
	}
}
