---
id: evidence:root/20261106235520
title: Embedded browser analysis
status: active
created: 2026-11-06T23:55:19Z
updated: 2026-11-06T23:55:19Z
---

# Embedded browser analysis

## What was measured

Read-only installed-artifact research, 2026-11-07 KST. No target execution,
restart, debug enablement, live attach, account/profile/database/memory reads or
UI mutation. The main ASAR SHA-256 was independently rechecked in this chat.
Raw artifacts and extracted source are private, never attachments to this index.

### Identity and coordinates

E1: resources/app.asar in OpenAI.Codex_26.1029.7803.0_x64__2p2nqsd0c76g0/app:
720,736,234 bytes; SHA-256
`77ed4ddc5b388ac8b29f5035f9c98f982b095df55aad6fa6c27bb80d763b9691`.
Package JavaScript metadata 26.1029.61003 is a separate version field from MSIX.

| Member alias | ASAR member | Absolute ASAR byte start | Member bytes | Member SHA-256 |
|---|---|---:|---:|---|
| M | .vite/build/early-bootstrap-632-62c641245a52ef0e.js | 8319993 | 5373536 | c16675e64dae86bdb09f7720f038a8c2f2beacf0e85bbadd369f407d38662aa4 |
| P | .vite/build/browser-page-preload.js | 5230716 | 557741 | 2d4218e02e1f2f06cb8470b41b5dd2dae4954c2636f28561bbed7502f0914953 |
| B | .vite/build/early-bootstrap.js | 14066838 | 596158 | c58375117c12c1fa4629490f4184d0982d5ef0645625e79b68fa3a7399fbf434 |

All ranges below are **zero-based Unicode codepoint, half-open** ranges in the
UTF-8 decoded member, calculated by the private Python extraction scripts. They
are not ASAR offsets, byte offsets or JavaScript UTF-16 indices. ASAR byte starts
above are separately calculated from its header/data layout. Hash the member
before applying a range. Private manifests retain ranges, exact slice hashes,
transformations and source snippets for audit without publishing original code.

| ID | Member and range | Original observation summary |
|---|---|---|
| E2 | M [2670866,2685345) | Owner will/did-attach-webview callbacks, admission/route/storage/generation checks, pending instance/adoption consumption, fixed browser session and preload, policy sender-frame check. |
| E3 | M [2147382,2147718) | Guest preferences: sandbox/context isolation/web security on; Node/subframe/worker integration, insecure content, nested webviews/plugins off. |
| E4 | M [2492662,2493367) | Custom adoption API feature check and lease creation, owner binding, pending-map tracking and destruction cleanup. |
| E5 | M [2251855,2252705) | WebContentsView is constructed for downloadURL, with timeout and close cleanup. |
| E6 | M [219661,219731), [2148697,2153197) | Persistent partition helper, app browser session configurator, permissions/download/navigation hooks. |
| E7 | M [2467481,2468425), [2470646,2474072) | Resolve session route; reject absent/dead window; create session backend with auth supplier/current-state checks. |
| E8 | M [2436818,2438320) | Native-pipe creation with peer authorizer, bounded frames, CDP/detach/page/download event forwarding and disposal. |
| E9 | M [284767,285986), [2432900,2436400) | Debugger attachment ownership/child sessions, command dispatch to resolved guest target/session and adjacent supported-method restrictions. |
| E10 | P [541574,541694), [547818,548718), [552778,553478) | Host IPC invocation, annotation policy query, main-world context bridge. |
| E11 | M [30567,34067), B [287386,287456) | Main chunk imports module 4482 as eo; bootstrap defines module 4482 as Electron. |
| E12 | M [2530319,2533819), [2588051,2591551) | window.open child guest adoption and registered opener placeTab/completeAdoption coordination. |

### REA evidence and provenance

Five derived slices (attach, debugger, preferences, downloads, adoption) were
analyzed as an inert local directory using REA analyze_javascript_application.
The attach function has only a parenthesized-expression wrapper added. Other
slices preserve the selected text. Dependencies and lexical aliases are omitted;
zero Electron-class findings in this slice analysis do not negate manual
bootstrap alias resolution or imply absence of Electron in the installation.

- Complete derived-directory evidence:
  `ev_9bc80d5d19c1ac261a679c29fbdfc1088ec0871d6feca5866df359aa48a26502`.
- Summary projection:
  `ev_54484d788dfa680ec89d123fa75a9905160de0e8e78b0ea748a256552b8c806c`.
- Derived directory digest:
  `9f4d69cb489f4718be99af00875352c086ad1188f3d0ae2ad71b0f3fba379708`.
- Five files / 17,592 analyzed bytes / 4,321 AST nodes / zero parse failures;
  application coverage partial, semantic coverage unknown, 941 semantic unknowns.
- Per-slice SHA-256, in E2/E9/E3/E5/E4 order:
  `fda5ce6239d50ee70b397735474cbddb646ba80732b3c295cea4739d9bd0c013`,
  `9a0637a09fc287a988e843812cbd6a01e0adefae391e4fae5e7888a9a274613d`,
  `8bd93084a94832fdd7a8c1fc18c8b6e8160fd9c7bf773b43f1123248297ef07c`,
  `ef54cbb99d9d4c40294976842288dc1c570783c86fafe83368ded01dd23b7cbd`,
  `1277aef57562cf423af1fc6e7a9215faa75b148f1898eb689664d85581a9536d`.

REA labels the analyzed directory shipped-artifact/derived, but provenance to
the original package also depends on the analyst extraction/hash/range record.
The directory evidence ID is not the original ASAR evidence ID. E6–E8 and
E10–E12 are manual static observations with source hashes/ranges, not claims that
REA recovered complete cross-module relationships for them.

Private record location: .spec.local/root/analysis/embedded-browser/ contains
extract.py, manifest.json, slices.py, slice-manifest.json, observations.py,
observations.json, windows.json, selected members, slices/ and rea-session.json.
The REA bundle is 4,688,530 bytes with two records. No full prior bundle was imported.

H1 (inherited historical evidence): initial installation/package inspection
`ev_9f1e472964fce6a08a27d0b486f72a96e0d352b9567294811ada0beb262d874d`;
partial selected-source parent
`ev_82be17046bb08532fa206c52c10e3ef080a2281c88e5611fd1e01c9ebb107b77`;
summary `ev_f4b894ce2677d20e30e46ce1b00d57c6e787f637dc8b3cdd0a33dbe61577664b`.
Private original location is .spec.local/root/analysis/owl/. Main chunk analysis
timed out at 30 seconds, retaining 100,000 semantic unknowns. Eight unpacked
native companions had declared/observed integrity contradictions under
record-and-continue; no native trust or malware conclusion follows. Missing fuse
sentinel is unknown, and canceled native overview produced no evidence.

### External and inherited sources

| ID | Source | Scope and date |
|---|---|---|
| O1 | [Official Browser documentation](https://learn.chatgpt.com/docs/browser) | Retrieved 2026-11-07 KST. Separate profile/tabs/history, full/split view, controlled developer CDP. Does not name embedding library. |
| T1 | [How Codex embeds its browser](https://csarko.sh/research/how-codex-embeds-its-browser) | 2026-10-24 third-party inference; guest/local socket claims are historical context. |
| T2 | [Browser Use Deep Dive](https://www.ifuryst.com/en/blog/2026/open-browser-use/) | 2026-05-09 author analysis of client 0.1.0-alpha2, including windowId/sessionId abstraction. |
| T3a | [Issue 30178](https://github.com/openai/codex/issues/30178) | User report of navigation crash; upstream report, not our reproduction. |
| T3b | [Issue 33514](https://github.com/openai/codex/issues/33514) | Windows attach crash report; no evidence of current runtime defect here. |
| T4 | [Reddit discussion supplied by parent](https://www.reddit.com/r/codex/comments/1wnqcbm/does_anyone_know_what_codex_uses_for_its_inapp/) | Not independently fetched and not relied on; question/answer speculation has no implementation authority. |

R1: Parent-chat supplied read-only httpbin user-agent/headers echo: Windows 10
x64, Chrome/156.0.0.0 Safari/537.36; client hints Chromium/Google Chrome 156,
Not:A-Brand 8, Windows, mobile false; ko language. That response had no
Electron/ChatGPT token, Cookie or Authorization header. This chat did not access
the parent tab or independently reproduce the result. It is not proof of an exact
patch, embedding library, absence of cookies on other requests, or human activity.
Parent navigator evaluation returned undefined; chrome://version was policy
blocked. No bypass or direct CDP attempt was made.

## How to read it

See [analysis SoT](../../sot/embedded-browser-analysis.sot.md) for the reconstructed
dataflow, original architectural principles and unverified frontier. Static
presence proves code exists in this artifact, not that a running tab executed it.
An omitted dependency or timeout is an unknown, never negative proof. This public
index contains analyst summaries and identities only, not recovered implementation.
