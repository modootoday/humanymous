---
id: evidence:root/20261107164800
title: Actual Windows controlled watchdog subprocess boundary
status: active
created: 2026-11-07T16:48:00Z
updated: 2026-11-07T16:50:36Z
rel:
  depends_on: [sot:root/cli-installation]
files:
  - {path: summary.json, sha256: f3caba7888c840299ab58124dd8236cd30b59d97953bf30ffc0d37fded19dfc6, bytes: 3247, store: repo}
---

One user-authorized actual Windows x86_64-pc-windows-msvc parent invocation passed.
The exact Rust harness is1passed/0failed/0ignored/0measured/0filtered, verified
independently from the outer exit boolean. Four distinct ordered JSON observations
are retained in summary.json: positive exit0, stdout-pressure exit74,
stderr-pressure exit74 and delayed completion exit74. Every case observed its
controlled writer-entry prefix and released3parent endpoints after actual child
exit. No forced-stop recovery may count as a subcase pass.

CLI immutable sourcec57cf0c6a63d9a99f0596e14e5328b8bf6a97c0b contains the
fb6f26ed shared-barrier consumer. Source archive SHA256 is
d1b228c1945329122923475b30fefc48119e5ddc28d2f48ce95d1d0eb711dc05.
The same shared controller was extended under source
2400432116a0ddd5b9d29ced92ab46954c2f24d2; Native's distinct five-provider-case
receiptccd83927 preceded this run and is not retroactively relabeled as this
profile. The new profile had owner-reported pure/check/strict-Clippy/fmt/build
success before handoff. CLI no-run build, all-target strict Clippy and package fmt
passed; raw build JSON binds actual binaries and features.

| Executed artifact | SHA256 |
|---|---|
| Exact watchdog parent | a22fb7d528ab1b21a6977bf51cd0e1fd811326906ab630f4ab64a9c0e8eea8aa |
| Actual compile-time service-fixture path | f13ea8cda9eec7d188478095399f1dec47a520ab9136a7b9e619d09fcab17d1e |
| Outer controller | 3ff6cb8e35454571a682e4a2d224d62e4bd3a4852dbc398d81d07ff2c7d474c0 |

Parent and fixture were independently hash-pinned before and after, with an exact
artifact-write lease through invocation; a private parent copy alone would not
have frozen the fixture at its baked absolute path. Before/after artifact manifest
hash is5fbd9bc15043159f1c626210cc2da83751a21ebb19af8550d60cbadb07db208c.

The native Job was assigned before the shared cooperative PID/selector/nonce
grant. Parent run/cleanup thresholds8/12seconds began before its barrier; outer
run/cleanup thresholds10/15seconds began before process creation. The parent
naturally exited0 and outer controller returned0 after2018ms. Actual parent exit
and Job population0 were observed within original cleanup; cleanup termination
had no error and outer_cleanup_unknown=false. Job cleanup requested termination
after natural parent completion; it did not manufacture the four inner exits.
Actual admission was35.55GiB free,63.80percent committed and126832685056disk bytes
free. No duplicate or automatic rerun occurred.

This is a fixed trusted cooperative component. Marker entry does not prove a
specific kernel write blocked, and std endpoint release is not EOF/native-close/
reader-join proof. The direct-child API's tree_cleanup_proven=false remains distinct
from the outer controller's actual assigned-Job empty observation. Simulated
runtime completion is explicit; real audit flush/drain, independent producer,
SCM/account/privilege, hosted cross-OS, arbitrary untrusted launch and universal
syscall wall-clock claims remain unqualified. No product qualification follows.

Private bundle20261107164000 retains archive, binaries, original recipe, build and
lint logs, admission, before/after hashes, grant and complete parent/controller
logs with hashes. Public summary omits raw native PIDs, paths and nonce. All owned
handles, compiler/runtime targets, artifact-write leases and Git mutex ended.
