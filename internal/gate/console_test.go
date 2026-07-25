package gate

import (
	"os/exec"
	"strings"
	"testing"
)

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

// The console is a single embedded document, so one malformed template
// expression prevents every view from loading. Ask Node to parse the inline
// script when it is available in the test environment.
func TestConsoleJavaScriptSyntax(t *testing.T) {
	node, err := exec.LookPath("node")
	if err != nil {
		t.Skip("node is not installed")
	}

	const openTag, closeTag = "<script>", "</script>"
	start := strings.Index(consoleHTML, openTag)
	end := strings.LastIndex(consoleHTML, closeTag)
	if start < 0 || end < 0 || end <= start {
		t.Fatal("console inline script not found")
	}

	cmd := exec.Command(node, "--check", "-")
	cmd.Stdin = strings.NewReader(consoleHTML[start+len(openTag) : end])
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("console JavaScript syntax: %v\n%s", err, output)
	}
}
