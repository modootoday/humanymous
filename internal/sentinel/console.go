package sentinel

import (
	"embed"
	"sort"
	"strings"
)

// console.go embeds the operator console SPA (served at /__hmn/admin/console) and
// its modular stylesheet. The page markup is console.html; its CSS is assembled
// from the SoT-33/SoT-34 SRP partials under styles/ and injected into the page's
// <style id="app-css"> placeholder at load — modular in source, single
// self-contained asset when served (CSP-clean, no external stylesheet request).

//go:embed console.html
var consoleHTML string

//go:embed styles/*.css
var stylesFS embed.FS

// consoleCSS is the assembled stylesheet: the SRP partials concatenated in
// filename order, so the @layer cascade order (declared in 00-layers.css) is
// deterministic.
var consoleCSS = assembleConsoleCSS()

// styledConsoleHTML is the served page with the assembled CSS injected.
var styledConsoleHTML = strings.Replace(consoleHTML,
	`<style id="app-css"></style>`,
	`<style id="app-css">`+consoleCSS+`</style>`, 1)

func assembleConsoleCSS() string {
	ents, err := stylesFS.ReadDir("styles")
	if err != nil {
		return ""
	}
	names := make([]string, 0, len(ents))
	for _, e := range ents {
		if strings.HasSuffix(e.Name(), ".css") {
			names = append(names, e.Name())
		}
	}
	sort.Strings(names)
	var b strings.Builder
	for _, n := range names {
		data, _ := stylesFS.ReadFile("styles/" + n)
		b.Write(data)
		b.WriteByte('\n')
	}
	return b.String()
}
