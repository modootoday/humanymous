---
id: sot:root/automation-evidence
title: Automation evidence research and tracking
status: active
created: 2026-11-06T21:41:01Z
updated: 2026-11-06T21:48:59Z
verify: [manual]
rel:
  depends_on: [sot:root/rust-migration, sot:root/adapter-ecosystem]
---

# Automation evidence research and tracking

## Rule

Version 0.1.0; researched on 2026-11-07 KST. This is a research/design contract,
not measured vendor blocking, production tracking implementation or permission
to change frozen weights, hard-rule predicates or verdict bands.

### Evidence classification and threat scope

Record provider/product, official source URL, checked date, document maturity,
fact/provider claim/external observation/design inference/unconfirmed status,
coverage and next verification step. Separate model capability, execution host,
egress, crawler purpose, authenticated operator, user authorization and actual
observed behavior. Do not manufacture actor CIDRs, unique fingerprints, malicious
intent or robots violations. No third-party probing is permitted.

| Subject | Officially documented operating model | Defensible consequence |
| --- | --- | --- |
| Grok Bot | Persistent cloud computer with browser/terminal/files/shared sessions; default shared static egress, optional user-desktop routing | Model both browser and direct API activity; datacenter blocking cannot cover desktop egress; no public CIDR was verified |
| Meta Muse/Muse Spark | Browsing/connected apps/background multi-step work and computer-use scripts/UI/batch actions | Hybrid execution is in scope; authoritative site-visible UA/IP/signature identity remains unverified; model capability is not a measured bypass |
| Pickle candidate, pickle.com | Separate real browser, login vault, persistent delegated tasks and possible user handoff | Test real account/human-agent handoff; distinguish education PICL. Korean legal/regional identity was not independently established |
| OpenAI/Anthropic/Gemini computer-use APIs | Customer-executed screenshot/action/browser loops | Runtime/egress need not be the model provider's network; model API identity does not identify visiting traffic |
| Microsoft computer use | Windows/hosted browser execution environments | Corporate VDI and legitimate automation are controls, not automatically malicious identities |
| Browser hosting/proxies | Cloud, static/custom and residential egress options | Residential IP is not proof of a person; proxy/cloud use is not proof of abuse |

Named-service sources: [Grok overview](https://docs.x.ai/grok-bot/overview),
[Grok security](https://docs.x.ai/grok-bot/security),
[Grok settings](https://docs.x.ai/grok-bot/settings-and-notifications),
[Muse](https://ai.meta.com/muse/),
[Muse capabilities](https://ai.meta.com/blog/introducing-muse-spark-meta-model-api/),
[Pickle delegation](https://www.pickle.com/help/tasks/delegate),
[Pickle login](https://www.pickle.com/help/connect/save-login),
[Pickle terms](https://www.pickle.com/terms-of-service),
[OpenAI computer use](https://developers.openai.com/api/docs/guides/tools-computer-use),
[Anthropic computer use](https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool),
[Gemini computer use](https://ai.google.dev/gemini-api/docs/computer-use),
[Microsoft computer use](https://learn.microsoft.com/en-us/microsoft-copilot-studio/computer-use),
[execution host](https://learn.microsoft.com/en-us/microsoft-copilot-studio/configure-where-computer-use-runs),
[browser proxy infrastructure](https://docs.browserbase.com/platform/identity/proxies).

### Crawlers, fetchers and preferences

Training crawlers, search crawlers, user-triggered fetchers and hosted interactive
agents are distinct. OpenAI documents GPTBot/OAI-SearchBot/ChatGPT-User separately;
Anthropic documents ClaudeBot/Claude-SearchBot/Claude-User separately from hosted
Claude-Agent. Google distinguishes common/special/user-triggered fetchers and
Google-Extended content-use preferences. Perplexity documents distinct bot/user
fetcher policies and feeds. Verify current identity through official mechanisms;
UA strings alone are forgeable and commercial verified status is not local
authorization.

Anthropic hosted-browser documentation describes Claude-Agent plus published
SAIL IP information; SAIL draft-kelley-webbotauth-sail-00 is not an RFC. Pin and
recheck draft versions before implementation. Meta's current crawler document
returned 429 during research: its current tokens/policy remain pending official
reverification, not inferred from Muse or an old third-party list.

robots.txt RFC 9309 is preference publication, not access authentication. Record
served policy digest, authority/scheme/port, served/applied time, parser/cache/
redirect/error state, actor provenance and resource-match result. Separate an
observed policy conflict from verified operator attribution. Absence of a recent
robots fetch does not prove noncompliance: cache/worker/proxy history may differ.
Protect restricted content with origin authorization independent of preferences.

Cloudflare's 2025-08-04 Perplexity report is an external operator's published
experiment, not a humanymous reproduction or a property of every current request.
Do not generalize that report to Grok, Muse, Pickle or all user-directed fetches.

Sources: [OpenAI bots](https://developers.openai.com/api/docs/bots),
[Anthropic crawlers](https://privacy.claude.com/en/articles/8896518-does-anthropic-crawl-data-from-the-web-and-how-can-site-owners-block-the-crawler),
[hosted-browser identity](https://support.claude.com/en/articles/18234133-how-do-anthropic-hosted-browsers-identify-themselves-when-they-browse-the-web-on-a-user-s-behalf),
[Google crawlers](https://developers.google.com/crawling/docs/crawlers-fetchers/google-common-crawlers),
[Google verification](https://developers.google.com/crawling/docs/crawlers-fetchers/verify-google-requests),
[Perplexity policies](https://docs.perplexity.ai/docs/resources/perplexity-crawlers),
[RFC 9309](https://www.rfc-editor.org/rfc/rfc9309.html),
[external Cloudflare report](https://blog.cloudflare.com/perplexity-is-using-stealth-undeclared-crawlers-to-evade-website-no-crawl-directives/).

### Network-source classification

Keep actual transport peer separate from client-asserted forwarding fields and
authenticated proxy provenance. IP/ASN/feed data records source URL, retrieval
and publication times, digest/version, IPv4/IPv6, expiry, rollback and failure.
Hosting/ASN classification is not operator identity, human absence or malicious
intent. Official ranges may omit BYOIP/services; a hosting block can affect
legitimate dependent services. Missing/stale feed behavior is explicit per-route
capability policy, never a hidden universal allow/deny or scoring change.

Include cloud VDI, shared NAT, corporate proxies, VPN/privacy browsers, monitoring,
CI/webhooks and accessibility users as controls. Bind budgets to authenticated
account/key/tenant/workflow where appropriate; an IP change is not a new right.
Sources: [AWS range limitations](https://docs.aws.amazon.com/vpc/latest/userguide/aws-ip-ranges.html),
[change notifications](https://docs.aws.amazon.com/vpc/latest/userguide/subscribe-notifications.html).

### Proposed causal event contract

Research direction: server-verified connection → request → principal → workflow
transition → budget reservation → integrity validation → audit commitment →
origin effect → result. This is a project proposal, not a new international
standard or current implemented subsystem. Reuse the adapter envelope/trust
contract in [[sot:root/adapter-ecosystem]] rather than a parallel tracking engine.

Events carry version/producer/tenant/node/event ID/sequence; connection/request/
stream and parent/decision references; server receipt time/monotonic duration/
clock quality; evidence provenance; route template/method/upstream; body byte
layer/completeness/digest; credential validation/scope; resource reservation/
settlement/refund; truncation/drop/overflow; verdict versus ingestion/dependency
error; origin-effect boundary and audit reference.

Provenance distinguishes direct server observation, authenticated forwarded
fact, application/browser assertion, derived result and not-observed. Never
adopt an external trace/request ID as the internal authority ID, elevate a
browser-reported server/solved/isTrusted flag, or misattribute H2 connection
facts across stream/tenant boundaries. Replay/dedup is idempotent for budgets,
audit and effects; decisions remain route/body/session/policy/freshness bound.
Complete-body integrity finishes before origin effects. Infinite streams have
a separately specified start/stream contract. A receipt proves the specified
continuity, not human presence or benign intent.

DOM isTrusted distinguishes UA dispatch from dispatchEvent, not natural persons.
WebDriver's 2026-07-02 Working Draft describes trusted automation input. Preserve
the coherent-browser T4 ceiling; do not gate Pass on motor richness or speed.
Authenticated API clients receive a legitimate machine-client path instead of
mandatory browser events. WebAuthn L3 (2026-08-25 Recommendation) authenticates a
credential ceremony, not all later actions or their benevolence.

Sources: [DOM](https://dom.spec.whatwg.org/#dom-event-istrusted),
[WebDriver draft](https://www.w3.org/TR/2026/WD-webdriver2-20260702/),
[WebAuthn L3](https://www.w3.org/TR/webauthn-3/).

### Authentication and standards maturity

HTTP Message Signatures RFC 9421 and digests RFC 9530 provide building blocks;
the profile must specify covered method/target/body and recompute actual bytes.
DPoP RFC 9449 and mTLS RFC 8705 bind token/key or certificate, not universal
payload integrity or authorization. Keep them separate from frozen RIT.
Web Bot Auth revision 01 (2026-10-29) is an active Internet-Draft, not RFC; its
dictionary Signature-Agent form and authority-only minimal coverage require
version-aware compatibility and independent request binding/replay design.
Verified keys/operators still need explicit scope, quota and origin permission.

Key discovery is bounded against SSRF, redirect/DNS changes, local/private/link-
local targets, oversized documents, churn, time/concurrency and negative-cache
abuse. Local management is never a discovery target or public adapter route.
Cloudflare's current implementation documents missing nonce/replay-DB validation;
do not import commercial verification as proof of complete replay prevention.

Sources: [signatures](https://www.rfc-editor.org/rfc/rfc9421.html),
[digests](https://www.rfc-editor.org/rfc/rfc9530.html),
[DPoP limits](https://www.rfc-editor.org/rfc/rfc9449.html#section-11.7),
[mTLS](https://www.rfc-editor.org/rfc/rfc8705.html),
[Web Bot Auth revision 01](https://datatracker.ietf.org/doc/html/draft-ietf-webbotauth-httpsig-protocol-01),
[Cloudflare implementation](https://developers.cloudflare.com/bots/reference/bot-verification/web-bot-auth/),
[commercial verification](https://developers.cloudflare.com/bots/concepts/bot/verified-bots/).

### Scale, privacy and measurement

W3C Trace Context is correlation, not authentication. External sampled flags
cannot suppress security audit. Separate durable enforcement audit from sampled
analytics export. Bound queue/memory/disk/batch/event/attribute/cardinality costs;
count attempts/ALLOW/CHALLENGE/DENY/error/skip independently without sampling.
Expose loss/overflow/degraded intervals and incomplete dimensions. Never label
an export queue/WAL unconditionally lossless under disk/queue/retry failure.
Metrics use bounded route/reason/profile dimensions, not raw IP/session/URL labels.

Exclude raw keystrokes/passwords/forms/clipboard/full DOM/screenshots by default.
Minimize browser aggregates and lifetime; low event count/assistive technology
does not imply abuse. Hashing small-domain identifiers is not anonymity. Declare
tenant/purpose pseudonyms, retention/key rotation/access/erasure, preserving
per-subject cryptographic erasure and dual control. Public Draft v0 uses neutral
abstract capabilities and synthetic examples, not private traces or vendor blame.

Sources: [Trace Context security](https://www.w3.org/TR/trace-context/#security-considerations),
[OTel cardinality](https://opentelemetry.io/docs/specs/otel/metrics/sdk/#cardinality-limits),
[export resilience](https://opentelemetry.io/docs/collector/resiliency/),
[sensitive data](https://opentelemetry.io/docs/security/handling-sensitive-data/),
[Privacy Principles, W3C Statement](https://www.w3.org/TR/privacy-principles/).

## Why

The user requested broad named-agent/crawler/datacenter/API research and stable,
systematic tracking. Three official-source subagent investigations distinguish
operating models from verified site behavior and separate cryptographic identity,
human presence, preferences, authorization and effects. This supports a testable
interoperability proposal without claiming universal Computer Use detection.

## Verification

Expand only the isolated owned lab: real-browser/screenshot/DOM/API/hybrid agents;
cloud/desktop/residential/trusted-terminator topology; signed/forged/revoked/replay
identities; ordinary authenticated API/webhook/CI users; human keyboard/screen-
reader/mobile/VDI controls; stale feeds; robots cache/redirect/errors; cross-
tenant/body/stream receipts; discovery SSRF; cardinality/disk/queue loss; and
streaming/idempotent origin effects. Record all outcomes and full denominators.
No named vendor blocking performance or runtime implementation was measured.

## Accessibility desktop tools research

Vercel agent-desktop v1.2.0 mechanisms, observation limits, AX-01 through AX-07 and owned-lab qualification cases are canonical in sot:root/accessibility-agent-tools. Accessibility semantic actions, physical synthesis, CDP and direct API phases have distinct provenance. Vendor attribution and blocking performance remain unmeasured; scoring and a11y constraints are unchanged.

## Mobile device automation research

Android Monkey, UI Automator, Appium, accessibility/input paths, Apple XCTest/WDA,
Maestro and AI/MCP orchestration are canonical in sot:root/mobile-agent-tools.
MOB-01 through MOB-09 distinguish hardware/app/request integrity from human
operation, preserve AT controls and require independent origin-effect evidence.
All mobile pilots remain not_run; no native attestation adapter or scoring change
is shipped. Real-device farms do not prove carrier egress or unautomated use.

