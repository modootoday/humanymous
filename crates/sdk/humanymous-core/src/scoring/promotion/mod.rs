use crate::scoring::{policy::SessionVerdict, trace::HardRuleEval};
use crate::signals::{observation::Signal, report::CrossCheck};
use std::collections::BTreeMap;

pub(in crate::scoring) struct RuleContext<'a> {
    pub signals: Vec<Signal>,
    pub cross: Vec<CrossCheck>,
    pub has_client: bool,
    pub browser_claim: bool,
    pub modes: &'a BTreeMap<String, String>,
    pub net_policy: Option<&'a BTreeMap<String, String>>,
}
impl RuleContext<'_> {
    fn fired(&self, id: &str) -> bool {
        self.signals.iter().any(|signal| {
            signal.id == id && matches!(signal.verdict.as_str(), "BOT" | "SUSPICIOUS")
        })
    }
    fn bot(&self, id: &str) -> bool {
        self.signals
            .iter()
            .any(|signal| signal.id == id && signal.verdict.as_str() == "BOT")
    }
    fn any_fired(&self, ids: &[&str]) -> bool {
        ids.iter().any(|id| self.fired(id))
    }
    fn any_bot(&self, ids: &[&str]) -> bool {
        ids.iter().any(|id| self.bot(id))
    }
    fn cross_fail(&self, id: &str) -> bool {
        self.cross
            .iter()
            .any(|check| check.id == id && !check.consistent)
    }
    fn net_mode(&self, class: &str) -> &str {
        let Some(policy) = self.net_policy else {
            return "enforce";
        };
        if let Some(mode) = policy.get(class).filter(|mode| !mode.is_empty()) {
            return mode;
        }
        if class == "net.correlation" {
            "enforce"
        } else {
            "monitor"
        }
    }
}
struct Rule {
    id: &'static str,
    verdict: SessionVerdict,
    predicate: fn(&RuleContext<'_>) -> bool,
    why: &'static str,
}
use SessionVerdict::{Challenge, Deny};

static RULES: [Rule; 24] = [
    Rule {
        id: "HR-1",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&[
                "l1.artifact.selenium",
                "l1.artifact.playwright",
                "l1.artifact.phantom",
            ])
        },
        why: "hard automation artifact (Selenium/Puppeteer/Playwright)",
    },
    Rule {
        id: "HR-2",
        verdict: Deny,
        predicate: |context| context.cross_fail("x.ua_vs_ja4") && context.cross_fail("x.ua_vs_h2"),
        why: "UA claims a browser but TLS + HTTP/2 both resolve to a different engine",
    },
    Rule {
        id: "HR-3",
        verdict: Deny,
        predicate: |context| {
            context.bot("l4.event.untrusted")
                && context.any_fired(&["l1.navigator.webdriver", "l1.cdp.runtime_enable"])
        },
        why: "synthetic (untrusted) events plus a webdriver/CDP tell",
    },
    Rule {
        id: "HR-4",
        verdict: Deny,
        predicate: |context| {
            context.fired("l1.navigator.webdriver") && context.fired("l3.integrity.native_tostring")
        },
        why: "webdriver flag plus a patched native getter hiding it",
    },
    Rule {
        id: "HR-5",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&["l5.rit.header_tampered", "l5.rit.invalid_hmac"])
                && context.cross_fail("x.ua_vs_ja4")
        },
        why: "RIT tamper/invalid HMAC together with a TLS engine mismatch",
    },
    Rule {
        id: "HR-6",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&["l3.guard.native_hooked", "l3.guard.proto_polluted"])
                && context.any_fired(&["l1.navigator.webdriver", "l1.cdp.runtime_enable"])
        },
        why: "runtime hooking / prototype pollution plus a webdriver/CDP tell",
    },
    Rule {
        id: "HR-7",
        verdict: Deny,
        predicate: |context| {
            context.bot("l1.ua.headless_token")
                && context.any_fired(&[
                    "l1.navigator.webdriver",
                    "l1.window.outer_eq_inner",
                    "l3.integrity.native_tostring",
                ])
        },
        why: "headless browser plus a second automation indicator",
    },
    Rule {
        id: "HR-8",
        verdict: Deny,
        predicate: |context| {
            context.fired("l3.integrity.native_tostring")
                && (context.bot("l1.ua.headless_token")
                    || context
                        .any_fired(&["l1.window.outer_eq_inner", "l3.integrity.iframe_recheck"]))
        },
        why: "patched native getter in a chromeless/headless window (stealth tool)",
    },
    Rule {
        id: "HR-9",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&["l1.cdp.runtime_enable", "l1.cdp.proxy_leak"])
                && (context.any_fired(&[
                    "l1.navigator.webdriver",
                    "l1.window.outer_eq_inner",
                    "l3.integrity.native_tostring",
                    "l4.event.no_interaction",
                ]) || context.bot("l1.ua.headless_token"))
        },
        why: "a CDP leak together with any automation hint",
    },
    Rule {
        id: "HR-13",
        verdict: Deny,
        predicate: |context| {
            context.bot("l3.guard.console_disabled")
                && (context.any_fired(&[
                    "l1.window.outer_eq_inner",
                    "l4.event.no_interaction",
                    "l3.integrity.native_tostring",
                ]) || context.bot("l1.ua.headless_token"))
        },
        why: "a disabled Console API (Patchright) plus a chromeless/automation tell",
    },
    Rule {
        id: "HR-10",
        verdict: Challenge,
        predicate: |context| !context.has_client,
        why: "no client-side (WASM/JS) signals at all",
    },
    Rule {
        id: "HR-16",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&[
                "l5.rit.header_tampered",
                "l5.rit.invalid_hmac",
                "l5.rit.body_mismatch",
            ])
        },
        why: "a RIT token that fails the body HMAC (request-body tamper)",
    },
    Rule {
        id: "HR-14",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&["l5.traffic.engine_rotation", "l5.traffic.ja4_rotation"])
        },
        why: "the TLS fingerprint rotated within one session",
    },
    Rule {
        id: "HR-18",
        verdict: Deny,
        predicate: |context| context.cross_fail("x.browser_no_js"),
        why: "a browser UA that delivered zero client-side JS evidence (HTTP parrot)",
    },
    Rule {
        id: "HR-20",
        verdict: Deny,
        predicate: |context| {
            context.bot("l4.mouse.click_no_trajectory")
                && (context.any_bot(&[
                    "l4.agent.burst_silence",
                    "l4.key.machine_speed",
                    "l1.cdp.proxy_leak",
                    "l1.cdp.runtime_enable",
                ]) || context.fired("l4.mouse.coalesced_synthetic"))
        },
        why: "AI browser-agent signature (teleport click + a second agent/CDP tell)",
    },
    Rule {
        id: "HR-19",
        verdict: Deny,
        predicate: |context| {
            context.bot("l7.pow.too_fast")
                || (context.net_mode("net.correlation") != "monitor"
                    && context.any_bot(&[
                        "l5.correlation.proxy_rotation",
                        "l5.correlation.fp_churn_proxy",
                    ]))
        },
        why: "NET-POLICY: proxy rotation / fp-churn (audit+admin; score-exempt residual)",
    },
    Rule {
        id: "HR-21",
        verdict: Deny,
        predicate: |context| {
            context.any_bot(&["l5.h2dos.rapid_reset", "l5.h2dos.continuation_flood"])
        },
        why: "HTTP-2 DoS (Rapid Reset / CONTINUATION flood)",
    },
    Rule {
        id: "HR-15",
        verdict: Deny,
        predicate: |context| {
            context.fired("l5.traffic.ua_rotation")
                && (context.bot("l5.traffic.ja4_rotation") || context.fired("l5.traffic.ip_hop"))
        },
        why: "multi-axis rotation (UA changed together with TLS/IP rotation)",
    },
    Rule {
        id: "HR-17",
        verdict: Challenge,
        predicate: |context| context.fired("l5.rit.stale_replay") || context.bot("l5.rit.absent"),
        why: "a replayed or absent RIT token on an API call",
    },
    Rule {
        id: "HR-12",
        verdict: Challenge,
        predicate: |context| context.fired("l4.event.no_interaction"),
        why: "zero interaction over the observation window (heuristic — can catch some humans)",
    },
    Rule {
        id: "HR-11",
        verdict: Challenge,
        predicate: |context| {
            context.browser_claim
                && context.fired("l5.ip.datacenter_asn")
                && !context.cross_fail("x.ua_vs_ja4")
                && !context.cross_fail("x.ua_vs_h2")
        },
        why: "a consistent browser from a datacenter ASN",
    },
    Rule {
        id: "HR-22",
        verdict: Challenge,
        predicate: |context| {
            context.any_bot(&[
                "l2.adv.audio_24k",
                "l2.adv.webgpu_mismatch",
                "l2.adv.mobile_inconsistent",
            ])
        },
        why: "near-ceiling advanced residual (audio 24k / GPU mismatch / mobile-desktop profile)",
    },
    Rule {
        id: "HR-23",
        verdict: Challenge,
        predicate: |context| {
            context.browser_claim
                && [
                    "l2.adv.no_widevine",
                    "l2.adv.no_media_devices",
                    "l2.adv.no_voices",
                ]
                .iter()
                .all(|id| context.fired(id))
        },
        why: "Chrome UA with Chromium-container residual (no Widevine/media/voices)",
    },
    Rule {
        id: "HR-24",
        verdict: Challenge,
        predicate: network_residual,
        why: "NET-POLICY: network residual (proxy/VPN/Tor) — audit+admin; score-exempt",
    },
];

fn network_residual(context: &RuleContext<'_>) -> bool {
    let classes: &[(&str, &[&str], bool, bool)] = &[
        ("net.proxy.hop", &["l5.header.proxy_hop"], false, true),
        ("net.vpn", &["l5.proxy.vpn_webrtc_leak"], false, true),
        (
            "net.proxy.spoof",
            &["l5.header.client_ip_spoof", "l5.header.forwarded_private"],
            false,
            true,
        ),
        ("net.proxy.anon", &["l5.proxy.anon_chain"], false, true),
        (
            "net.tor",
            &["l5.proxy.tor_circuit", "l5.ip.tor_exit"],
            false,
            false,
        ),
        (
            "net.h2.spoof",
            &[
                "l5.http2.unknown_under_browser",
                "l5.http2.browser_settings_atypical",
                "l5.http2.flow_control_atypical",
                "l5.http2.priority_atypical",
            ],
            true,
            false,
        ),
        ("net.header.order", &["l5.header.order"], true, false),
        ("net.tls.pq", &["l5.tls.pq_keyshare"], true, false),
        ("net.tls.alps", &["l5.tls.alps_absent"], true, false),
        (
            "net.tls.certcomp",
            &["l5.tls.cert_compression_absent"],
            true,
            false,
        ),
        ("net.http.h1", &["l5.http.browser_tls_over_h1"], true, false),
        ("net.tls.ech", &["l5.tls.ech_absent"], true, false),
    ];
    if classes.iter().any(|(class, ids, browser, bot)| {
        context.net_mode(class) == "enforce"
            && (!browser || context.browser_claim)
            && if *bot {
                context.any_bot(ids)
            } else {
                context.any_fired(ids)
            }
    }) {
        return true;
    }
    context.net_mode("net.proxy.hop") == "enforce"
        && ((context.fired("l5.header.xff_multi_hop") && context.fired("l5.traffic.ip_hop"))
            || (context.browser_claim && context.fired("l5.ip.proxy_vpn_tor")))
}
fn rule_mode<'a>(context: &'a RuleContext<'_>, id: &str) -> &'a str {
    context
        .modes
        .get(id)
        .filter(|mode| !mode.is_empty())
        .map(String::as_str)
        .unwrap_or("enforce")
}
pub(in crate::scoring) fn apply(
    context: &RuleContext<'_>,
) -> Option<(SessionVerdict, &'static str)> {
    RULES
        .iter()
        .find(|rule| {
            !matches!(rule_mode(context, rule.id), "monitor" | "off") && (rule.predicate)(context)
        })
        .map(|rule| (rule.verdict, rule.id))
}
pub(in crate::scoring) fn evaluations(context: &RuleContext<'_>) -> Vec<HardRuleEval> {
    let mut won = false;
    RULES
        .iter()
        .map(|rule| {
            let matched = (rule.predicate)(context);
            let winner = matched && rule_mode(context, rule.id) == "enforce" && !won;
            won |= winner;
            HardRuleEval {
                rule: rule.id.into(),
                verdict: rule.verdict.as_str().into(),
                why: rule.why.into(),
                matched,
                won: winner,
            }
        })
        .collect()
}
