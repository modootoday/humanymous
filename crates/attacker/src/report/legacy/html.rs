use super::{Error, FIELD_BYTES, MAX_ROWS, OUTPUT_BYTES, Summary};
use std::fmt::{self, Write};

struct Html {
    text: String,
    limit: usize,
}
impl Write for Html {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self
            .text
            .len()
            .checked_add(value.len())
            .is_none_or(|size| size > self.limit)
        {
            return Err(fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}
impl Html {
    fn literal(&mut self, value: &str) -> Result<(), Error> {
        self.write_str(value).map_err(|_| Error::OutputTooLarge)
    }
    fn escaped(&mut self, value: &str) -> Result<(), Error> {
        if value.len() > FIELD_BYTES {
            return Err(Error::Limit);
        }
        for ch in value.chars() {
            match ch {
                '&' => self.literal("&amp;")?,
                '<' => self.literal("&lt;")?,
                '>' => self.literal("&gt;")?,
                '"' => self.literal("&quot;")?,
                '\'' => self.literal("&#39;")?,
                other => self.write_char(other).map_err(|_| Error::OutputTooLarge)?,
            }
        }
        Ok(())
    }
}
macro_rules! add {
    ($out:expr, $($arg:tt)*) => { write!($out, $($arg)*).map_err(|_| Error::OutputTooLarge)? };
}

fn card(out: &mut Html, label: &str, value: &str) -> Result<(), Error> {
    out.literal("<div class=card><strong>")?;
    out.escaped(value)?;
    out.literal("</strong><span>")?;
    out.escaped(label)?;
    out.literal("</span></div>")
}
fn rate(value: f64, population: usize) -> String {
    if population == 0 {
        "N/A (no observations)".into()
    } else {
        format!("{:.0}%", value * 100.0)
    }
}

/// Render deterministic escaped legacy observations, never a product-green result.
pub fn render(summary: &Summary, note: &str) -> Result<String, Error> {
    render_with_limit(summary, note, OUTPUT_BYTES)
}

pub(super) fn render_with_limit(
    summary: &Summary,
    note: &str,
    limit: usize,
) -> Result<String, Error> {
    if note.len() > FIELD_BYTES
        || summary.profiles.len() > MAX_ROWS
        || summary.notices.len() > MAX_ROWS
        || !summary.bot_tpr.is_finite()
        || !summary.human_fpr.is_finite()
    {
        return Err(Error::Limit);
    }
    let mut out = Html {
        text: String::new(),
        limit: limit.min(OUTPUT_BYTES),
    };
    out.literal(HEAD)?;
    out.literal("<h1>humanymous · Legacy e2e observations</h1><p class=note>")?;
    out.escaped(note)?;
    out.literal(" · legacy template policy 1.0.0</p><p class=notice>Declared-label metrics only. Unsupported coverage is not encoded. Human/AT participation is unverified. This report is not a product pass.</p><div class=cards>")?;
    card(
        &mut out,
        "Bot TPR (DENY + CHALLENGE)",
        &rate(summary.bot_tpr, summary.bot_runs),
    )?;
    card(
        &mut out,
        "Human FPR (DENY only)",
        &rate(summary.human_fpr, summary.human_runs),
    )?;
    card(
        &mut out,
        "Eligible bot observations",
        &summary.bot_runs.to_string(),
    )?;
    card(
        &mut out,
        "Eligible declared-human observations",
        &summary.human_runs.to_string(),
    )?;
    out.literal("</div><h2>Complete input accounting</h2><dl>")?;
    for (name, count) in [
        ("Scheduled rows", summary.accounting.scheduled),
        ("Legacy metric-eligible rows", summary.accounting.eligible),
        ("Skipped rows", summary.accounting.skipped),
        ("Non-skipped error rows", summary.accounting.errors),
        (
            "Error-free rows missing a label",
            summary.accounting.missing_label,
        ),
        (
            "Error-marked rows (may overlap skipped)",
            summary.accounting.error_marked,
        ),
        ("Declared-human CHALLENGE", summary.human_challenges),
        ("Declared-human non-ALLOW", summary.human_non_allow),
    ] {
        out.literal("<dt>")?;
        out.escaped(name)?;
        add!(out, "</dt><dd>{count}</dd>");
    }
    out.literal("<dt>Unsupported rows</dt><dd>not encoded (unknown, not zero)</dd></dl>")?;
    if summary.human_non_allow > 0 {
        out.literal("<p class=alert>Some declared-human observations did not ALLOW. CHALLENGE is not successful human access.</p>")?;
    }
    if summary.accounting.skipped + summary.accounting.errors + summary.accounting.missing_label > 0
    {
        out.literal("<p class=alert>Incomplete rows are retained above and excluded only from the frozen legacy metric denominators.</p>")?;
    }
    out.literal("<div class=scroll><table><thead><tr><th>Profile label</th><th>Declared type</th><th>Runs</th><th>Verdicts</th><th>Mean risk</th><th>Rules</th><th>Top signals</th></tr></thead><tbody>")?;
    for profile in &summary.profiles {
        if !profile.mean_risk.is_finite() || profile.runs == 0 {
            return Err(Error::Record);
        }
        out.literal("<tr><td>")?;
        out.escaped(&profile.label)?;
        add!(
            out,
            "</td><td>{}</td><td>{}</td><td>",
            if profile.is_bot { "bot" } else { "human" },
            profile.runs
        );
        let known = ["ALLOW", "CHALLENGE", "DENY", "NO_RESPONSE"];
        for key in known.iter().copied().chain(
            profile
                .verdicts
                .keys()
                .map(String::as_str)
                .filter(|key| !known.contains(key)),
        ) {
            if let Some(count) = profile.verdicts.get(key) {
                out.literal("<span class=badge>")?;
                out.escaped(key)?;
                add!(out, "×{count}</span> ");
            }
        }
        add!(out, "</td><td>{:.1}</td><td>", profile.mean_risk);
        for (rule, count) in &profile.rules {
            out.escaped(rule)?;
            add!(out, "×{count} ");
        }
        out.literal("</td><td>")?;
        for (index, (id, count)) in profile.sorted_top_hits().into_iter().take(4).enumerate() {
            if index > 0 {
                out.literal(", ")?;
            }
            out.escaped(id)?;
            add!(out, "×{count}");
        }
        out.literal("</td></tr>")?;
    }
    out.literal("</tbody></table></div>")?;
    if !summary.notices.is_empty() {
        out.literal("<h2>Skipped, error and unlabelled rows</h2><ul>")?;
        for notice in &summary.notices {
            out.literal("<li>")?;
            out.escaped(&notice.profile)?;
            out.literal(" · ")?;
            out.escaped(&notice.label)?;
            out.literal(" · ")?;
            out.escaped(notice.code)?;
            out.literal("</li>")?;
        }
        out.literal("</ul>")?;
    }
    out.literal("<p class=note>Frozen legacy observations, not live participant or product qualification. Raw error and reason text is omitted.</p></body></html>\n")?;
    Ok(out.text)
}

const HEAD: &str = r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'"><title>humanymous legacy e2e observations</title><style>
:root{color-scheme:light dark;font:15px/1.5 system-ui,sans-serif}body{max-width:1100px;margin:auto;padding:2rem}.note{opacity:.7}.notice{border:1px solid;padding:1rem}.alert{color:#b33}.cards{display:flex;flex-wrap:wrap;gap:1rem}.card{border:1px solid #888;padding:1rem;flex:1}.card strong,.card span{display:block}.scroll{overflow-x:auto}table{border-collapse:collapse;width:100%}th,td{text-align:left;padding:.5rem;border-bottom:1px solid #888;vertical-align:top}.badge{display:inline-block;border:1px solid #888;padding:0 .3rem}dt{font-weight:bold}dd{margin-bottom:.4rem}
</style></head><body>"#;
