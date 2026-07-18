package report

import (
	"fmt"
	"html"
	"os"
	"strings"
)

// html.go renders a self-contained HTML report (no external assets; theme-aware)
// from a Summary (plan/05 §4). Pure rendering: the caller injects any timestamp.

// RenderHTML writes the report to path. note is a caller-supplied caption
// (e.g. a timestamp) so this function stays deterministic.
func RenderHTML(path string, s Summary, note string) error {
	var b strings.Builder
	b.WriteString(head)
	fmt.Fprintf(&b, `<h1>humanymous · Red vs Blue e2e report</h1><p class="note">%s · policy 1.0.0</p>`, html.EscapeString(note))

	// Metric cards.
	b.WriteString(`<div class="cards">`)
	card(&b, "Bot TPR (blocked)", pct(s.BotTPR), s.BotTPR >= 1.0)
	card(&b, "Human FPR (denied)", pct(s.HumanFPR), s.HumanFPR == 0.0)
	card(&b, "Bot runs", fmt.Sprintf("%d", s.BotRuns), true)
	card(&b, "Human runs", fmt.Sprintf("%d", s.HumanRuns), true)
	b.WriteString(`</div>`)

	// Verdict alert.
	if s.BotTPR < 1.0 || s.HumanFPR > 0 {
		b.WriteString(`<p class="alert">⚠ Not all bots blocked or a human was denied — see table.</p>`)
	} else if s.BotRuns > 0 {
		b.WriteString(`<p class="ok">✓ All bot profiles blocked (DENY/CHALLENGE); no human denied.</p>`)
	}

	// Per-profile table.
	b.WriteString(`<div class="scroll"><table><thead><tr>
<th>Profile</th><th>Type</th><th>Runs</th><th>Verdicts</th><th>Mean risk</th><th>Rules</th><th>Top signals</th></tr></thead><tbody>`)
	for _, p := range s.Profiles {
		typ := "human"
		cls := "human"
		if p.IsBot {
			typ = "bot"
			cls = "bot"
			if p.Blocked == p.Runs {
				cls = "bot blocked"
			}
		}
		top := p.SortedTopHits()
		if len(top) > 4 {
			top = top[:4]
		}
		fmt.Fprintf(&b, `<tr class="%s"><td>%s</td><td>%s</td><td>%d</td><td>%s</td><td>%.1f</td><td>%s</td><td class="mono">%s</td></tr>`,
			cls, html.EscapeString(p.Label), typ, p.Runs,
			verdictBadges(p.Verdicts), p.MeanRisk, mapStr(p.Rules), html.EscapeString(strings.Join(top, ", ")))
	}
	b.WriteString(`</tbody></table></div>`)

	if len(s.Skipped) > 0 {
		b.WriteString(`<h2>Skipped profiles</h2><ul>`)
		for _, sk := range s.Skipped {
			fmt.Fprintf(&b, `<li class="mono">%s</li>`, html.EscapeString(sk))
		}
		b.WriteString(`</ul>`)
	}

	b.WriteString(foot)
	return os.WriteFile(path, []byte(b.String()), 0o644)
}

func card(b *strings.Builder, label, value string, good bool) {
	cls := "card"
	if good {
		cls += " good"
	} else {
		cls += " bad"
	}
	fmt.Fprintf(b, `<div class="%s"><div class="v">%s</div><div class="l">%s</div></div>`,
		cls, html.EscapeString(value), html.EscapeString(label))
}

func verdictBadges(v map[string]int) string {
	var parts []string
	for _, k := range []string{"ALLOW", "CHALLENGE", "DENY", "NO_RESPONSE"} {
		if n := v[k]; n > 0 {
			parts = append(parts, fmt.Sprintf(`<span class="badge %s">%s×%d</span>`, strings.ToLower(k), k, n))
		}
	}
	return strings.Join(parts, " ")
}

func mapStr(m map[string]int) string {
	var parts []string
	for k, v := range m {
		parts = append(parts, fmt.Sprintf("%s×%d", k, v))
	}
	return html.EscapeString(strings.Join(parts, ", "))
}

func pct(f float64) string { return fmt.Sprintf("%.0f%%", f*100) }

const head = `<!doctype html><meta charset="utf-8"><title>humanymous e2e report</title>
<meta name="viewport" content="width=device-width, initial-scale=1"><style>
:root{--bg:#fff;--fg:#1a1a1a;--mut:#666;--line:#e2e2e2;--good:#0a7f3f;--bad:#c02626;--chip:#f0f0f0}
@media(prefers-color-scheme:dark){:root{--bg:#14161a;--fg:#e8e8e8;--mut:#9aa;--line:#2a2e35;--good:#3ecf8e;--bad:#ff6b6b;--chip:#22262d}}
body{font:15px/1.5 system-ui,sans-serif;margin:0;padding:2rem;max-width:1100px;margin:auto;background:var(--bg);color:var(--fg)}
h1{font-size:1.5rem;margin:0 0 .2rem}h2{font-size:1.1rem;margin-top:2rem}
.note{color:var(--mut);margin-top:0}
.cards{display:flex;gap:1rem;flex-wrap:wrap;margin:1.5rem 0}
.card{flex:1;min-width:140px;border:1px solid var(--line);border-radius:10px;padding:1rem}
.card .v{font-size:1.8rem;font-weight:700}.card .l{color:var(--mut);font-size:.85rem}
.card.good .v{color:var(--good)}.card.bad .v{color:var(--bad)}
.ok{color:var(--good);font-weight:600}.alert{color:var(--bad);font-weight:600}
.scroll{overflow-x:auto}table{border-collapse:collapse;width:100%;font-size:.9rem}
th,td{text-align:left;padding:.5rem .6rem;border-bottom:1px solid var(--line);vertical-align:top}
th{color:var(--mut);font-weight:600}
tr.bot.blocked td:first-child::before{content:"🛡 "}tr.human td:first-child::before{content:"👤 "}
.mono{font-family:ui-monospace,monospace;font-size:.82rem;color:var(--mut)}
.badge{display:inline-block;padding:.1rem .4rem;border-radius:6px;font-size:.75rem;background:var(--chip)}
.badge.deny{background:#f5d3d3;color:#8b1a1a}.badge.challenge{background:#f6e8c8;color:#7a5b12}
.badge.allow{background:#cdeede;color:#0a5f30}
@media(prefers-color-scheme:dark){.badge.deny{background:#3a1e1e;color:#ff9c9c}.badge.challenge{background:#3a2f18;color:#f0d488}.badge.allow{background:#183526;color:#7fe0ad}}
ul{padding-left:1.2rem}</style>`

const foot = `<p class="note" style="margin-top:2rem">humanymous — defensive anti-bot detection reference (educational). See sots/ and plan/.</p>`
