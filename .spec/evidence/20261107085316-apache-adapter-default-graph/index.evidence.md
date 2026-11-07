---
id: evidence:root/20261107085316
title: Apache default graph isolation and safe unit verification
status: active
created: 2026-11-07T08:53:16Z
updated: 2026-11-07T08:54:31Z
files:
  - {path: result.json, sha256: c52f051102ba6b6b4341e530de4cfb4cb8a937260cd496472abd66bd046df9c3, bytes: 1991, store: repo}
---

# Apache default graph isolation and safe unit verification

Root assigned D with isolated target/validation-native, jobs1/incremental0 and
Rust1.99.1. Safe tests passed9with0failed/ignored/filtered. Native package DEFAULT
all-target check and warnings-denied Clippy passed. Source/graph hashes captured
before and after all commands matched. Memory:33.14GiBfree/81%commit before,
33.09GiB/81%after. The slot was promptly returned.

The default package required no Apache headers, libclang or bindgen. This does
not compile the selected httpd bridge, run the native module or exercise a host
process/IPC/network/Docker pilot. All45hosted scenarios remain NOT_RUN.
