# Crate architecture research provenance

Official sources checked 2026-11-07 KST. Three read-only agents:
- hosted_runner_research: Cargo layout/naming, package/bin identifiers,
  metadata/MSRV/registry and normal/dev dependency publication requirements.
- dedicated_runner_research: actual context/DAG/source-inventory audit,
  application/secret/provenance gaps and native-versus-portable boundaries.
- crossos_matrix_research: public SDK/runtime/WASM/ABI/release taxonomy and
  optional substantive core consolidation with migration acceptance gates.

Canonical synthesis and primary URLs: sot:root/crate-architecture.
package-graph.json is a static Cargo metadata snapshot, not runtime parity,
publication readiness or registry ownership proof. Private full metadata and
separate integrated key/vault implementation tests are retained in root evidence.
All research agents made no code/Git changes. No crate rename, merge, registry
reservation, publish=false override or release-gate weakening was performed.
