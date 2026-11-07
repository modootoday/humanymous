---
id: change:root/20261107162913
title: nginx adapter naming migration handoff
status: active
created: 2026-11-07T16:29:13Z
updated: 2026-11-07T16:35:45Z
rel:
  depends_on: [sot:root/rust-migration, sot:root/nginx-adapter]
---

# nginx adapter naming migration handoff

## Change

Document the nginx impact inventory for the root-accepted target package
humanymous-adapter-nginx in the existing crate INTEGRATION handoff. Current
humanymous-nginx-adapter remains unchanged. The inventory distinguishes Cargo
selection, default Rust library/output names, explicit binary names, installed
nginx loader path, native exports/directive, strict unsigned package identity,
protocol/schema IDs, CI/lab consumers and path/claim associations.

The later user clarification supersedes retaining an implementation-specific
public loader: public identity is humanymous, while adapter-family package names
are internal. Root must map the public facade and host/ABI-isolated loader paths
atomically. Existing paths are legacy inventory, not target public names.
Shared providers and historical
source-bound evidence are excluded from mechanical renaming. No code, manifest,
lockfile, native artifact or docs/reviews file changed in this slice.

## Migration

Root owns one graph/source/lock/workflow/artifact/docs/claim migration under
sot:root/structural-development. After its accepted exact mapping, run fmt,
selected locked pure tests/Clippy and actual source-bound nginx/authority pilot
with explicit compiler-output versus installed-loader-path checks. Preserve
old receipts and raw hashes. No compilation or runtime was launched for this
documentation-only handoff, and exact8132117 qualification remains historical.
